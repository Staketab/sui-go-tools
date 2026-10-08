//! Walrus storage node commission. Walrus runs on Sui: commission accrues inside the node's
//! staking pool and `staking::collect_commission` pays it out as a `Coin<WAL>` to the pool's
//! commission receiver: an address that sends the transaction, or an object it presents,
//! in practice the node's `StorageNodeCap`. The pool's governance-authorized party is a
//! separate role and cannot collect unless it is also the commission receiver.

use anyhow::{Context as _, Result, anyhow};
use serde_json::Value;
use sui_sdk_types::{Address, Digest, StructTag, TypeTag};
use sui_transaction_builder::{ObjectInput, TransactionBuilder};

use crate::Context;
use crate::amount;
use crate::cli::{Target, TxArgs, WalrusCommand};
use crate::keys::{self, Actor, Chain};
use crate::sui::{self, Plan, SuiNode};
use crate::ui;

/// Bag key under which a pool keeps the commission added at the start of an epoch; it can be
/// collected once the epoch's voting ends.
const BLOCKED_COMMISSION_KEY: &str = "::staking_pool::NewEpochCommissionBlockedForCollection";

struct Staking {
    id: Address,
    initial_shared_version: u64,
    /// The current walrus package. The staking object refuses calls through older package
    /// versions, so it is read from the object instead of being configured.
    package: Address,
    /// The package that defined the Walrus types; type names keep it across upgrades.
    types_package: Address,
    /// `ObjectTable<ID, StakingPool>` with one pool per storage node.
    pools: Address,
}

/// `walrus::auth::Authorized`: who may collect commission or vote for a pool.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Authorized {
    Address(Address),
    Object(Address),
    Unknown,
}

impl Authorized {
    fn parse(json: &Value) -> Self {
        let id = json["pos0"]
            .as_str()
            .and_then(|s| keys::parse_address(s).ok());
        match (json["@variant"].as_str(), id) {
            (Some("Address"), Some(id)) => Authorized::Address(id),
            (Some("ObjectID"), Some(id)) => Authorized::Object(id),
            _ => Authorized::Unknown,
        }
    }

    /// How `owner`, holding `caps`, can act as this party, if it can.
    fn auth(&self, owner: Address, caps: &[NodeCap]) -> Option<Auth> {
        match *self {
            Authorized::Address(address) if address == owner => Some(Auth::Sender),
            Authorized::Object(id) => caps.iter().find(|c| c.id == id).cloned().map(Auth::Cap),
            _ => None,
        }
    }
}

impl std::fmt::Display for Authorized {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Authorized::Address(address) => write!(f, "{address}"),
            Authorized::Object(id) => write!(f, "the holder of object {id}"),
            Authorized::Unknown => f.write_str("an unknown party"),
        }
    }
}

struct Pool {
    /// The pool's ID is the storage node ID.
    id: Address,
    name: String,
    state: String,
    commission: u64,
    commission_rate_bps: u64,
    receiver: Authorized,
    governance: Authorized,
    /// The pool's `extra_fields` bag, when it has entries.
    extra_fields: Option<Address>,
}

impl Pool {
    fn parse(json: &Value) -> Result<Self> {
        let id = json["id"]
            .as_str()
            .map(keys::parse_address)
            .context("staking pool without id")??;
        let extra_fields = match sui::json_u64(&json["extra_fields"]["size"]) {
            Some(size) if size > 0 => json["extra_fields"]["id"]
                .as_str()
                .and_then(|s| keys::parse_address(s).ok()),
            _ => None,
        };
        Ok(Pool {
            id,
            name: json["node_info"]["name"]
                .as_str()
                .unwrap_or("unnamed")
                .to_owned(),
            state: json["state"]["@variant"]
                .as_str()
                .unwrap_or("unknown")
                .to_owned(),
            commission: sui::json_u64(&json["commission"])
                .with_context(|| format!("pool {id} has no commission field"))?,
            commission_rate_bps: sui::json_u64(&json["commission_rate"]).unwrap_or(0),
            receiver: Authorized::parse(&json["commission_receiver"]),
            governance: Authorized::parse(&json["governance_authorized"]),
            extra_fields,
        })
    }

