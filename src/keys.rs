use anyhow::{Context as _, Result, anyhow, bail};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;
use sui_crypto::simple::SimpleKeypair;
use sui_crypto::{Signer as _, SuiSigner as _};
use sui_sdk_types::hash::Hasher;
use sui_sdk_types::{Address, MultisigMemberPublicKey, Transaction, UserSignature};

use crate::cli::Target;
use crate::ui;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Chain {
    Sui,
    Iota,
    /// IKA accounts are Sui accounts that hold IKA validator caps.
    Ika,
    /// Walrus accounts are Sui accounts that receive storage node commission.
    Walrus,
}

impl Chain {
    pub const ALL: [Chain; 4] = [Chain::Sui, Chain::Iota, Chain::Ika, Chain::Walrus];

    pub fn env_prefix(self) -> &'static str {
        match self {
            Chain::Sui => "SUI_PRIVATE_KEY",
            Chain::Iota => "IOTA_PRIVATE_KEY",
            Chain::Ika => "IKA_PRIVATE_KEY",
            Chain::Walrus => "WALRUS_PRIVATE_KEY",
        }
    }
}

impl fmt::Display for Chain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Chain::Sui => "SUI",
            Chain::Iota => "IOTA",
            Chain::Ika => "IKA",
            Chain::Walrus => "Walrus",
        })
    }
}

pub struct Account {
    pub chain: Chain,
    pub label: String,
    pub var: String,
    pub address: Address,
    keypair: SimpleKeypair,
}

impl Account {
    pub fn sign_sui(&self, tx: &Transaction) -> Result<UserSignature> {
        self.keypair
            .sign_transaction(tx)
            .map_err(|e| anyhow!("cannot sign with {}: {e}", self.var))
    }

    /// Signs transaction bytes built by an IOTA node. IOTA kept Sui's scheme: the key signs
    /// blake2b256(intent [0, 0, 0] || tx_bytes) and the signature is `flag || sig || pubkey`.
    pub fn sign_iota(&self, tx_bytes: &[u8]) -> Result<UserSignature> {
        let mut hasher = Hasher::new();
        hasher.update([0u8, 0, 0]);
        hasher.update(tx_bytes);
        let digest = hasher.finalize().into_inner();
        self.keypair
            .try_sign(&digest)
            .map_err(|e| anyhow!("cannot sign with {}: {e}", self.var))
    }
}

/// The address a command works with: an account from .env, or a bare address passed with
/// `--address`, which can only be read and dry-run.
pub struct Actor<'a> {
    pub label: String,
    pub address: Address,
    pub account: Option<&'a Account>,
}

impl fmt::Display for Actor<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.label, self.address)
    }
}

/// Loads keys from the .env file, then lets variables from the process environment
/// override them, so a one-off `SUI_PRIVATE_KEY=… mcli …` works too.
pub fn load_accounts(env_file: &Path) -> Result<Vec<Account>> {
    let mut vars = BTreeMap::new();
    if env_file.exists() {
        check_permissions(env_file)?;
        let entries = dotenvy::from_path_iter(env_file)
            .with_context(|| format!("cannot read {}", env_file.display()))?;
        for entry in entries {
            let (name, value) =
                entry.with_context(|| format!("invalid line in {}", env_file.display()))?;
            vars.insert(name, value);
        }
    }
    let environment = std::env::vars_os()
        .filter_map(|(name, value)| Some((name.into_string().ok()?, value.into_string().ok()?)));
    vars.extend(environment.filter(|(name, _)| classify(name).is_some()));

    let mut accounts = Vec::new();
    for (var, value) in vars {
        let Some((chain, label)) = classify(&var) else {
            continue;
        };
        if value.trim().is_empty() {
            continue;
        }
        let keypair =
            parse_key(chain, &value).with_context(|| format!("{var} is not a valid key"))?;
        let address = derive_address(chain, &keypair);
        accounts.push(Account {
            chain,
            label,
            var,
            address,
            keypair,
        });
    }
    Ok(accounts)
}

fn derive_address(chain: Chain, keypair: &SimpleKeypair) -> Address {
    let key = keypair.verifying_key();
    if chain == Chain::Iota
        && let MultisigMemberPublicKey::Ed25519(public_key) = key.public_key()
    {
        // IOTA kept Stardust-compatible Ed25519 addresses: the scheme flag is not hashed.
        return Address::new(Hasher::digest(public_key.inner()).into_inner());
    }
    key.derive_address()
}

