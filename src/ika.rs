//! IKA validator commission. IKA runs on Sui: commission accrues inside the validator's
//! staking pool and `system::collect_commission` pays it out as a `Coin<IKA>` to whoever
//! presents the validator's `ValidatorCommissionCap`.

use anyhow::{Context as _, Result, anyhow, bail};
use sui_sdk_types::{Address, Digest, StructTag};
use sui_transaction_builder::{ObjectInput, TransactionBuilder};

use crate::Context;
use crate::amount;
use crate::cli::{IkaCommand, Target, TxArgs};
use crate::keys::{self, Actor, Chain};
use crate::sui::{self, Plan, SuiNode};
use crate::ui;

struct IkaSystem {
    id: Address,
    initial_shared_version: u64,
    /// The current `ika_system` package. The system object refuses calls through older
    /// package versions, so it is read from the object instead of being configured.
    package: Address,
}

struct CommissionCap {
    id: Address,
    version: u64,
    digest: Digest,
    validator_id: Address,
}

struct Validator {
    id: Address,
    name: String,
    state: String,
    commission: u64,
    commission_rate_bps: u64,
}

impl Validator {
    fn matches(&self, filter: &str) -> bool {
        self.name.eq_ignore_ascii_case(filter) || keys::parse_address(filter).ok() == Some(self.id)
    }
}

pub async fn run(ctx: &Context, command: IkaCommand) -> Result<()> {
    let mut node = SuiNode::connect(&ctx.config.sui.grpc_url)?;
    match command {
        IkaCommand::Status(target) => status(ctx, &mut node, &target).await,
        IkaCommand::Collect {
            validators,
            to,
            amount,
            tx,
        } => {
            collect(
                ctx,
                &mut node,
                &validators,
                to.as_deref(),
                amount.as_deref(),
                &tx,
            )
            .await
        }
        IkaCommand::Send {
            to,
            amount,
            all,
            tx,
        } => {
            sui::send_coin(
                ctx,
                &mut node,
                Chain::Ika,
                &ctx.config.ika.coin_type,
                "IKA",
                &to,
                amount.as_deref(),
                all,
                &tx,
            )
            .await
        }
    }
}

fn ika_type(ctx: &Context) -> Result<StructTag> {
    ctx.config
        .ika
        .coin_type
        .parse()
        .map_err(|_| anyhow!("invalid [ika].coin_type {:?}", ctx.config.ika.coin_type))
}

async fn load_system(ctx: &Context, node: &mut SuiNode) -> Result<IkaSystem> {
    let id = keys::parse_address(&ctx.config.ika.system_object_id)?;
    let object = node.object(id).await?;
    let package = sui::json(&object)["package_id"]
        .as_str()
        .map(keys::parse_address)
        .context("IKA system object has no package_id")??;
    Ok(IkaSystem {
        id,
        initial_shared_version: object.owner().version(),
        package,
    })
}

async fn commission_caps(
    ctx: &Context,
    node: &mut SuiNode,
    owner: Address,
) -> Result<Vec<CommissionCap>> {
    let cap_type = format!(
        "{}::validator_cap::ValidatorCommissionCap",
        ctx.config.ika.common_package_id
    );
    let objects = node.owned_objects(owner, &cap_type).await?;
    objects
        .iter()
        .map(|object| {
            let validator_id = sui::json(object)["validator_id"]
                .as_str()
                .map(keys::parse_address)
                .context("commission cap without validator_id")??;
            Ok(CommissionCap {
                id: keys::parse_address(object.object_id())?,
                version: object.version(),
                digest: object
                    .digest()
                    .parse()
                    .map_err(|_| anyhow!("invalid digest of cap {}", object.object_id()))?,
                validator_id,
            })
        })
        .collect()
}

async fn load_validator(node: &mut SuiNode, id: Address) -> Result<Validator> {
    let json = sui::json(&node.object(id).await?);
    Ok(Validator {
        id,
        name: json["validator_info"]["name"]
            .as_str()
            .unwrap_or("unnamed")
            .to_owned(),
        state: json["state"]["@variant"]
            .as_str()
            .unwrap_or("unknown")
            .to_owned(),
        commission: sui::json_u64(&json["commission"])
            .with_context(|| format!("validator {id} has no commission field"))?,
        commission_rate_bps: sui::json_u64(&json["commission_rate"]).unwrap_or(0),
    })
}

/// Validators whose commission caps `actor` holds.
async fn holdings(
    ctx: &Context,
    node: &mut SuiNode,
    actor: &Actor<'_>,
) -> Result<Vec<(CommissionCap, Validator)>> {
    let mut holdings = Vec::new();
    for cap in commission_caps(ctx, node, actor.address).await? {
        let validator = load_validator(node, cap.validator_id).await?;
        holdings.push((cap, validator));
    }
    Ok(holdings)
}

/// A validator whose commission cap one of the IKA accounts holds.
pub struct ValidatorSummary {
    pub id: Address,
    pub name: String,
    pub holder: String,
    pub commission: u64,
}

