//! Interactive mode: `mcli` without arguments walks through menus and builds the same
//! commands the CLI runs, so every transaction is simulated and confirmed before signing.

use anyhow::Result;
use inquire::validator::Validation;
use inquire::{InquireError, MultiSelect, Select, Text};
use std::fmt;

use crate::Context;
use crate::amount;
use crate::cli::{ChainCommand, IkaCommand, Target, TxArgs, WalrusCommand};
use crate::keys::{self, Chain};
use crate::{ika, iota, sui, ui, walrus};

/// Runs a prompt. Esc answers `None` (go back), Ctrl-C leaves the program.
fn ask<T>(answer: Result<T, InquireError>) -> Result<Option<T>> {
    match answer {
        Ok(value) => Ok(Some(value)),
        Err(InquireError::OperationCanceled) => Ok(None),
        Err(InquireError::OperationInterrupted) => {
            println!();
            std::process::exit(130);
        }
        Err(e) => Err(e.into()),
    }
}

/// The lists are short, so typing does not filter them: a filter that matches nothing would
/// leave an empty list on which Enter does nothing.
fn choose<T: fmt::Display>(message: &str, options: Vec<T>) -> Result<Option<T>> {
    let prompt = Select::new(message, options)
        .with_page_size(12)
        .without_filtering()
        .with_help_message("↑↓ to move, Enter to select, Esc to go back");
    ask(prompt.prompt())
}

fn choose_many<T: fmt::Display>(
    message: &str,
    options: Vec<T>,
    preselected: &[usize],
) -> Result<Option<Vec<T>>> {
    let prompt = MultiSelect::new(message, options)
        .with_default(preselected)
        .without_filtering()
        .with_help_message("↑↓ to move, Space to toggle, → all, ← none, Enter to confirm");
    ask(prompt.prompt())
}

fn address(message: &str) -> Result<Option<String>> {
    let prompt = Text::new(message).with_validator(|input: &str| {
        Ok(match keys::parse_address(input) {
            Ok(_) => Validation::Valid,
            Err(_) => Validation::Invalid("expected a 0x… address".into()),
        })
    });
    ask(prompt.prompt())
}

fn coins(message: &str) -> Result<Option<String>> {
    let prompt = Text::new(message)
        .with_help_message("whole coins, e.g. 12.5")
        .with_validator(|input: &str| {
            Ok(match amount::parse(input) {
                Ok(_) => Validation::Valid,
                Err(e) => Validation::Invalid(e.to_string().into()),
            })
        });
    ask(prompt.prompt())
}

fn is_all(input: &str) -> bool {
    input.trim().eq_ignore_ascii_case("all")
}

/// Asks for whole coins or `all`. `Some(None)` means all; with `default_all`, Enter alone
/// answers all.
fn coins_or_all(message: &str, help: &str, default_all: bool) -> Result<Option<Option<String>>> {
    let mut prompt = Text::new(message)
        .with_help_message(help)
        .with_validator(|input: &str| {
            if is_all(input) {
                return Ok(Validation::Valid);
            }
            if input.trim().is_empty() {
                return Ok(Validation::Invalid("type an amount or all".into()));
            }
            Ok(match amount::parse(input) {
                Ok(_) => Validation::Valid,
                Err(e) => Validation::Invalid(e.to_string().into()),
            })
        });
    if default_all {
        prompt = prompt.with_default("all");
    }
    Ok(ask(prompt.prompt())?.map(|answer| (!is_all(&answer)).then(|| answer.trim().to_owned())))
}

#[derive(Clone, Copy)]
enum Main {
    Ika,
    Walrus,
    Sui,
    Iota,
    Accounts,
    Quit,
}

impl fmt::Display for Main {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Main::Ika => "IKA validators",
            Main::Walrus => "Walrus nodes",
            Main::Sui => "SUI",
            Main::Iota => "IOTA",
            Main::Accounts => "Accounts",
            Main::Quit => "Quit",
        })
    }
}

