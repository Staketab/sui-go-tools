//! SUI over gRPC. Mysten full nodes stopped serving JSON-RPC, so transactions are assembled
//! locally with the official SDK and resolved (object versions, gas) by simulating them.

use anyhow::{Context as _, Result, anyhow, bail};
use futures::TryStreamExt as _;
use serde_json::Value;
use std::time::Duration;
use sui_rpc::Client;
use sui_rpc::client::{DelegatedStake, ExecuteAndWaitError};
use sui_rpc::field::{FieldMask, FieldMaskUtil as _};
use sui_rpc::proto::sui::rpc::v2 as proto;
use sui_sdk_types::{Address, Digest, Identifier, StructTag, Transaction, TypeTag};
use sui_transaction_builder::{Function, ObjectInput, TransactionBuilder};

use crate::Context;
use crate::cli::{ChainCommand, Target, TxArgs};
use crate::keys::{self, Actor, Chain};
use crate::{amount, guard, ui};

pub const SUI_TYPE: &str = "0x2::sui::SUI";
const SYSTEM_STATE: Address = Address::from_static("0x5");
/// Each withdrawn stake takes two PTB commands and a PTB holds at most 1024.
const STAKES_PER_TX: usize = 400;
/// A transaction can pay gas with at most 256 coin objects.
const COINS_PER_TX: usize = 250;
const CHECKPOINT_TIMEOUT: Duration = Duration::from_secs(60);
/// Cap on a gas budget estimated by the node: 5 SUI. Withdrawing 22 stakes costs ~0.01 SUI.
const MAX_ESTIMATED_BUDGET: u64 = 5_000_000_000;

pub struct SuiNode {
    client: Client,
    explorer: &'static str,
}

pub struct CoinRef {
    pub id: Address,
    pub version: u64,
    pub digest: Digest,
    pub balance: u64,
}

impl CoinRef {
    fn input(&self) -> ObjectInput {
        ObjectInput::owned(self.id, self.version, self.digest)
    }
}

/// What a transaction may touch, checked before signing, and how its preview names
/// addresses and coin types.
pub struct Plan {
    names: Vec<(Address, String)>,
    coins: Vec<(StructTag, &'static str)>,
    packages: Vec<Address>,
    shared_objects: Vec<Address>,
    ids: Vec<Address>,
}

impl Plan {
    pub fn new(actor: &Actor<'_>) -> Self {
        Self {
            names: vec![(actor.address, actor.label.clone())],
            coins: vec![(StructTag::sui(), "SUI")],
            packages: Vec::new(),
            shared_objects: Vec::new(),
            ids: Vec::new(),
        }
    }

    /// Allows funds to go to `address`.
    pub fn recipient(mut self, address: Address) -> Self {
        if !self.names.iter().any(|(a, _)| *a == address) {
            self.names.push((address, "recipient".to_owned()));
        }
        self
    }

    pub fn coin(mut self, coin_type: StructTag, symbol: &'static str) -> Self {
        self.coins.push((coin_type, symbol));
        self
    }

    pub fn package(mut self, package: Address) -> Self {
        self.packages.push(package);
        self
    }

    pub fn shared_object(mut self, id: Address) -> Self {
        self.shared_objects.push(id);
        self
    }

    /// Allows `id` as a by-value argument: an object ID, not a destination for funds.
    pub fn id(mut self, id: Address) -> Self {
        self.ids.push(id);
        self
    }

    fn expected(&self, max_budget: u64) -> guard::Expected {
        let mut expected = guard::Expected::new(self.names[0].0, max_budget);
        expected.recipients = self.names.iter().map(|(a, _)| *a).collect();
        expected.packages = self.packages.clone();
        expected.shared_objects = self.shared_objects.clone();
        expected.ids = self.ids.clone();
        expected
    }