    fn matches(&self, filter: &str) -> bool {
        self.name.eq_ignore_ascii_case(filter) || keys::parse_address(filter).ok() == Some(self.id)
    }
}

#[derive(Clone)]
struct NodeCap {
    id: Address,
    version: u64,
    digest: Digest,
    node_id: Address,
}

/// How an account proves it is the commission receiver.
#[derive(Clone)]
enum Auth {
    Sender,
    Cap(NodeCap),
}

/// A node an account receives commission from, governs, or whose cap it holds.
struct Holding<'p> {
    pool: &'p Pool,
    /// How the account collects the commission; `None` when it goes to someone else.
    auth: Option<Auth>,
    governs: bool,
    holds_cap: bool,
    blocked: u64,
}

impl Holding<'_> {
    fn ready(&self) -> u64 {
        self.pool.commission.saturating_sub(self.blocked)
    }
}

pub async fn run(ctx: &Context, command: WalrusCommand) -> Result<()> {
    let mut node = SuiNode::connect(&ctx.config.sui.grpc_url)?;
    match command {
        WalrusCommand::Status(target) => status(ctx, &mut node, &target).await,
        WalrusCommand::Collect { nodes, to, tx } => {
            collect(ctx, &mut node, &nodes, to.as_deref(), &tx).await
        }
        WalrusCommand::Send {
            to,
            amount,
            all,
            tx,
        } => {
            sui::send_coin(
                ctx,
                &mut node,
                Chain::Walrus,
                &ctx.config.walrus.coin_type,
                "WAL",
                &to,
                amount.as_deref(),
                all,
                &tx,
            )
            .await
        }
    }
}

fn wal_type(ctx: &Context) -> Result<StructTag> {
    ctx.config.walrus.coin_type.parse().map_err(|_| {
        anyhow!(
            "invalid [walrus].coin_type {:?}",
            ctx.config.walrus.coin_type
        )
    })
}

async fn load_staking(ctx: &Context, node: &mut SuiNode) -> Result<Staking> {
    let id = keys::parse_address(&ctx.config.walrus.staking_object_id)?;
    let object = node.object(id).await?;
    let json = sui::json(&object);
    let package = json["package_id"]
        .as_str()
        .map(keys::parse_address)
        .context("Walrus staking object has no package_id")??;
    let version =
        sui::json_u64(&json["version"]).context("Walrus staking object has no version")?;
    let types_package = *object
        .object_type()
        .parse::<StructTag>()
        .map_err(|_| anyhow!("unexpected type of the Walrus staking object"))?
        .address();
    // The staking state lives in a dynamic field keyed by the version number.
    let inner_id = id.derive_dynamic_child_id(&TypeTag::U64, &bcs::to_bytes(&version)?);
    let inner = sui::json(&node.object(inner_id).await?);
    let pools = inner["value"]["pools"]["id"]
        .as_str()
        .map(keys::parse_address)
        .context("Walrus staking state has no pools table")??;
    Ok(Staking {
        id,
        initial_shared_version: object.owner().version(),
        package,
        types_package,
        pools,
    })
}

/// Every storage node's staking pool: about 140 on mainnet, read in one paged request.
async fn load_pools(node: &mut SuiNode, staking: &Staking) -> Result<Vec<Pool>> {
    let fields = node
        .dynamic_fields(staking.pools, &["child_id", "child_object.json"])
        .await?;
    fields
        .iter()
        .map(|field| {
            let pool = field
                .child_object
                .as_ref()
                .context("the node returned a staking pool without contents")?;
            Pool::parse(&sui::json(pool))
        })
        .collect()
}

async fn node_caps(node: &mut SuiNode, staking: &Staking, owner: Address) -> Result<Vec<NodeCap>> {
    let cap_type = format!("{}::storage_node::StorageNodeCap", staking.types_package);
    let objects = node.owned_objects(owner, &cap_type).await?;
    objects
        .iter()
        .map(|object| {
            let node_id = sui::json(object)["node_id"]
                .as_str()
                .map(keys::parse_address)
                .context("StorageNodeCap without node_id")??;
            Ok(NodeCap {
                id: keys::parse_address(object.object_id())?,
                version: object.version(),
                digest: object
                    .digest()
                    .parse()
                    .map_err(|_| anyhow!("invalid digest of cap {}", object.object_id()))?,
                node_id,
            })
        })
        .collect()
}