pub async fn run(ctx: &Context) -> Result<()> {
    println!("{} {}", ui::bold("mcli"), env!("CARGO_PKG_VERSION"));
    let counts: Vec<String> = Chain::ALL
        .into_iter()
        .map(|chain| {
            format!(
                "{chain} {}",
                ctx.accounts.iter().filter(|a| a.chain == chain).count()
            )
        })
        .collect();
    println!(
        "{}",
        ui::dim(&format!(
            "keys from {}: {} · arrows to move, Enter to pick, Esc to go back",
            ctx.paths.env.display(),
            counts.join(", ")
        ))
    );
    let options = vec![
        Main::Ika,
        Main::Walrus,
        Main::Sui,
        Main::Iota,
        Main::Accounts,
        Main::Quit,
    ];
    loop {
        println!();
        let outcome = match choose("What do you want to do?", options.clone())? {
            None | Some(Main::Quit) => return Ok(()),
            Some(Main::Ika) => ika_menu(ctx).await,
            Some(Main::Walrus) => walrus_menu(ctx).await,
            Some(Main::Sui) => chain_menu(ctx, Chain::Sui).await,
            Some(Main::Iota) => chain_menu(ctx, Chain::Iota).await,
            Some(Main::Accounts) => {
                keys::print_accounts(&ctx.accounts, &ctx.paths.env);
                Ok(())
            }
        };
        if let Err(error) = outcome {
            ui::error(&format!("{error:#}"));
        }
    }
}

/// An account to act with, as offered in the menus.
enum Pick {
    All,
    Account { label: String, address: String },
    Watch,
}

impl fmt::Display for Pick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Pick::All => f.write_str("All accounts"),
            Pick::Account { label, address } => write!(f, "{label}  {address}"),
            Pick::Watch => f.write_str("Another address (read-only, dry run)"),
        }
    }
}

/// Asks which account to use; `None` means the user went back.
fn target(ctx: &Context, chain: Chain, allow_all: bool) -> Result<Option<Target>> {
    let accounts: Vec<_> = ctx.accounts.iter().filter(|a| a.chain == chain).collect();
    let mut options = Vec::new();
    if allow_all && accounts.len() > 1 {
        options.push(Pick::All);
    }
    options.extend(accounts.iter().map(|a| Pick::Account {
        label: a.label.clone(),
        address: a.address.to_string(),
    }));
    options.push(Pick::Watch);
    if accounts.is_empty() {
        ui::warn(&format!("no {chain} keys in {}", ctx.paths.env.display()));
    }
    Ok(match choose("Account", options)? {
        None => None,
        Some(Pick::All) => Some(Target::default()),
        Some(Pick::Account { address, .. }) => Some(Target {
            account: Some(address),
            address: None,
        }),
        Some(Pick::Watch) => address("Address")?.map(|a| Target {
            account: None,
            address: Some(a),
        }),
    })
}

fn tx(target: Target) -> TxArgs {
    TxArgs {
        target,
        ..TxArgs::default()
    }
}

/// Where withdrawn or collected funds go: `Some(None)` keeps them on the account.
fn destination(question: &str, default: &str) -> Result<Option<Option<String>>> {
    let other = "Another address…";
    match choose(question, vec![default, other])? {
        None => Ok(None),
        Some(choice) if choice == other => Ok(address("Recipient address")?.map(Some)),
        Some(_) => Ok(Some(None)),
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Action {
    Balance,
    WithdrawAll,
    MergeAll,
    Send,
    Back,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Action::Balance => "Balance and stakes",
            Action::WithdrawAll => "Withdraw all stakes",
            Action::MergeAll => "Merge all coins",
            Action::Send => "Send",
            Action::Back => "← Back",
        })
    }
}

async fn chain_menu(ctx: &Context, chain: Chain) -> Result<()> {
    let actions = vec![
        Action::Balance,
        Action::WithdrawAll,
        Action::MergeAll,
        Action::Send,
        Action::Back,
    ];
    loop {
        println!();
        let Some(action) =
            choose(&format!("{chain}"), actions.clone())?.filter(|a| *a != Action::Back)
        else {
            return Ok(());
        };
        let Some(command) = chain_command(ctx, chain, action)? else {
            continue;
        };
        let outcome = match chain {
            Chain::Iota => iota::run(ctx, command).await,
            _ => sui::run(ctx, command).await,
        };
        if let Err(error) = outcome {
            ui::error(&format!("{error:#}"));
        }
    }
}