    fn print(&self, executed: &proto::ExecutedTransaction, budget: u64) {
        println!("  {}", ui::dim("simulation:"));
        for change in &executed.balance_changes {
            let owner = change.address();
            let who = keys::parse_address(owner)
                .ok()
                .and_then(|a| self.names.iter().find(|(n, _)| *n == a))
                .map(|(_, name)| format!("{name} {}", ui::short(owner)))
                .unwrap_or_else(|| owner.to_owned());
            let coin_type: Option<StructTag> = change.coin_type().parse().ok();
            let amount: i128 = change.amount().parse().unwrap_or(0);
            match self
                .coins
                .iter()
                .find(|(tag, _)| Some(tag) == coin_type.as_ref())
            {
                Some((_, symbol)) => ui::change_line(&who, amount, symbol),
                None => println!(
                    "    {who:<30} {amount} base units of {}",
                    change.coin_type()
                ),
            }
        }
        let gas = executed.effects().gas_used();
        let fee = gas.computation_cost() as i128 + gas.storage_cost() as i128
            - gas.storage_rebate() as i128;
        println!(
            "    {:<30} {} SUI (budget {})",
            "gas fee",
            amount::format(fee),
            amount::format(budget as i128)
        );
    }
}

impl SuiNode {
    pub fn connect(url: &str) -> Result<Self> {
        let client = Client::new(url).map_err(|e| anyhow!("bad Sui gRPC url {url}: {e}"))?;
        let explorer = if url.contains("testnet") {
            "https://suiscan.xyz/testnet/tx/"
        } else {
            "https://suiscan.xyz/mainnet/tx/"
        };
        Ok(Self { client, explorer })
    }

    pub async fn epoch(&mut self) -> Result<u64> {
        let request =
            proto::GetEpochRequest::latest().with_read_mask(FieldMask::from_paths(["epoch"]));
        let response = self
            .client
            .ledger_client()
            .get_epoch(request)
            .await?
            .into_inner();
        response
            .epoch
            .and_then(|e| e.epoch)
            .context("node returned no epoch")
    }

    /// Total of coin objects and the address balance.
    pub async fn balance(&mut self, owner: Address, coin_type: &str) -> Result<proto::Balance> {
        let request = proto::GetBalanceRequest::default()
            .with_owner(owner.to_string())
            .with_coin_type(coin_type);
        let response = self
            .client
            .state_client()
            .get_balance(request)
            .await?
            .into_inner();
        Ok(response.balance.unwrap_or_default())
    }