/// Commission added at the start of the epoch, which `collect_commission` leaves in the pool
/// until the epoch's voting ends.
async fn blocked_commission(node: &mut SuiNode, pool: &Pool) -> Result<u64> {
    let Some(bag) = pool.extra_fields else {
        return Ok(0);
    };
    for field in node.dynamic_fields(bag, &["name", "value"]).await? {
        let is_blocked = field
            .name
            .as_ref()
            .and_then(|name| name.name.as_deref())
            .is_some_and(|key_type| key_type.ends_with(BLOCKED_COMMISSION_KEY));
        if is_blocked {
            let bytes = field
                .value
                .as_ref()
                .and_then(|value| value.value.as_deref())
                .context("blocked commission without a value")?;
            return Ok(bcs::from_bytes(bytes)?);
        }
    }
    Ok(0)
}

/// Nodes whose commission `actor` receives, plus nodes it governs or whose cap it holds.
async fn holdings<'p>(
    node: &mut SuiNode,
    staking: &Staking,
    pools: &'p [Pool],
    actor: &Actor<'_>,
) -> Result<Vec<Holding<'p>>> {
    let caps = node_caps(node, staking, actor.address).await?;
    let mut holdings = Vec::new();
    for pool in pools {
        let auth = pool.receiver.auth(actor.address, &caps);
        let governs = pool.governance.auth(actor.address, &caps).is_some();
        let holds_cap = caps.iter().any(|c| c.node_id == pool.id);
        if auth.is_none() && !governs && !holds_cap {
            continue;
        }
        let blocked = match auth {
            Some(_) => blocked_commission(node, pool).await?,
            None => 0,
        };
        holdings.push(Holding {
            pool,
            auth,
            governs,
            holds_cap,
            blocked,
        });
    }
    Ok(holdings)
}

/// "governance rights are here, but the commission goes to 0x…" for nodes an account
/// cannot collect from.
fn elsewhere(holding: &Holding<'_>) -> String {
    let here = match (holding.holds_cap, holding.governs) {
        (true, true) => "the StorageNodeCap and governance rights are here",
        (true, false) => "the StorageNodeCap is here",
        _ => "governance rights are here",
    };
    format!(
        "{here}, but the commission goes to {}",
        holding.pool.receiver
    )
}

/// A storage node whose commission one of the Walrus accounts receives.
pub struct NodeSummary {
    pub id: Address,
    pub name: String,
    pub holder: String,
    /// Commission that can be collected now.
    pub ready: u64,
    /// Commission that unlocks when the epoch's voting ends.
    pub later: u64,
}

pub async fn nodes(ctx: &Context, target: &Target) -> Result<Vec<NodeSummary>> {
    let mut node = SuiNode::connect(&ctx.config.sui.grpc_url)?;
    let staking = load_staking(ctx, &mut node).await?;
    let pools = load_pools(&mut node, &staking).await?;
    let mut summaries = Vec::new();
    for actor in ctx.actors(Chain::Walrus, target)? {
        for holding in holdings(&mut node, &staking, &pools, &actor).await? {
            if holding.auth.is_some() {
                summaries.push(NodeSummary {
                    id: holding.pool.id,
                    name: holding.pool.name.clone(),
                    holder: actor.label.clone(),
                    ready: holding.ready(),
                    later: holding.blocked,
                });
            }
        }
    }
    Ok(summaries)
}