/// Maps `IKA_PRIVATE_KEY_2` to (Ika, "2") and `SUI_PRIVATE_KEY` to (Sui, "default").
fn classify(var: &str) -> Option<(Chain, String)> {
    Chain::ALL.into_iter().find_map(|chain| {
        let rest = var.strip_prefix(chain.env_prefix())?;
        if rest.is_empty() {
            return Some((chain, "default".to_owned()));
        }
        let label = rest.strip_prefix('_').filter(|l| !l.is_empty())?;
        Some((chain, label.to_ascii_lowercase()))
    })
}

fn parse_key(chain: Chain, raw: &str) -> Result<SimpleKeypair> {
    let key = raw.trim();
    let lower = key.to_ascii_lowercase();
    if lower.starts_with("suiprivkey1") {
        if chain == Chain::Iota {
            bail!("this is a Sui key (suiprivkey…), IOTA needs an iotaprivkey… key");
        }
        return SimpleKeypair::from_suiprivkey(key).map_err(|e| anyhow!("{e}"));
    }
    if lower.starts_with("iotaprivkey1") {
        if chain != Chain::Iota {
            bail!("this is an IOTA key (iotaprivkey…), {chain} needs a suiprivkey… key");
        }
        let (hrp, payload) = bech32::decode(key).map_err(|e| anyhow!("bad iotaprivkey: {e}"))?;
        if hrp.as_str() != "iotaprivkey" {
            bail!("bad iotaprivkey prefix {hrp}");
        }
        // Same `flag || private key` payload as the keystore's base64 form.
        return SimpleKeypair::from_base64(&BASE64.encode(payload)).map_err(|e| anyhow!("{e}"));
    }
    match BASE64.decode(key) {
        Ok(bytes) if bytes.len() == 33 => {
            SimpleKeypair::from_base64(key).map_err(|e| anyhow!("{e}"))
        }
        _ => bail!("expected a suiprivkey1…/iotaprivkey1… string from `keytool export`"),
    }
}

#[cfg(unix)]
fn check_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(path)?.permissions().mode() & 0o777;
    if mode & 0o077 != 0 {
        bail!(
            "{path} holds private keys but other users can read it (mode {mode:o}); run: chmod 600 {path}",
            path = path.display()
        );
    }
    Ok(())
}

#[cfg(not(unix))]
fn check_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

pub fn parse_address(s: &str) -> Result<Address> {
    s.trim()
        .parse()
        .map_err(|_| anyhow!("invalid address {s:?}"))
}

/// Accounts of `chain` selected by `--account`/`--address`.
pub fn actors<'a>(
    accounts: &'a [Account],
    env_file: &Path,
    chain: Chain,
    target: &Target,
) -> Result<Vec<Actor<'a>>> {
    if let Some(address) = &target.address {
        return Ok(vec![Actor {
            label: "watch-only".into(),
            address: parse_address(address)?,
            account: None,
        }]);
    }
    let wanted = target.account.as_deref();
    let wanted_address = wanted.and_then(|w| parse_address(w).ok());
    let mut actors: Vec<Actor> = Vec::new();
    for account in accounts.iter().filter(|a| a.chain == chain) {
        let selected = match wanted {
            None => true,
            Some(w) => {
                account.label.eq_ignore_ascii_case(w) || wanted_address == Some(account.address)
            }
        };
        if selected && !actors.iter().any(|a| a.address == account.address) {
            actors.push(Actor {
                label: account.label.clone(),
                address: account.address,
                account: Some(account),
            });
        }
    }
    if actors.is_empty() {
        match wanted {
            Some(w) => bail!("no {chain} account {w:?} in {}", env_file.display()),
            None => bail!(
                "no {chain} keys: add {}=… to {} (or pass --address for a read-only run)",
                chain.env_prefix(),
                env_file.display()
            ),
        }
    }
    Ok(actors)
}

/// Like [`actors`], but for commands that must not fan out over several accounts.
pub fn single_actor<'a>(
    accounts: &'a [Account],
    env_file: &Path,
    chain: Chain,
    target: &Target,
) -> Result<Actor<'a>> {
    let mut actors = actors(accounts, env_file, chain, target)?;
    if actors.len() > 1 {
        let labels: Vec<_> = actors.iter().map(|a| a.label.as_str()).collect();
        bail!(
            "several {chain} accounts ({}), pick one with --account",
            labels.join(", ")
        );
    }
    Ok(actors.remove(0))
}