    /// Coin objects of `coin_type`, largest first.
    pub async fn coins(&mut self, owner: Address, coin_type: &str) -> Result<Vec<CoinRef>> {
        let request = proto::ListOwnedObjectsRequest::default()
            .with_owner(owner.to_string())
            .with_object_type(format!("0x2::coin::Coin<{coin_type}>"))
            .with_page_size(500u32)
            .with_read_mask(FieldMask::from_paths([
                "object_id",
                "version",
                "digest",
                "balance",
            ]));
        let objects: Vec<proto::Object> = self
            .client
            .list_owned_objects(request)
            .try_collect()
            .await?;
        let mut coins = objects
            .iter()
            .map(|o| {
                Ok(CoinRef {
                    id: keys::parse_address(o.object_id())?,
                    version: o.version(),
                    digest: parse_digest(o.digest())?,
                    balance: o.balance(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        coins.sort_by(|a, b| b.balance.cmp(&a.balance));
        Ok(coins)
    }

    /// Objects of exactly `object_type` owned by `owner`, with JSON contents.
    pub async fn owned_objects(
        &mut self,
        owner: Address,
        object_type: &str,
    ) -> Result<Vec<proto::Object>> {
        let request = proto::ListOwnedObjectsRequest::default()
            .with_owner(owner.to_string())
            .with_object_type(object_type)
            .with_page_size(500u32)
            .with_read_mask(FieldMask::from_paths([
                "object_id",
                "version",
                "digest",
                "json",
            ]));
        Ok(self
            .client
            .list_owned_objects(request)
            .try_collect()
            .await?)
    }

    pub async fn object(&mut self, id: Address) -> Result<proto::Object> {
        let request = proto::GetObjectRequest::default()
            .with_object_id(id.to_string())
            .with_read_mask(FieldMask::from_paths([
                "object_id",
                "version",
                "digest",
                "owner",
                "object_type",
                "json",
            ]));
        let response = self
            .client
            .ledger_client()
            .get_object(request)
            .await
            .with_context(|| format!("cannot load object {id}"))?;
        response
            .into_inner()
            .object
            .with_context(|| format!("object {id} not found"))
    }

    /// Dynamic fields of `parent` with the parts named by `paths`, e.g. `child_object.json`.
    pub async fn dynamic_fields(
        &mut self,
        parent: Address,
        paths: &[&str],
    ) -> Result<Vec<proto::DynamicField>> {
        let request = proto::ListDynamicFieldsRequest::default()
            .with_parent(parent.to_string())
            .with_page_size(1000u32)
            .with_read_mask(FieldMask::from_paths(paths));
        self.client
            .list_dynamic_fields(request)
            .try_collect()
            .await
            .with_context(|| format!("cannot list dynamic fields of {parent}"))
    }

    /// StakedSui objects with the rewards they have earned so far.
    pub async fn stakes(&mut self, owner: Address) -> Result<Vec<DelegatedStake>> {
        Ok(self.client.list_delegated_stake(&owner).await?)
    }

    /// Resolves the transaction on the node, shows what it will do and, unless this is a dry
    /// run, signs and executes it.
    pub async fn execute(
        &mut self,
        actor: &Actor<'_>,
        mut tx: TransactionBuilder,
        args: &TxArgs,
        default_budget: Option<u64>,
        plan: &Plan,
    ) -> Result<()> {
        tx.set_sender(actor.address);
        let budget = args.gas_budget.or(default_budget);
        if let Some(budget) = budget {
            tx.set_gas_budget(budget);
        }
        let tx = tx
            .build(&mut self.client)
            .await
            .map_err(|e| anyhow!("{e}"))?;
        guard::check(&tx, &plan.expected(budget.unwrap_or(MAX_ESTIMATED_BUDGET)))?;
        let simulated = self.simulate(&tx).await?;
        plan.print(&simulated, tx.gas_payment.budget);

        let Some(account) = actor.account.filter(|_| !args.dry_run) else {
            ui::info("dry run: nothing was signed or sent");
            return Ok(());
        };
        if !args.yes && !ui::confirm("Sign and execute?")? {
            ui::warn("cancelled");
            return Ok(());
        }
        let signature = account.sign_sui(&tx)?;
        let request = proto::ExecuteTransactionRequest::new(tx.into())
            .with_signatures(vec![signature.into()])
            .with_read_mask(FieldMask::from_paths(["digest", "effects.status"]));
        let executed = match self
            .client
            .execute_transaction_and_wait_for_checkpoint(request, CHECKPOINT_TIMEOUT)
            .await
        {
            Ok(response) => response.into_inner().transaction.unwrap_or_default(),
            Err(ExecuteAndWaitError::CheckpointTimeout(response))
            | Err(ExecuteAndWaitError::CheckpointStreamError { response, .. }) => {
                ui::warn("executed, but the node did not confirm a checkpoint in time");
                response.into_inner().transaction.unwrap_or_default()
            }
            Err(e) => bail!("execution failed: {e}"),
        };
        let digest = executed.digest();
        match executed.effects.as_ref().and_then(|e| e.status.as_ref()) {
            Some(status) if status.success() => ui::ok(&format!("executed {}", ui::bold(digest))),
            Some(status) => bail!("transaction {digest} failed: {}", describe(status.error())),
            None => ui::warn(&format!("sent {digest}, but the node returned no status")),
        }
        println!("  {}{digest}", self.explorer);
        Ok(())
    }

    async fn simulate(&mut self, tx: &Transaction) -> Result<proto::ExecutedTransaction> {
        let request = proto::SimulateTransactionRequest::new(tx.clone().into()).with_read_mask(
            FieldMask::from_paths([
                "transaction.effects.status",
                "transaction.effects.gas_used",
                "transaction.balance_changes",
            ]),
        );
        let response = self
            .client
            .execution_client()
            .simulate_transaction(request)
            .await?
            .into_inner();
        let executed = response.transaction.unwrap_or_default();
        let status = executed.effects().status();
        if !status.success() {
            bail!("simulation failed: {}", describe(status.error()));
        }
        Ok(executed)
    }
}

fn describe(error: &proto::ExecutionError) -> String {
    match error.description_opt() {
        Some(description) if !description.is_empty() => description.to_owned(),
        _ => format!("{error:?}"),
    }
}

fn parse_digest(text: &str) -> Result<Digest> {
    text.parse()
        .map_err(|_| anyhow!("invalid object digest {text:?}"))
}

pub fn function(package: Address, module: &'static str, name: &'static str) -> Function {
    Function::new(
        package,
        Identifier::from_static(module),
        Identifier::from_static(name),
    )
}

fn sui_type_tag() -> TypeTag {
    StructTag::sui().into()
}

/// Sends `amount` of a coin type, taking it from the address balance and coin objects.
pub fn transfer_tx(coin_type: StructTag, amount: u64, recipient: Address) -> TransactionBuilder {
    let mut tx = TransactionBuilder::new();
    let coin = tx.coin(coin_type, amount);
    let recipient = tx.pure(&recipient);
    tx.transfer_objects(vec![coin], recipient);
    tx
}

/// Sends a token (IKA, WAL) from the one selected account of `chain`, taking it from the
/// address balance and coin objects. Gas is paid in SUI.
#[allow(clippy::too_many_arguments)]
pub async fn send_coin(
    ctx: &Context,
    node: &mut SuiNode,
    chain: Chain,
    coin_type: &str,
    symbol: &'static str,
    to: &str,
    amount: Option<&str>,
    all: bool,
    args: &TxArgs,
) -> Result<()> {
    let actor = ctx.single_actor(chain, &args.target)?;
    let recipient = keys::parse_address(to)?;
    let tag: StructTag = coin_type
        .parse()
        .map_err(|_| anyhow!("invalid {symbol} coin type {coin_type:?}"))?;
    let amount = if all {
        node.balance(actor.address, coin_type).await?.balance()
    } else {
        amount::parse(amount.context("--amount is required")?)?
    };
    if amount == 0 {
        bail!("{} has no {symbol}", actor.address);
    }
    ui::header(&format!("{chain} · {actor}"));
    ui::info(&format!(
        "sending {} {symbol} to {recipient}",
        amount::format(amount as i128)
    ));
    let tx = transfer_tx(tag.clone(), amount, recipient);
    let plan = Plan::new(&actor).recipient(recipient).coin(tag, symbol);
    node.execute(&actor, tx, args, ctx.config.sui.gas_budget, &plan)
        .await
}

/// Moves object contents (`google.protobuf.Value`) into serde_json for field access.
pub fn json(object: &proto::Object) -> Value {
    object.json.as_deref().map(to_json).unwrap_or(Value::Null)
}

fn to_json(value: &prost_types::Value) -> Value {
    use prost_types::value::Kind;
    match &value.kind {
        None | Some(Kind::NullValue(_)) => Value::Null,
        Some(Kind::BoolValue(b)) => Value::Bool(*b),
        Some(Kind::NumberValue(n)) => {
            serde_json::Number::from_f64(*n).map_or(Value::Null, Value::Number)
        }
        Some(Kind::StringValue(s)) => Value::String(s.clone()),
        Some(Kind::ListValue(list)) => Value::Array(list.values.iter().map(to_json).collect()),
        Some(Kind::StructValue(s)) => Value::Object(
            s.fields
                .iter()
                .map(|(k, v)| (k.clone(), to_json(v)))
                .collect(),
        ),
    }
}

/// Move u64 values arrive as JSON strings, smaller integers as numbers.
pub fn json_u64(value: &Value) -> Option<u64> {
    match value {
        Value::String(s) => s.parse().ok(),
        Value::Number(n) => n.as_u64().or_else(|| n.as_f64().map(|f| f as u64)),
        _ => None,
    }
}

pub async fn run(ctx: &Context, command: ChainCommand) -> Result<()> {
    let mut node = SuiNode::connect(&ctx.config.sui.grpc_url)?;
    match command {
        ChainCommand::Balance(target) => balance(ctx, &mut node, &target).await,
        ChainCommand::WithdrawAll { to, tx } => {
            withdraw_all(ctx, &mut node, to.as_deref(), &tx).await
        }
        ChainCommand::MergeAll(tx) => merge_all(ctx, &mut node, &tx).await,
        ChainCommand::Merge {
            primary_coin,
            coins_to_merge,
            tx,
        } => merge(ctx, &mut node, &primary_coin, &coins_to_merge, &tx).await,
        ChainCommand::Send {
            to,
            amount,
            all,
            tx,
        } => send(ctx, &mut node, &to, amount.as_deref(), all, &tx).await,
    }
}

async fn balance(ctx: &Context, node: &mut SuiNode, target: &Target) -> Result<()> {
    let epoch = node.epoch().await?;
    for actor in ctx.actors(Chain::Sui, target)? {
        ui::header(&format!("SUI · {actor}"));
        let balance = node.balance(actor.address, SUI_TYPE).await?;
        let coins = node.coins(actor.address, SUI_TYPE).await?;
        println!(
            "  balance   {} SUI  (address balance {}, {})",
            amount::format(balance.balance() as i128),
            amount::format(balance.address_balance() as i128),
            ui::count(coins.len(), "coin object")
        );
        let stakes = node.stakes(actor.address).await?;
        let (active, pending): (Vec<_>, Vec<_>) =
            stakes.iter().partition(|s| s.activation_epoch <= epoch);
        let principal: u64 = active.iter().map(|s| s.principal).sum();
        let rewards: u64 = active.iter().map(|s| s.rewards).sum();
        println!(
            "  stakes    {} active: {} SUI + {} SUI rewards",
            active.len(),
            amount::format(principal as i128),
            amount::format(rewards as i128)
        );
        if !pending.is_empty() {
            let pending_sum: u64 = pending.iter().map(|s| s.principal).sum();
            println!(
                "            {} pending until next epoch: {} SUI",
                pending.len(),
                amount::format(pending_sum as i128)
            );
        }
    }
    Ok(())
}

async fn withdraw_all(
    ctx: &Context,
    node: &mut SuiNode,
    to: Option<&str>,
    args: &TxArgs,
) -> Result<()> {
    let to = to.map(keys::parse_address).transpose()?;
    let epoch = node.epoch().await?;
    for actor in ctx.actors(Chain::Sui, &args.target)? {
        ui::header(&format!("SUI · {actor}"));
        let stakes = node.stakes(actor.address).await?;
        // A stake requested this epoch has no rewards yet: withdrawing it only returns the principal.
        let (active, pending): (Vec<_>, Vec<_>) = stakes
            .into_iter()
            .partition(|s| s.activation_epoch <= epoch);
        if !pending.is_empty() {
            ui::info(&format!(
                "skipping {}: active from next epoch",
                ui::count(pending.len(), "stake")
            ));
        }
        if active.is_empty() {
            ui::info("no active stakes to withdraw");
            continue;
        }
        let recipient = to.unwrap_or(actor.address);
        for chunk in active.chunks(STAKES_PER_TX) {
            let principal: u64 = chunk.iter().map(|s| s.principal).sum();
            let rewards: u64 = chunk.iter().map(|s| s.rewards).sum();
            ui::info(&format!(
                "withdrawing {}: {} SUI + {} SUI rewards",
                ui::count(chunk.len(), "stake"),
                amount::format(principal as i128),
                amount::format(rewards as i128)
            ));
            let tx = withdraw_tx(chunk, recipient);
            let plan = Plan::new(&actor)
                .recipient(recipient)
                .shared_object(SYSTEM_STATE);
            node.execute(&actor, tx, args, ctx.config.sui.gas_budget, &plan)
                .await?;
        }
    }
    Ok(())
}

/// One PTB: every stake is withdrawn as a Balance, the balances are joined and sent to the
/// recipient's address balance, so no coin object is created per stake.
fn withdraw_tx(stakes: &[DelegatedStake], recipient: Address) -> TransactionBuilder {
    let mut tx = TransactionBuilder::new();
    let system = tx.object(ObjectInput::shared(SYSTEM_STATE, 1, true));
    let mut total = None;
    for stake in stakes {
        let staked = tx.object(ObjectInput::new(stake.staked_sui_id));
        let withdrawn = tx.move_call(
            function(
                Address::THREE,
                "sui_system",
                "request_withdraw_stake_non_entry",
            ),
            vec![system, staked],
        );
        match total {
            None => total = Some(withdrawn),
            Some(total) => {
                tx.move_call(
                    function(Address::TWO, "balance", "join").with_type_args(vec![sui_type_tag()]),
                    vec![total, withdrawn],
                );
            }
        }
    }
    let total = total.expect("at least one stake");
    let recipient = tx.pure(&recipient);
    tx.move_call(
        function(Address::TWO, "balance", "send_funds").with_type_args(vec![sui_type_tag()]),
        vec![total, recipient],
    );
    tx
}

/// Paying gas with all the coins merges them into the first one, which then goes back to
/// the owner.
fn merge_tx(coins: &[CoinRef], owner: Address) -> TransactionBuilder {
    let mut tx = TransactionBuilder::new();
    tx.add_gas_objects(coins.iter().map(CoinRef::input));
    let gas = tx.gas();
    let owner = tx.pure(&owner);
    tx.transfer_objects(vec![gas], owner);
    tx
}

async fn merge_all(ctx: &Context, node: &mut SuiNode, args: &TxArgs) -> Result<()> {
    for actor in ctx.actors(Chain::Sui, &args.target)? {
        ui::header(&format!("SUI · {actor}"));
        let coins = node.coins(actor.address, SUI_TYPE).await?;
        if coins.len() < 2 {
            ui::info(&format!(
                "nothing to merge: {}",
                ui::count(coins.len(), "SUI coin object")
            ));
            continue;
        }
        for chunk in coins.chunks(COINS_PER_TX).filter(|c| c.len() > 1) {
            let sum: u64 = chunk.iter().map(|c| c.balance).sum();
            ui::info(&format!(
                "merging {} coins ({} SUI) into {}",
                chunk.len(),
                amount::format(sum as i128),
                chunk[0].id
            ));
            let tx = merge_tx(chunk, actor.address);
            node.execute(
                &actor,
                tx,
                args,
                ctx.config.sui.gas_budget,
                &Plan::new(&actor),
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
    node: &mut SuiNode,
    primary: &str,
    others: &[String],
    args: &TxArgs,
) -> Result<()> {
    let actor = ctx.single_actor(Chain::Sui, &args.target)?;
    ui::header(&format!("SUI · {actor}"));
    let mut coins = Vec::new();
    for id in std::iter::once(primary).chain(others.iter().map(String::as_str)) {
        let id = keys::parse_address(id)?;
        if coins.iter().any(|c: &CoinRef| c.id == id) {
            bail!("coin {id} is listed twice");
        }
        let object = node.object(id).await?;
        let is_sui_coin =
            object.object_type().parse::<StructTag>().ok() == Some(StructTag::gas_coin());
        if !is_sui_coin {
            bail!("{id} is not a SUI coin ({})", object.object_type());
        }
        if keys::parse_address(object.owner().address()).ok() != Some(actor.address) {
            bail!("coin {id} is not owned by {}", actor.address);
        }
        coins.push(CoinRef {
            id,
            version: object.version(),
            digest: parse_digest(object.digest())?,
            balance: 0,
        });
    }
    ui::info(&format!(
        "merging {} coins into {}",
        coins.len() - 1,
        coins[0].id
    ));
    let tx = merge_tx(&coins, actor.address);
    node.execute(
        &actor,
        tx,
        args,
        ctx.config.sui.gas_budget,
        &Plan::new(&actor),
    )
    .await
}

async fn send(
    ctx: &Context,
    node: &mut SuiNode,
    to: &str,
    amount: Option<&str>,
    all: bool,
    args: &TxArgs,
) -> Result<()> {
    if all {
        bail!(
            "--all is not supported for SUI because gas comes from the same balance; pass --amount"
        );
    }
    let actor = ctx.single_actor(Chain::Sui, &args.target)?;
    let recipient = keys::parse_address(to)?;
    let amount = amount::parse(amount.context("--amount is required")?)?;
    ui::header(&format!("SUI · {actor}"));
    ui::info(&format!(
        "sending {} SUI to {recipient}",
        amount::format(amount as i128)
    ));
    let tx = transfer_tx(StructTag::sui(), amount, recipient);
    let plan = Plan::new(&actor).recipient(recipient);
    node.execute(&actor, tx, args, ctx.config.sui.gas_budget, &plan)
        .await
}