pub async fn validators(ctx: &Context, target: &Target) -> Result<Vec<ValidatorSummary>> {
    let mut node = SuiNode::connect(&ctx.config.sui.grpc_url)?;
    let mut summaries = Vec::new();
    for actor in ctx.actors(Chain::Ika, target)? {
        for (_, validator) in holdings(ctx, &mut node, &actor).await? {
            summaries.push(ValidatorSummary {
                id: validator.id,
                name: validator.name,
                holder: actor.label.clone(),
                commission: validator.commission,
            });
        }
    }
    Ok(summaries)
}

async fn status(ctx: &Context, node: &mut SuiNode, target: &Target) -> Result<()> {
    let mut total = 0u128;
    let mut validators = 0;
    for actor in ctx.actors(Chain::Ika, target)? {
        ui::header(&format!("IKA · {actor}"));
        let gas = node.balance(actor.address, sui::SUI_TYPE).await?.balance();
        let ika = node
            .balance(actor.address, &ctx.config.ika.coin_type)
            .await?
            .balance();
        println!(
            "  wallet       {} IKA, {} SUI for gas",
            amount::format(ika as i128),
            amount::format(gas as i128)
        );
        let holdings = holdings(ctx, node, &actor).await?;
        if holdings.is_empty() {
            ui::warn("this address holds no ValidatorCommissionCap");
        }
        for (_, validator) in holdings {
            println!(
                "  {}  {}  {}",
                ui::bold(&validator.name),
                validator.state,
                ui::dim(&validator.id.to_string())
            );
            println!(
                "    commission {} IKA ready to collect, rate {}%",
                ui::green(&amount::format(validator.commission as i128)),
                validator.commission_rate_bps as f64 / 100.0
            );
            total += validator.commission as u128;
            validators += 1;
        }
    }
    if validators > 1 {
        println!(
            "\n{} {} IKA across {validators} validators",
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
    amount: Option<&str>,
    args: &TxArgs,
) -> Result<()> {
    let amount = amount.map(amount::parse).transpose()?;
    let to = to
        .or(ctx.config.ika.recipient.as_deref())
        .map(keys::parse_address)
        .transpose()?;
    let ika = ika_type(ctx)?;
    let system = load_system(ctx, node).await?;
    let mut matched = vec![false; filters.len()];

    for actor in ctx.actors(Chain::Ika, &args.target)? {
        ui::header(&format!("IKA · {actor}"));
        let mut caps = Vec::new();
        for (cap, validator) in holdings(ctx, node, &actor).await? {
            if !filters.is_empty() {
                let mut hit = false;
                for (i, filter) in filters.iter().enumerate() {
                    if validator.matches(filter) {
                        matched[i] = true;
                        hit = true;
                    }
                }
                if !hit {
                    continue;
                }
            }
            let available = validator.commission;
            if available == 0 {
                ui::info(&format!("{}: no commission to collect", validator.name));
                continue;
            }
            let wanted = amount.unwrap_or(available);
            if wanted > available {
                bail!(
                    "{} has only {} IKA of commission",
                    validator.name,
                    amount::format(available as i128)
                );
            }
            ui::info(&format!(
                "{}: collecting {} of {} IKA",
                validator.name,
                amount::format(wanted as i128),
                amount::format(available as i128)
            ));
            caps.push(cap);
        }
        if caps.is_empty() {
            continue;
        }
        let recipient = to.unwrap_or(actor.address);
        let tx = collect_tx(&system, &caps, amount, recipient);
        let plan = Plan::new(&actor)
            .recipient(recipient)
            .coin(ika.clone(), "IKA")
            .package(system.package)
            .shared_object(system.id);
        node.execute(&actor, tx, args, ctx.config.sui.gas_budget, &plan)
            .await?;
    }

    for (filter, found) in filters.iter().zip(matched) {
        if !found {
            ui::warn(&format!("no validator {filter:?} among the IKA accounts"));
        }
    }
    Ok(())
}

fn collect_tx(
    system: &IkaSystem,
    caps: &[CommissionCap],
    amount: Option<u64>,
    recipient: Address,
) -> TransactionBuilder {
    let mut tx = TransactionBuilder::new();
    let system_arg = tx.object(ObjectInput::shared(
        system.id,
        system.initial_shared_version,
        true,
    ));
    let amount = tx.pure(&amount);
    let coins: Vec<_> = caps
        .iter()
        .map(|cap| {
            let cap = tx.object(ObjectInput::owned(cap.id, cap.version, cap.digest));
            tx.move_call(
                sui::function(system.package, "system", "collect_commission"),
                vec![system_arg, cap, amount],
            )
        })
        .collect();
    let (coin, rest) = coins.split_first().expect("at least one cap");
    if !rest.is_empty() {
        tx.merge_coins(*coin, rest.to_vec());
    }
    let recipient = tx.pure(&recipient);
    tx.transfer_objects(vec![*coin], recipient);
    tx
}
