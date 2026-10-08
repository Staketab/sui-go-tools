use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "mcli",
    version,
    about = "Validator toolkit for SUI, IKA, Walrus and IOTA",
    long_about = "Validator toolkit for SUI, IKA, Walrus and IOTA: withdraw stakes, collect IKA and \
                  Walrus commission, merge and send coins.\n\nPrivate keys are read from \
                  ~/.mcli/.env, so there is no CLI profile to switch. Run `mcli init` to create it."
)]
pub struct Cli {
    /// Directory with config.toml and .env
    #[arg(
        long,
        global = true,
        env = "MCLI_HOME",
        value_name = "DIR",
        default_value = "~/.mcli"
    )]
    pub home: PathBuf,

    /// Without a command mcli opens the interactive menu
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Interactive menu (the default when no command is given)
    Menu,
    /// Create config.toml and a .env template for private keys
    Init,
    /// List accounts loaded from .env (addresses only, keys are never printed)
    Accounts,
    /// SUI: validator stakes, coins and transfers
    #[command(subcommand)]
    Sui(ChainCommand),
    /// IOTA: validator stakes, coins and transfers
    #[command(subcommand)]
    Iota(ChainCommand),
    /// IKA validators: commission and IKA transfers (IKA runs on Sui)
    #[command(subcommand)]
    Ika(IkaCommand),
    /// Walrus storage nodes: commission and WAL transfers (Walrus runs on Sui)
    #[command(subcommand)]
    Walrus(WalrusCommand),
    /// Print the version as JSON
    Version,
}

#[derive(Subcommand)]
pub enum ChainCommand {
    /// Show balance, coin objects and stakes
    Balance(Target),
    /// Withdraw all active stakes (principal and rewards) in one transaction
    WithdrawAll {
        /// Send the withdrawn amount to this address instead of the account itself
        #[arg(long, value_name = "ADDRESS")]
        to: Option<String>,
        #[command(flatten)]
        tx: TxArgs,
    },
    /// Merge all coin objects into one
    MergeAll(TxArgs),
    /// Merge the given coin objects into the primary coin
    Merge {
        /// Coin that receives the others
        #[arg(short, long, value_name = "OBJECT_ID")]
        primary_coin: String,
        /// Coins to merge, comma-separated
        #[arg(
            short,
            long,
            value_name = "OBJECT_ID",
            value_delimiter = ',',
            required = true
        )]
        coins_to_merge: Vec<String>,
        #[command(flatten)]
        tx: TxArgs,
    },
    /// Send coins to an address
    Send {
        /// Recipient address
        #[arg(short = 'r', long, visible_alias = "recipient", value_name = "ADDRESS")]
        to: String,
        /// Amount in whole coins, e.g. 12.5 (not in MIST/NANOS)
        #[arg(short, long, required_unless_present = "all", conflicts_with = "all")]
        amount: Option<String>,
        /// Send the whole balance (IOTA only: SUI pays gas from the same balance)
        #[arg(long)]
        all: bool,
        #[command(flatten)]
        tx: TxArgs,
    },
}

#[derive(Subcommand)]
pub enum IkaCommand {
    /// Show validators whose commission caps are held by the IKA keys
    Status(Target),
    /// Collect validator commission in IKA
    Collect {
        /// Only this validator (name or validator ID); repeat for several. Default: all
        #[arg(long = "validator", value_name = "NAME|ID")]
        validators: Vec<String>,
        /// Send the IKA here (default: [ika].recipient from config.toml, else the cap holder)
        #[arg(long, value_name = "ADDRESS")]
        to: Option<String>,
        /// Collect only this many IKA per validator (default: everything accrued)
        #[arg(long)]
        amount: Option<String>,
        #[command(flatten)]
        tx: TxArgs,
    },
    /// Send IKA tokens from a validator account
    Send {
        /// Recipient address
        #[arg(short = 'r', long, visible_alias = "recipient", value_name = "ADDRESS")]
        to: String,
        /// Amount in whole IKA, e.g. 1000.5
        #[arg(short, long, required_unless_present = "all", conflicts_with = "all")]
        amount: Option<String>,
        /// Send the whole IKA balance
        #[arg(long)]
        all: bool,
        #[command(flatten)]
        tx: TxArgs,
    },
}

#[derive(Subcommand)]
pub enum WalrusCommand {
    /// Show storage nodes whose commission the Walrus keys receive
    Status(Target),
    /// Collect storage node commission in WAL
    Collect {
        /// Only this node (name or node ID); repeat for several. Default: all
        #[arg(long = "node", value_name = "NAME|ID")]
        nodes: Vec<String>,
        /// Send the WAL here (default: [walrus].recipient from config.toml, else the account itself)
        #[arg(long, value_name = "ADDRESS")]
        to: Option<String>,
        #[command(flatten)]
        tx: TxArgs,
    },
    /// Send WAL tokens from a node account
    Send {
        /// Recipient address
        #[arg(short = 'r', long, visible_alias = "recipient", value_name = "ADDRESS")]
        to: String,
        /// Amount in whole WAL, e.g. 1000.5
        #[arg(short, long, required_unless_present = "all", conflicts_with = "all")]
        amount: Option<String>,
        /// Send the whole WAL balance
        #[arg(long)]
        all: bool,
        #[command(flatten)]
        tx: TxArgs,
    },
}

/// Which accounts a command works with.
#[derive(Args, Clone, Default)]
pub struct Target {
    /// Use only this account: its .env label (`main` for SUI_PRIVATE_KEY_MAIN) or address
    #[arg(long, value_name = "LABEL|ADDRESS")]
    pub account: Option<String>,
    /// Work with this address without a key: read-only commands and dry runs
    #[arg(long, value_name = "ADDRESS", conflicts_with = "account")]
    pub address: Option<String>,
}

#[derive(Args, Clone, Default)]
pub struct TxArgs {
    #[command(flatten)]
    pub target: Target,
    /// Simulate only: nothing is signed or sent
    #[arg(long)]
    pub dry_run: bool,
    /// Execute without asking for confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
    /// Gas budget in MIST/NANOS (default: estimated by simulation)
    #[arg(long, value_name = "AMOUNT")]
    pub gas_budget: Option<u64>,
}