fn chain_command(ctx: &Context, chain: Chain, action: Action) -> Result<Option<ChainCommand>> {
    let allow_all = action != Action::Send;
    let Some(target) = target(ctx, chain, allow_all)? else {
        return Ok(None);
    };
    Ok(match action {
        Action::Balance => Some(ChainCommand::Balance(target)),
        Action::MergeAll => Some(ChainCommand::MergeAll(tx(target))),
        Action::WithdrawAll => destination("Send the withdrawn coins to", "The account itself")?
            .map(|to| ChainCommand::WithdrawAll { to, tx: tx(target) }),
        Action::Send => {
            let Some(to) = address("Recipient address")? else {
                return Ok(None);
            };
            // SUI pays gas from the balance being sent, so only IOTA offers "all".
            let message = format!("Amount in {chain}");
            let amount = if chain == Chain::Iota {
                coins_or_all(&message, "whole coins, e.g. 12.5, or all", false)?
            } else {
                coins(&message)?.map(Some)
            };
            amount.map(|amount| ChainCommand::Send {
                to,
                all: amount.is_none(),
                amount,
                tx: tx(target),
            })
        }
        Action::Back => None,
    })
}

#[derive(Clone, Copy, PartialEq)]
enum IkaAction {
    Status,
    Collect,
    Send,
    Back,
}

impl fmt::Display for IkaAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            IkaAction::Status => "Validators and commission",
            IkaAction::Collect => "Collect commission",
            IkaAction::Send => "Send IKA",
            IkaAction::Back => "← Back",
        })
    }
}

async fn ika_menu(ctx: &Context) -> Result<()> {
    let actions = vec![
        IkaAction::Status,
        IkaAction::Collect,
        IkaAction::Send,
        IkaAction::Back,
    ];
    loop {
        println!();
        let Some(action) =
            choose("IKA validators", actions.clone())?.filter(|a| *a != IkaAction::Back)
        else {
            return Ok(());
        };
        let command = match action {
            IkaAction::Status => target(ctx, Chain::Ika, true)?.map(IkaCommand::Status),
            IkaAction::Collect => collect_command(ctx).await?,
            IkaAction::Send => send_ika_command(ctx)?,
            IkaAction::Back => None,
        };
        if let Some(command) = command
            && let Err(error) = ika::run(ctx, command).await
        {
            ui::error(&format!("{error:#}"));
        }
    }
}

struct ValidatorChoice(ika::ValidatorSummary);

impl fmt::Display for ValidatorChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let v = &self.0;
        write!(
            f,
            "{}  {} IKA  (key {})",
            v.name,
            amount::format(v.commission as i128),
            v.holder
        )
    }
}

async fn collect_command(ctx: &Context) -> Result<Option<IkaCommand>> {
    let Some(target) = target(ctx, Chain::Ika, true)? else {
        return Ok(None);
    };
    let found = ika::validators(ctx, &target).await?;
    if found.is_empty() {
        ui::warn("none of these addresses holds a ValidatorCommissionCap");
        return Ok(None);
    }
    if found.iter().all(|v| v.commission == 0) {
        ui::info("no commission to collect yet");
        return Ok(None);
    }
    let preselected: Vec<usize> = (0..found.len())
        .filter(|&i| found[i].commission > 0)
        .collect();
    let choices: Vec<ValidatorChoice> = found.into_iter().map(ValidatorChoice).collect();
    let Some(picked) = choose_many("Validators", choices, &preselected)? else {
        return Ok(None);
    };
    if picked.is_empty() {
        ui::info("no validators picked");
        return Ok(None);
    }

    let holder = match &ctx.config.ika.recipient {
        Some(recipient) => format!("Recipient from config.toml ({})", ui::short(recipient)),
        None => "The cap holder account".to_owned(),
    };
    let Some(to) = destination("Send the IKA to", &holder)? else {
        return Ok(None);
    };
    let message = if picked.len() > 1 {
        "How much IKA from each validator"
    } else {
        "How much IKA"
    };
    let Some(amount) = coins_or_all(message, "Enter takes everything accrued", true)? else {
        return Ok(None);
    };
    Ok(Some(IkaCommand::Collect {
        validators: picked.iter().map(|v| v.0.id.to_string()).collect(),
        to,
        amount,
        tx: tx(target),
    }))
}