async fn status(ctx: &Context, node: &mut SuiNode, target: &Target) -> Result<()> {
    let staking = load_staking(ctx, node).await?;
    let pools = load_pools(node, &staking).await?;
    let mut total = 0u128;
    let mut collectible_nodes = 0;
    for actor in ctx.actors(Chain::Walrus, target)? {
        ui::header(&format!("Walrus · {actor}"));
        let gas = node.balance(actor.address, sui::SUI_TYPE).await?.balance();
        let wal = node
            .balance(actor.address, &ctx.config.walrus.coin_type)
            .await?
            .balance();
        println!(
            "  wallet       {} WAL, {} SUI for gas",
            amount::format(wal as i128),
            amount::format(gas as i128)
        );
        let holdings = holdings(node, &staking, &pools, &actor).await?;
        if holdings.is_empty() {
            ui::warn("no storage node pays its commission to this address");
        }
        for holding in holdings {
            let pool = holding.pool;
            println!(
                "  {}  {}  {}",
                ui::bold(&pool.name),
                pool.state,
                ui::dim(&pool.id.to_string())
            );
            if holding.auth.is_none() {
                println!("    {}", elsewhere(&holding));
                continue;
            }
            println!(
                "    commission {} WAL ready to collect, rate {}%",
                ui::green(&amount::format(holding.ready() as i128)),
                pool.commission_rate_bps as f64 / 100.0
            );
            if holding.blocked > 0 {
                println!(
                    "    {}",
                    ui::dim(&format!(
                        "{} WAL more becomes collectable when this epoch's voting ends",
                        amount::format(holding.blocked as i128)
                    ))
                );
            }
            total += holding.ready() as u128;
            collectible_nodes += 1;
        }
    }
    if collectible_nodes > 1 {
        println!(
            "\n{} {} WAL across {collectible_nodes} nodes",
            ui::bold("total"),
            amount::format(total as i128)
        );
    }
    Ok(())
}

async fn collect(
    ctx: &Context,
    node: &mut SuiNode,
    filters: &[String],
    to: Option<&str>,
    args: &TxArgs,
) -> Result<()> {
    let to = to
        .or(ctx.config.walrus.recipient.as_deref())
        .map(keys::parse_address)
        .transpose()?;
    let wal = wal_type(ctx)?;
    let staking = load_staking(ctx, node).await?;
    let pools = load_pools(node, &staking).await?;
    let mut matched = vec![false; filters.len()];

    for actor in ctx.actors(Chain::Walrus, &args.target)? {
        ui::header(&format!("Walrus · {actor}"));
        let mut picked = Vec::new();
        for holding in holdings(node, &staking, &pools, &actor).await? {
            let pool = holding.pool;
            if !filters.is_empty() {
                let mut hit = false;
                for (i, filter) in filters.iter().enumerate() {
                    if pool.matches(filter) {
                        matched[i] = true;
                        hit = true;
                    }
                }
                if !hit {
                    continue;
                }
            }
            let Some(auth) = holding.auth.clone() else {
                ui::warn(&format!("{}: {}", pool.name, elsewhere(&holding)));
                continue;
            };
            let ready = holding.ready();
            let later = if holding.blocked > 0 {
                format!(
                    "; {} WAL more after this epoch's voting ends",
                    amount::format(holding.blocked as i128)
                )
            } else {
                String::new()
            };
            if ready == 0 {
                ui::info(&format!("{}: no commission to collect{later}", pool.name));
                continue;
            }
            ui::info(&format!(
                "{}: collecting {} WAL{later}",
                pool.name,
                amount::format(ready as i128)
            ));
            picked.push((pool.id, auth));
        }
        if picked.is_empty() {
            continue;
        }
        let recipient = to.unwrap_or(actor.address);
        let tx = collect_tx(&staking, &picked, recipient)?;
        let mut plan = Plan::new(&actor)
            .recipient(recipient)
            .coin(wal.clone(), "WAL")
            .package(staking.package)
            .shared_object(staking.id);
        for (node_id, _) in &picked {
            plan = plan.id(*node_id);
        }
        node.execute(&actor, tx, args, ctx.config.sui.gas_budget, &plan)
            .await?;
    }

    for (filter, found) in filters.iter().zip(matched) {
        if !found {
            ui::warn(&format!("no node {filter:?} among the Walrus accounts"));
        }
    }
    Ok(())
}

