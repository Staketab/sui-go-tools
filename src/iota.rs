//! IOTA over JSON-RPC. The node's transaction builder methods (`unsafe_*`) assemble the
//! transaction bytes; they are checked and signed locally with the key from .env. IOTA is a
//! fork of Sui with the same transaction format, so the SUI SDK types decode them.

use anyhow::{Context as _, Result, bail, ensure};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::time::Duration;
use sui_sdk_types::{Address, Transaction};

use crate::Context;
use crate::cli::{ChainCommand, Target, TxArgs};
use crate::keys::{self, Actor, Chain};
use crate::{amount, guard, ui};

const IOTA_TYPE: &str = "0x2::iota::IOTA";
const IOTA_COIN_TYPE: &str = "0x2::coin::Coin<0x2::iota::IOTA>";
const SYSTEM_STATE: Address = Address::from_static("0x5");
/// Each withdrawn stake takes two PTB commands and a PTB holds at most 1024.
const STAKES_PER_TX: usize = 400;
/// A transaction can pay gas with at most 256 coin objects.
const COINS_PER_TX: usize = 250;
/// Budget of the dry run that measures what a transaction really costs: 1 IOTA.
const PROBE_BUDGET: u64 = 1_000_000_000;
/// Cap on an estimated budget: 25 IOTA. Withdrawing 22 stakes needs ~0.5 IOTA up front
/// (storage, refunded right away) and costs ~0.002 IOTA.
const MAX_ESTIMATED_BUDGET: u64 = 25_000_000_000;

