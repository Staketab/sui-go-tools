mod amount;
mod cli;
mod config;
mod guard;
mod ika;
mod iota;
mod keys;
mod menu;
mod sui;
mod ui;
mod walrus;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use std::io::IsTerminal;

use cli::{Cli, Command, Target};
use config::{Config, Paths};
use keys::{Account, Actor, Chain};

/// Settings and keys shared by all commands.
pub struct Context {
    pub config: Config,
    pub paths: Paths,
    pub accounts: Vec<Account>,
}

impl Context {
    fn load(paths: Paths) -> Result<Self> {
        Ok(Self {
            config: Config::load(&paths.config)?,
            accounts: keys::load_accounts(&paths.env)?,
            paths,
        })
    }

    pub fn actors(&self, chain: Chain, target: &Target) -> Result<Vec<Actor<'_>>> {
        keys::actors(&self.accounts, &self.paths.env, chain, target)
    }

    pub fn single_actor(&self, chain: Chain, target: &Target) -> Result<Actor<'_>> {
        keys::single_actor(&self.accounts, &self.paths.env, chain, target)
    }
}

#[tokio::main]
async fn main() {
    if let Err(error) = run(Cli::parse()).await {
        ui::error(&format!("{error:#}"));
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    let paths = Paths::resolve(&cli.home)?;
    let command = match cli.command {
        Some(command) => command,
        None if std::io::stdin().is_terminal() => Command::Menu,
        None => {
            Cli::command().print_help()?;
            return Ok(());
        }
    };
    match command {
        Command::Menu => menu::run(&Context::load(paths)?).await,
        Command::Init => config::init(&paths),
        Command::Version => {
            println!("{{\"version\":\"{}\"}}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Command::Accounts => {
            let ctx = Context::load(paths)?;
            keys::print_accounts(&ctx.accounts, &ctx.paths.env);
            Ok(())
        }
        Command::Sui(command) => sui::run(&Context::load(paths)?, command).await,
        Command::Iota(command) => iota::run(&Context::load(paths)?, command).await,
        Command::Ika(command) => ika::run(&Context::load(paths)?, command).await,
        Command::Walrus(command) => walrus::run(&Context::load(paths)?, command).await,
    }
}