fn collect_tx(
    staking: &Staking,
    picked: &[(Address, Auth)],
    recipient: Address,
) -> Result<TransactionBuilder> {
    let cap_type: TypeTag = format!("{}::storage_node::StorageNodeCap", staking.types_package)
        .parse::<StructTag>()
        .map_err(|_| anyhow!("invalid StorageNodeCap type"))?
        .into();
    let mut tx = TransactionBuilder::new();
    let staking_arg = tx.object(ObjectInput::shared(
        staking.id,
        staking.initial_shared_version,
        true,
    ));
    let coins: Vec<_> = picked
        .iter()
        .map(|(node_id, auth)| {
            let auth = match auth {
                Auth::Sender => tx.move_call(
                    sui::function(staking.package, "auth", "authenticate_sender"),
                    vec![],
                ),
                Auth::Cap(cap) => {
                    let cap = tx.object(ObjectInput::owned(cap.id, cap.version, cap.digest));
                    tx.move_call(
                        sui::function(staking.package, "auth", "authenticate_with_object")
                            .with_type_args(vec![cap_type.clone()]),
                        vec![cap],
                    )
                }
            };
            let node_id = tx.pure(node_id);
            tx.move_call(
                sui::function(staking.package, "staking", "collect_commission"),
                vec![staking_arg, node_id, auth],
            )
        })
        .collect();
    let (coin, rest) = coins.split_first().context("no node to collect from")?;
    if !rest.is_empty() {
        tx.merge_coins(*coin, rest.to_vec());
    }
    let recipient = tx.pure(&recipient);
    tx.transfer_objects(vec![*coin], recipient);
    Ok(tx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_pools() {
        let pool = Pool::parse(&json!({
            "id": "0x248a6e1a20a83623179856ea69a38efcfd16cb7fdb5d5dba494adb98ac7f66bb",
            "node_info": { "name": "Staketab" },
            "state": { "@variant": "Active" },
            "commission": "16822622669135",
            "commission_rate": 6000,
            "commission_receiver": {
                "@variant": "Address",
                "pos0": "0x71352d5a6ce0d0fcba7b07402688602cef7ae90c79e940fcf223452f3b02f3db"
            },
            "governance_authorized": { "@variant": "ObjectID", "pos0": "0xab" },
            "extra_fields": {
                "id": "0x124fd80cd608dd88c45eaec84916cd8e15969c30fe24829c4abe48f151294765",
                "size": "1"
            }
        }))
        .unwrap();
        assert_eq!(pool.name, "Staketab");
        assert_eq!(pool.commission, 16_822_622_669_135);
        assert_eq!(pool.commission_rate_bps, 6000);
        let receiver: Address =
            "0x71352d5a6ce0d0fcba7b07402688602cef7ae90c79e940fcf223452f3b02f3db"
                .parse()
                .unwrap();
        assert_eq!(pool.receiver, Authorized::Address(receiver));
        assert_eq!(
            pool.governance,
            Authorized::Object(Address::from_static("0xab"))
        );
        assert!(pool.extra_fields.is_some());
        assert!(pool.matches("staketab"));
        assert!(pool.matches("0x248a6e1a20a83623179856ea69a38efcfd16cb7fdb5d5dba494adb98ac7f66bb"));
    }

    #[test]
    fn authenticates_receivers() {
        let owner = Address::from_static("0xa1");
        let cap = NodeCap {
            id: Address::from_static("0xc0"),
            version: 1,
            digest: Digest::ZERO,
            node_id: Address::from_static("0xd0"),
        };
        let caps = [cap];
        assert!(matches!(
            Authorized::Address(owner).auth(owner, &caps),
            Some(Auth::Sender)
        ));
        assert!(matches!(
            Authorized::Object(Address::from_static("0xc0")).auth(owner, &caps),
            Some(Auth::Cap(_))
        ));
        assert!(
            Authorized::Address(Address::from_static("0xb2"))
                .auth(owner, &caps)
                .is_none()
        );
        assert!(
            Authorized::Object(Address::from_static("0xc1"))
                .auth(owner, &caps)
                .is_none()
        );
        assert_eq!(Authorized::parse(&json!(null)), Authorized::Unknown);
    }
}