fn send_ika_command(ctx: &Context) -> Result<Option<IkaCommand>> {
    let Some(target) = target(ctx, Chain::Ika, false)? else {
        return Ok(None);
    };
    let Some(to) = address("Recipient address")? else {
        return Ok(None);
    };
    let Some(amount) = coins_or_all("Amount in IKA", "whole coins, e.g. 12.5, or all", false)?
    else {
        return Ok(None);
    };
    Ok(Some(IkaCommand::Send {
        to,
        all: amount.is_none(),
        amount,
        tx: tx(target),
    }))
}

#[derive(Clone, Copy, PartialEq)]
enum WalrusAction {
    Status,
    Collect,
    Send,
    Back,
}

impl fmt::Display for WalrusAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            WalrusAction::Status => "Nodes and commission",
            WalrusAction::Collect => "Collect commission",
            WalrusAction::Send => "Send WAL",
            WalrusAction::Back => "← Back",
        })
    }
}

async fn walrus_menu(ctx: &Context) -> Result<()> {
    let actions = vec![
        WalrusAction::Status,
        WalrusAction::Collect,
        WalrusAction::Send,
        WalrusAction::Back,
    ];
    loop {
        println!();
        let Some(action) =
            choose("Walrus nodes", actions.clone())?.filter(|a| *a != WalrusAction::Back)
        else {
            return Ok(());
        };
        let command = match action {
            WalrusAction::Status => target(ctx, Chain::Walrus, true)?.map(WalrusCommand::Status),
            WalrusAction::Collect => walrus_collect_command(ctx).await?,
            WalrusAction::Send => walrus_send_command(ctx)?,
            WalrusAction::Back => None,
        };
        if let Some(command) = command
            && let Err(error) = walrus::run(ctx, command).await
        {
            ui::error(&format!("{error:#}"));
        }
    }
}

struct NodeChoice(walrus::NodeSummary);

impl fmt::Display for NodeChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = &self.0;
        write!(
            f,
            "{}  {} WAL  (key {})",
            n.name,
            amount::format(n.ready as i128),
            n.holder
        )
    }
}

async fn walrus_collect_command(ctx: &Context) -> Result<Option<WalrusCommand>> {
    let Some(target) = target(ctx, Chain::Walrus, true)? else {
        return Ok(None);
    };
    let found = walrus::nodes(ctx, &target).await?;
    if found.is_empty() {
        ui::warn("no storage node pays its commission to these addresses");
        return Ok(None);
    }
    if found.iter().all(|n| n.ready == 0) {
        let later: u64 = found.iter().map(|n| n.later).sum();
        if later > 0 {
            ui::info(&format!(
                "nothing to collect now; {} WAL becomes collectable when this epoch's voting ends",
                amount::format(later as i128)
            ));
        } else {
            ui::info("no commission to collect yet");
        }
        return Ok(None);
    }
    let preselected: Vec<usize> = (0..found.len()).filter(|&i| found[i].ready > 0).collect();
    let choices: Vec<NodeChoice> = found.into_iter().map(NodeChoice).collect();
    let Some(picked) = choose_many("Nodes", choices, &preselected)? else {
        return Ok(None);
    };
    if picked.is_empty() {
        ui::info("no nodes picked");
        return Ok(None);
    }
    let receiver = match &ctx.config.walrus.recipient {
        Some(recipient) => format!("Recipient from config.toml ({})", ui::short(recipient)),
        None => "The account itself".to_owned(),
    };
    let Some(to) = destination("Send the WAL to", &receiver)? else {
        return Ok(None);
    };
    Ok(Some(WalrusCommand::Collect {
        nodes: picked.iter().map(|n| n.0.id.to_string()).collect(),
        to,
        tx: tx(target),
    }))
}

fn walrus_send_command(ctx: &Context) -> Result<Option<WalrusCommand>> {
    let Some(target) = target(ctx, Chain::Walrus, false)? else {
        return Ok(None);
    };
    let Some(to) = address("Recipient address")? else {
        return Ok(None);
    };
    let Some(amount) = coins_or_all("Amount in WAL", "whole coins, e.g. 12.5, or all", false)?
    else {
        return Ok(None);
    };
    Ok(Some(WalrusCommand::Send {
        to,
        all: amount.is_none(),
        amount,
        tx: tx(target),
    }))
}
