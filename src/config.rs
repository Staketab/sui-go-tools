use anyhow::{Context as _, Result};
use serde::Deserialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::ui;

pub struct Paths {
    pub home: PathBuf,
    pub config: PathBuf,
    pub env: PathBuf,
}

impl Paths {
    pub fn resolve(home: &Path) -> Result<Self> {
        let home = match home.strip_prefix("~") {
            Ok(rest) => {
                PathBuf::from(std::env::var_os("HOME").context("HOME is not set")?).join(rest)
            }
            Err(_) => home.to_path_buf(),
        };
        Ok(Self {
            config: home.join("config.toml"),
            env: home.join(".env"),
            home,
        })
    }
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub sui: SuiConfig,
    pub iota: IotaConfig,
    pub ika: IkaConfig,
    pub walrus: WalrusConfig,
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SuiConfig {
    pub grpc_url: String,
    pub gas_budget: Option<u64>,
}

impl Default for SuiConfig {
    fn default() -> Self {
        Self {
            grpc_url: "https://fullnode.mainnet.sui.io:443".into(),
            gas_budget: None,
        }
    }
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct IotaConfig {
    pub rpc_url: String,
    pub gas_budget: Option<u64>,
}

impl Default for IotaConfig {
    fn default() -> Self {
        Self {
            rpc_url: "https://api.mainnet.iota.cafe".into(),
            gas_budget: None,
        }
    }
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct IkaConfig {
    pub system_object_id: String,
    /// Package that defines `validator_cap::ValidatorCommissionCap`. Type names keep the
    /// original package ID across upgrades, so this stays valid after IKA upgrades.
    pub common_package_id: String,
    pub coin_type: String,
    pub recipient: Option<String>,
}

impl Default for IkaConfig {
    fn default() -> Self {
        Self {
            system_object_id: "0x215de95d27454d102d6f82ff9c54d8071eb34d5706be85b5c73cbd8173013c80"
                .into(),
            common_package_id: "0x9e1e9f8e4e51ee2421a8e7c0c6ab3ef27c337025d15333461b72b1b813c44175"
                .into(),
            coin_type:
                "0x7262fb2f7a3a14c888c438a3cd9b912469a58cf60f367352c46584262e8299aa::ika::IKA"
                    .into(),
            recipient: None,
        }
    }
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WalrusConfig {
    /// The shared `staking::Staking` object; packages and pools are read from it.
    pub staking_object_id: String,
    pub coin_type: String,
    pub recipient: Option<String>,
}

impl Default for WalrusConfig {
    fn default() -> Self {
        Self {
            staking_object_id: "0x10b9d30c28448939ce6c4d6c6e0ffce4a7f8a4ada8248bdad09ef8b70e4a3904"
                .into(),
            coin_type:
                "0x356a26eb9e012a68958082340d4c4116e7f55615cf27affcff209cf0ae544f59::wal::WAL"
                    .into(),
            recipient: None,
        }
    }
}

impl Config {
    /// A missing config.toml is fine: every setting has a mainnet default.
    pub fn load(path: &Path) -> Result<Self> {
        match fs::read_to_string(path) {
            Ok(text) => {
                toml::from_str(&text).with_context(|| format!("invalid {}", path.display()))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e).with_context(|| format!("cannot read {}", path.display())),
        }
    }
}

const CONFIG_TEMPLATE: &str = r#"# mcli settings. Private keys live in .env next to this file, never here.

[sui]
# gRPC endpoint of a Sui full node. Mysten nodes no longer serve JSON-RPC.
grpc_url = "https://fullnode.mainnet.sui.io:443"
# Gas budget in MIST. By default it is estimated by simulating the transaction.
# gas_budget = 50000000

[iota]
# JSON-RPC endpoint of an IOTA node.
rpc_url = "https://api.mainnet.iota.cafe"
# Gas budget in NANOS. By default it is estimated with a dry run.
# gas_budget = 50000000

[ika]
# IKA lives on Sui and is reached through [sui].grpc_url. Mainnet objects:
system_object_id = "0x215de95d27454d102d6f82ff9c54d8071eb34d5706be85b5c73cbd8173013c80"
common_package_id = "0x9e1e9f8e4e51ee2421a8e7c0c6ab3ef27c337025d15333461b72b1b813c44175"
coin_type = "0x7262fb2f7a3a14c888c438a3cd9b912469a58cf60f367352c46584262e8299aa::ika::IKA"
# Where `mcli ika collect` sends the commission. Default: the address holding the commission cap.
# recipient = "0x..."

[walrus]
# Walrus lives on Sui and is reached through [sui].grpc_url. Mainnet objects:
staking_object_id = "0x10b9d30c28448939ce6c4d6c6e0ffce4a7f8a4ada8248bdad09ef8b70e4a3904"
coin_type = "0x356a26eb9e012a68958082340d4c4116e7f55615cf27affcff209cf0ae544f59::wal::WAL"
# Where `mcli walrus collect` sends the commission. Default: the account that collects it.
# recipient = "0x..."
"#;

const ENV_TEMPLATE: &str = r#"# mcli private keys. Only you may read this file: chmod 600 .env
#
# One variable per key: <CHAIN>_PRIVATE_KEY or <CHAIN>_PRIVATE_KEY_<LABEL>.
# The label selects an account: `mcli ika collect --account 2` uses IKA_PRIVATE_KEY_2.
#
# Export keys from the CLI keystores:
#   sui keytool export --key-identity <address>    -> suiprivkey1...
#   iota keytool export <address>                  -> iotaprivkey1...

# SUI validator account: rewards arrive as StakedSui (withdraw-all, merge-all, send)
SUI_PRIVATE_KEY=

# IOTA validator account: rewards arrive as StakedIota
IOTA_PRIVATE_KEY=

# IKA validators: Sui accounts that hold a ValidatorCommissionCap, one per validator
IKA_PRIVATE_KEY_1=
IKA_PRIVATE_KEY_2=

# Walrus storage nodes: the Sui account set as the node's commission receiver, or the one
# holding the StorageNodeCap when the commission receiver is the cap
WALRUS_PRIVATE_KEY=
"#;

pub fn init(paths: &Paths) -> Result<()> {
    create_private_dir(&paths.home)?;
    write_new(&paths.config, CONFIG_TEMPLATE, 0o644)?;
    write_new(&paths.env, ENV_TEMPLATE, 0o600)?;
    println!(
        "\nNext: put your keys into {} and check them with `mcli accounts`.",
        paths.env.display()
    );
    Ok(())
}

fn create_private_dir(dir: &Path) -> Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    builder
        .create(dir)
        .with_context(|| format!("cannot create {}", dir.display()))
}

fn write_new(path: &Path, content: &str, mode: u32) -> Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, mode);
    #[cfg(not(unix))]
    let _ = mode;
    match options.open(path) {
        Ok(mut file) => {
            file.write_all(content.as_bytes())
                .with_context(|| format!("cannot write {}", path.display()))?;
            ui::ok(&format!("created {}", path.display()));
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            ui::info(&format!("kept existing {}", path.display()));
            Ok(())
        }
        Err(e) => Err(e).with_context(|| format!("cannot create {}", path.display())),
    }
}