struct IotaNode {
    http: reqwest::Client,
    url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Coin {
    coin_object_id: String,
    balance: String,
}

impl Coin {
    fn balance(&self) -> u64 {
        self.balance.parse().unwrap_or(0)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CoinPage {
    data: Vec<Coin>,
    next_cursor: Option<String>,
    has_next_page: bool,
}

#[derive(Deserialize)]
struct StakeGroup {
    stakes: Vec<Stake>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Stake {
    staked_iota_id: String,
    principal: String,
    /// Active, Pending or Unstaked.
    status: String,
    estimated_reward: Option<String>,
}

impl Stake {
    fn principal(&self) -> u64 {
        self.principal.parse().unwrap_or(0)
    }

    fn reward(&self) -> u64 {
        self.estimated_reward
            .as_deref()
            .and_then(|r| r.parse().ok())
            .unwrap_or(0)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TxBytes {
    tx_bytes: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DryRun {
    effects: Effects,
    balance_changes: Vec<BalanceChange>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Effects {
    status: Status,
    gas_used: GasUsed,
}

#[derive(Deserialize)]
struct Status {
    status: String,
    error: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GasUsed {
    computation_cost: String,
    storage_cost: String,
    storage_rebate: String,
}

impl GasUsed {
    fn get(value: &str) -> i128 {
        value.parse().unwrap_or(0)
    }

    /// What the budget must cover: the storage rebate is only paid back afterwards.
    fn gross(&self) -> i128 {
        Self::get(&self.computation_cost) + Self::get(&self.storage_cost)
    }

    fn net(&self) -> i128 {
        self.gross() - Self::get(&self.storage_rebate)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BalanceChange {
    owner: Value,
    coin_type: String,
    amount: String,
}

#[derive(Deserialize)]
struct Executed {
    digest: String,
    effects: Option<Effects>,
}

/// Transactions the node's builder API assembles for us.
enum Build {
    /// Merges `coins` (the first one pays gas) and sends the result to `recipient`.
    PayAll {
        coins: Vec<String>,
        recipient: Address,
    },
    /// Splits `amount` off `coins` and sends it to `recipient`.
    Pay {
        coins: Vec<String>,
        recipient: Address,
        amount: u64,
    },
    /// A PTB of Move calls; gas is picked by the node.
    Batch(Vec<Value>),
}

impl IotaNode {
    fn new(url: &str) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()?;
        Ok(Self {
            http,
            url: url.to_owned(),
        })
    }

    async fn call<T: DeserializeOwned>(&self, method: &str, params: Value) -> Result<T> {
        let request = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
        let response: Value = self
            .http
            .post(&self.url)
            .json(&request)
            .send()
            .await
            .with_context(|| format!("{method}: cannot reach {}", self.url))?
            .error_for_status()?
            .json()
            .await?;
        if let Some(error) = response.get("error") {
            let message = error
                .get("message")
                .and_then(Value::as_str)
                .map_or_else(|| error.to_string(), str::to_owned);
            bail!("{method}: {message}");
        }
        serde_json::from_value(response["result"].clone())
            .with_context(|| format!("unexpected {method} response"))
    }

    /// IOTA coin objects, largest first.
    async fn coins(&self, owner: Address) -> Result<Vec<Coin>> {
        let mut coins = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let page: CoinPage = self
                .call(
                    "iotax_getCoins",
                    json!([owner.to_string(), IOTA_TYPE, cursor, 50]),
                )
                .await?;
            coins.extend(page.data);
            cursor = page.next_cursor;
            if !page.has_next_page || cursor.is_none() {
                break;
            }
        }
        coins.sort_by_key(|c| std::cmp::Reverse(c.balance()));
        Ok(coins)
    }

    async fn stakes(&self, owner: Address) -> Result<Vec<Stake>> {
        let groups: Vec<StakeGroup> = self
            .call("iotax_getStakes", json!([owner.to_string()]))
            .await?;
        Ok(groups.into_iter().flat_map(|g| g.stakes).collect())
    }

    async fn build(&self, signer: Address, build: &Build, budget: u64) -> Result<String> {
        let signer = signer.to_string();
        let budget = budget.to_string();
        let built: TxBytes = match build {
            Build::PayAll { coins, recipient } => {
                self.call(
                    "unsafe_payAllIota",
                    json!([signer, coins, recipient.to_string(), budget]),
                )
                .await?
            }
            Build::Pay {
                coins,
                recipient,
                amount,
            } => {
                let params = json!([
                    signer,
                    coins,
                    [recipient.to_string()],
                    [amount.to_string()],
                    budget
                ]);
                self.call("unsafe_payIota", params).await?
            }
            Build::Batch(commands) => {
                self.call(
                    "unsafe_batchTransaction",
                    json!([signer, commands, null, budget]),
                )
                .await?
            }
        };
        Ok(built.tx_bytes)
    }

    async fn dry_run(&self, tx_bytes: &str) -> Result<DryRun> {
        let dry_run: DryRun = self
            .call("iota_dryRunTransactionBlock", json!([tx_bytes]))
            .await?;
        let status = &dry_run.effects.status;
        if status.status != "success" {
            bail!(
                "dry run failed: {}",
                status.error.as_deref().unwrap_or("unknown error")
            );
        }
        Ok(dry_run)
    }

    /// Builds the transaction with a measured gas budget, shows what it will do and, unless
    /// this is a dry run, signs and executes it. `probe_budget` must fit the gas coin.
    async fn execute(
        &self,
        actor: &Actor<'_>,
        build: Build,
        args: &TxArgs,
        default_budget: Option<u64>,
        probe_budget: u64,
        recipient: Address,
    ) -> Result<()> {
        let budget = match args.gas_budget.or(default_budget) {
            Some(budget) => budget,
            None => {
                let probe = self.build(actor.address, &build, probe_budget).await?;
                let gas = self.dry_run(&probe).await?.effects.gas_used;
                let budget = (gas.gross() as u64).saturating_mul(11) / 10 + 1_000_000;
                ensure!(
                    budget <= MAX_ESTIMATED_BUDGET,
                    "estimated gas budget {} IOTA is above the {} IOTA limit; pass --gas-budget to set it yourself",
                    amount::format(budget as i128),
                    amount::format(MAX_ESTIMATED_BUDGET as i128)
                );
                budget
            }
        };
        let tx_bytes = self.build(actor.address, &build, budget).await?;
        let raw = BASE64.decode(&tx_bytes)?;
        let tx: Transaction =
            bcs::from_bytes(&raw).context("cannot decode the transaction built by the node")?;
        let mut expected = guard::Expected::new(actor.address, budget);
        expected.recipients.push(recipient);
        if let Build::Batch(_) = build {
            expected.shared_objects.push(SYSTEM_STATE);
        }
        guard::check(&tx, &expected)?;
        let dry_run = self.dry_run(&tx_bytes).await?;
        print_preview(&dry_run, budget, actor, recipient);

        let Some(account) = actor.account.filter(|_| !args.dry_run) else {
            ui::info("dry run: nothing was signed or sent");
            return Ok(());
        };
        if !args.yes && !ui::confirm("Sign and execute?")? {
            ui::warn("cancelled");
            return Ok(());
        }
        let signature = account.sign_iota(&raw)?.to_base64();
        let params = json!([tx_bytes, [signature], {"showEffects": true}, "WaitForLocalExecution"]);
        let executed: Executed = self.call("iota_executeTransactionBlock", params).await?;
        let digest = &executed.digest;
        match &executed.effects {
            Some(effects) if effects.status.status == "success" => {
                ui::ok(&format!("executed {}", ui::bold(digest)))
            }
            Some(effects) => bail!(
                "transaction {digest} failed: {}",
                effects.status.error.as_deref().unwrap_or("unknown error")
            ),
            None => ui::warn(&format!("sent {digest}, but the node returned no status")),
        }
        println!(
            "  https://explorer.iota.org/txblock/{}?network=mainnet",
            executed.digest
        );
        Ok(())
    }
}

fn print_preview(dry_run: &DryRun, budget: u64, actor: &Actor<'_>, recipient: Address) {
    println!("  {}", ui::dim("simulation:"));
    for change in &dry_run.balance_changes {
        let owner = change
            .owner
            .get("AddressOwner")
            .and_then(Value::as_str)
            .unwrap_or("?");
        let who = match keys::parse_address(owner).ok() {
            Some(a) if a == actor.address => format!("{} {}", actor.label, ui::short(owner)),
            Some(a) if a == recipient => format!("recipient {}", ui::short(owner)),
            _ => owner.to_owned(),
        };
        let amount: i128 = change.amount.parse().unwrap_or(0);
        if is_iota(&change.coin_type) {
            ui::change_line(&who, amount, "IOTA");
        } else {
            println!("    {who:<30} {amount} base units of {}", change.coin_type);
        }
    }
    println!(
        "    {:<30} {} IOTA (budget {})",
        "gas fee",
        amount::format(dry_run.effects.gas_used.net()),
        amount::format(budget as i128)
    );
}

fn is_iota(coin_type: &str) -> bool {
    coin_type == IOTA_TYPE || coin_type.ends_with("0000000000000000000002::iota::IOTA")
}

fn coin_ids(coins: &[Coin]) -> Vec<String> {
    coins.iter().map(|c| c.coin_object_id.clone()).collect()
}

fn sum(coins: &[Coin]) -> u64 {
    coins.iter().map(Coin::balance).sum()
}

pub async fn run(ctx: &Context, command: ChainCommand) -> Result<()> {
    let node = IotaNode::new(&ctx.config.iota.rpc_url)?;
    match command {
        ChainCommand::Balance(target) => balance(ctx, &node, &target).await,
        ChainCommand::WithdrawAll { to, tx } => withdraw_all(ctx, &node, to.as_deref(), &tx).await,
        ChainCommand::MergeAll(tx) => merge_all(ctx, &node, &tx).await,
        ChainCommand::Merge {
            primary_coin,
            coins_to_merge,
            tx,
        } => merge(ctx, &node, &primary_coin, &coins_to_merge, &tx).await,
        ChainCommand::Send {
            to,
            amount,
            all,
            tx,
        } => send(ctx, &node, &to, amount.as_deref(), all, &tx).await,
    }
}

async fn balance(ctx: &Context, node: &IotaNode, target: &Target) -> Result<()> {
    for actor in ctx.actors(Chain::Iota, target)? {
        ui::header(&format!("IOTA · {actor}"));
        let coins = node.coins(actor.address).await?;
        println!(
            "  balance   {} IOTA in {}",
            amount::format(sum(&coins) as i128),
            ui::count(coins.len(), "coin object")
        );
        let stakes = node.stakes(actor.address).await?;
        let active: Vec<_> = stakes.iter().filter(|s| s.status == "Active").collect();
        println!(
            "  stakes    {} active: {} IOTA + {} IOTA rewards",
            active.len(),
            amount::format(active.iter().map(|s| s.principal() as i128).sum()),
            amount::format(active.iter().map(|s| s.reward() as i128).sum())
        );
        let pending: Vec<_> = stakes.iter().filter(|s| s.status == "Pending").collect();
        if !pending.is_empty() {
            println!(
                "            {} pending until next epoch: {} IOTA",
                pending.len(),
                amount::format(pending.iter().map(|s| s.principal() as i128).sum())
            );
        }
    }
    Ok(())
}

async fn withdraw_all(
    ctx: &Context,
    node: &IotaNode,
    to: Option<&str>,
    args: &TxArgs,
) -> Result<()> {
    let to = to.map(keys::parse_address).transpose()?;
    for actor in ctx.actors(Chain::Iota, &args.target)? {
        ui::header(&format!("IOTA · {actor}"));
        let stakes = node.stakes(actor.address).await?;
        let pending = stakes.iter().filter(|s| s.status == "Pending").count();
        if pending > 0 {
            ui::info(&format!(
                "skipping {}: active from next epoch",
                ui::count(pending, "stake")
            ));
        }
        let active: Vec<&Stake> = stakes.iter().filter(|s| s.status == "Active").collect();
        if active.is_empty() {
            ui::info("no active stakes to withdraw");
            continue;
        }
        let gas_coin = node
            .coins(actor.address)
            .await?
            .first()
            .map_or(0, Coin::balance);
        if gas_coin == 0 {
            bail!("{} has no IOTA coin to pay gas with", actor.address);
        }
        let recipient = to.unwrap_or(actor.address);
        for chunk in active.chunks(STAKES_PER_TX) {
            ui::info(&format!(
                "withdrawing {}: {} IOTA + ~{} IOTA rewards",
                ui::count(chunk.len(), "stake"),
                amount::format(chunk.iter().map(|s| s.principal() as i128).sum()),
                amount::format(chunk.iter().map(|s| s.reward() as i128).sum())
            ));
            let build = Build::Batch(withdraw_commands(chunk, recipient));
            let probe = gas_coin.min(PROBE_BUDGET);
            node.execute(
                &actor,
                build,
                args,
                ctx.config.iota.gas_budget,
                probe,
                recipient,
            )
            .await?;
        }
    }
    Ok(())
}

/// One PTB: every stake is withdrawn as a Balance, the balances are joined and the recipient
/// gets a single coin instead of one coin per stake.
fn withdraw_commands(stakes: &[&Stake], recipient: Address) -> Vec<Value> {
    fn call(
        package: &str,
        module: &str,
        function: &str,
        type_args: &[&str],
        args: Vec<Value>,
    ) -> Value {
        json!({"moveCallRequestParams": {
            "packageObjectId": package,
            "module": module,
            "function": function,
            "typeArguments": type_args,
            "arguments": args,
        }})
    }
    let mut commands: Vec<Value> = stakes
        .iter()
        .map(|s| {
            let args = vec![json!("0x5"), json!(s.staked_iota_id)];
            call(
                "0x3",
                "iota_system",
                "request_withdraw_stake_non_entry",
                &[],
                args,
            )
        })
        .collect();
    for i in 1..stakes.len() {
        let args = vec![json!({"Result": 0}), json!({"Result": i})];
        commands.push(call("0x2", "balance", "join", &[IOTA_TYPE], args));
    }
    commands.push(call(
        "0x2",
        "coin",
        "from_balance",
        &[IOTA_TYPE],
        vec![json!({"Result": 0})],
    ));
    let coin = commands.len() - 1;
    let args = vec![json!({"Result": coin}), json!(recipient.to_string())];
    commands.push(call(
        "0x2",
        "transfer",
        "public_transfer",
        &[IOTA_COIN_TYPE],
        args,
    ));
    commands
}

async fn merge_all(ctx: &Context, node: &IotaNode, args: &TxArgs) -> Result<()> {
    for actor in ctx.actors(Chain::Iota, &args.target)? {
        ui::header(&format!("IOTA · {actor}"));
        let coins = node.coins(actor.address).await?;
        if coins.len() < 2 {
            ui::info(&format!(
                "nothing to merge: {}",
                ui::count(coins.len(), "IOTA coin object")
            ));
            continue;
        }
        for chunk in coins.chunks(COINS_PER_TX).filter(|c| c.len() > 1) {
            ui::info(&format!(
                "merging {} coins ({} IOTA) into {}",
                chunk.len(),
                amount::format(sum(chunk) as i128),
                chunk[0].coin_object_id
            ));
            let build = Build::PayAll {
                coins: coin_ids(chunk),
                recipient: actor.address,
            };
            let probe = sum(chunk).min(PROBE_BUDGET);
            node.execute(
                &actor,
                build,
                args,
                ctx.config.iota.gas_budget,
                probe,
                actor.address,
            )
            .await?;
        }
        if coins.len() > COINS_PER_TX {
            ui::info("run merge-all again to merge what is left");
        }
    }
    Ok(())
}

async fn merge(
    ctx: &Context,
    node: &IotaNode,
    primary: &str,
    others: &[String],
    args: &TxArgs,
) -> Result<()> {
    let actor = ctx.single_actor(Chain::Iota, &args.target)?;
    ui::header(&format!("IOTA · {actor}"));
    let owned = node.coins(actor.address).await?;
    let mut ids = Vec::new();
    let mut total = 0;
    for id in std::iter::once(primary).chain(others.iter().map(String::as_str)) {
        let wanted = keys::parse_address(id)?;
        let coin = owned
            .iter()
            .find(|c| keys::parse_address(&c.coin_object_id).ok() == Some(wanted))
            .with_context(|| format!("{id} is not an IOTA coin of {}", actor.address))?;
        ids.push(coin.coin_object_id.clone());
        total += coin.balance();
    }
    ui::info(&format!("merging {} coins into {}", ids.len() - 1, ids[0]));
    let build = Build::PayAll {
        coins: ids,
        recipient: actor.address,
    };
    node.execute(
        &actor,
        build,
        args,
        ctx.config.iota.gas_budget,
        total.min(PROBE_BUDGET),
        actor.address,
    )
    .await
}

async fn send(
    ctx: &Context,
    node: &IotaNode,
    to: &str,
    amount: Option<&str>,
    all: bool,
    args: &TxArgs,
) -> Result<()> {
    let actor = ctx.single_actor(Chain::Iota, &args.target)?;
    let recipient = keys::parse_address(to)?;
    ui::header(&format!("IOTA · {actor}"));
    let coins = node.coins(actor.address).await?;
    if coins.len() > COINS_PER_TX {
        ui::warn(&format!(
            "using the {COINS_PER_TX} largest of {} coins; run merge-all first to send everything",
            coins.len()
        ));
    }
    let coins = &coins[..coins.len().min(COINS_PER_TX)];
    let total = sum(coins);
    let (build, probe) = if all {
        ui::info(&format!(
            "sending all {} IOTA to {recipient}",
            amount::format(total as i128)
        ));
        (
            Build::PayAll {
                coins: coin_ids(coins),
                recipient,
            },
            total.min(PROBE_BUDGET),
        )
    } else {
        let amount = amount::parse(amount.context("--amount is required")?)?;
        if amount >= total {
            bail!(
                "{} IOTA is not enough to send {} IOTA and pay gas",
                amount::format(total as i128),
                amount::format(amount as i128)
            );
        }
        ui::info(&format!(
            "sending {} IOTA to {recipient}",
            amount::format(amount as i128)
        ));
        let build = Build::Pay {
            coins: coin_ids(coins),
            recipient,
            amount,
        };
        (build, (total - amount).min(PROBE_BUDGET))
    };
    node.execute(
        &actor,
        build,
        args,
        ctx.config.iota.gas_budget,
        probe,
        recipient,
    )
    .await
}