pub fn print_accounts(accounts: &[Account], env_file: &Path) {
    if accounts.is_empty() {
        let hint = if env_file.exists() {
            "fill in the template"
        } else {
            "run `mcli init` to create it"
        };
        ui::warn(&format!("no keys in {}: {hint}", env_file.display()));
        return;
    }
    for chain in Chain::ALL {
        let list: Vec<_> = accounts.iter().filter(|a| a.chain == chain).collect();
        if list.is_empty() {
            continue;
        }
        ui::header(&format!("{chain}"));
        for account in list {
            println!(
                "  {:<12} {}  ({})",
                account.label, account.address, account.var
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Keys derived from the public BIP-39 test mnemonic "abandon … about" by the Sui and IOTA CLIs.
    const SUI_KEY: &str = "suiprivkey1qzyxnjc8z79lvlsg6lz2hh69fp7m7duunfzjlnkzsd59f062855mqacydfr";
    const SUI_ADDRESS: &str = "0x5e93a736d04fbb25737aa40bee40171ef79f65fae833749e3c089fe7cc2161f1";
    const IOTA_KEY: &str =
        "iotaprivkey1qzc9ekmjpy5dhjqmwlpycr3npaagvg4gj44gq4ayt768fu8dkhjpwrhs7ue";
    const IOTA_ADDRESS: &str = "0x365b74f27ca7c6d7ce019d73042f85cc4627e1aeec2b7822994e16010234e576";

    fn address_of(chain: Chain, key: &str) -> String {
        derive_address(chain, &parse_key(chain, key).unwrap()).to_string()
    }

    #[test]
    fn derives_addresses_like_the_cli() {
        assert_eq!(address_of(Chain::Sui, SUI_KEY), SUI_ADDRESS);
        assert_eq!(address_of(Chain::Ika, SUI_KEY), SUI_ADDRESS);
        assert_eq!(address_of(Chain::Walrus, SUI_KEY), SUI_ADDRESS);
        assert_eq!(address_of(Chain::Iota, IOTA_KEY), IOTA_ADDRESS);
    }

    #[test]
    fn accepts_keystore_base64() {
        let (_, payload) = bech32::decode(SUI_KEY).unwrap();
        assert_eq!(address_of(Chain::Sui, &BASE64.encode(payload)), SUI_ADDRESS);
    }

    #[test]
    fn rejects_keys_of_another_chain() {
        assert!(parse_key(Chain::Iota, SUI_KEY).is_err());
        assert!(parse_key(Chain::Ika, IOTA_KEY).is_err());
        assert!(parse_key(Chain::Walrus, IOTA_KEY).is_err());
        assert!(parse_key(Chain::Sui, "0xdeadbeef").is_err());
    }

    #[test]
    fn signs_iota_transactions_like_the_cli() {
        // `iota keytool sign --address <IOTA_ADDRESS> --data <TX>` output for a mainnet
        // `unsafe_batchTransaction` withdrawing two stakes.
        const TX: &str = include_str!("../tests/data/iota_tx.b64");
        const SIGNATURE: &str = "APC/z2QYrI9IYFROaupkRdsJbmtJ3tOV6du/2OMA9FO00C3bQ2tiiWfBJwjHMleDfvgGz1VYqeSkUd7LsCzmBQmTHFS2eIN8+WpJ7h0RIgJ/q63wrul9n5CUGH24vjlvYw==";
        let account = Account {
            chain: Chain::Iota,
            label: "test".into(),
            var: "IOTA_PRIVATE_KEY".into(),
            address: IOTA_ADDRESS.parse().unwrap(),
            keypair: parse_key(Chain::Iota, IOTA_KEY).unwrap(),
        };
        let tx = BASE64.decode(TX.trim()).unwrap();
        assert_eq!(account.sign_iota(&tx).unwrap().to_base64(), SIGNATURE);
    }

    #[test]
    fn classifies_env_names() {
        assert_eq!(
            classify("SUI_PRIVATE_KEY"),
            Some((Chain::Sui, "default".into()))
        );
        assert_eq!(
            classify("IKA_PRIVATE_KEY_Staketab_1"),
            Some((Chain::Ika, "staketab_1".into()))
        );
        assert_eq!(
            classify("IOTA_PRIVATE_KEY_2"),
            Some((Chain::Iota, "2".into()))
        );
        assert_eq!(
            classify("WALRUS_PRIVATE_KEY"),
            Some((Chain::Walrus, "default".into()))
        );
        assert_eq!(classify("IOTA_PRIVATE_KEYS"), None);
        assert_eq!(classify("IKA_PRIVATE_KEY_"), None);
        assert_eq!(classify("HOME"), None);
    }
}
