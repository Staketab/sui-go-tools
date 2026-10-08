# MCLI

Validator toolkit for SUI, IKA, Walrus and IOTA:

- **IKA**: see the validators whose `ValidatorCommissionCap` your keys hold and collect their commission.
- **Walrus**: see the storage nodes whose commission your keys receive and collect it.
- **SUI**: withdraw all `StakedSui` (principal and rewards) in one transaction, merge coins, send.
- **IOTA**: withdraw all `StakedIota` in one transaction into a single coin, merge coins, send.

Private keys are read from `~/.mcli/.env`, so there is no `sui`/`iota` CLI profile to switch and those
binaries are not needed. Run `mcli` without arguments for an interactive menu, or use the commands below.

## Install

With [rustup](https://rustup.rs):

```
git clone https://github.com/Staketab/sui-go-tools.git
cd sui-go-tools
make                    # builds ./mcli
make install            # also puts mcli into ~/.cargo/bin
```

`rust-toolchain.toml` pins Rust 1.94.1; rustup installs it on the first build next to your default
toolchain. The build needs about 1 GB for `target/`. To keep it on another disk, set `CARGO_TARGET_DIR`
or put `[build] target-dir = "…"` into an untracked `.cargo/config.toml`.

Prebuilt `mcli-linux-amd64` and `mcli-darwin-arm64` binaries are attached to each GitHub release.

## Set up keys

```
mcli init               # creates ~/.mcli/config.toml and ~/.mcli/.env (mode 600)
$EDITOR ~/.mcli/.env
mcli accounts           # shows the address of every key, never the key
```

One variable per key, `<CHAIN>_PRIVATE_KEY` or `<CHAIN>_PRIVATE_KEY_<LABEL>`:

```
SUI_PRIVATE_KEY=suiprivkey1...       # SUI validator account
IOTA_PRIVATE_KEY=iotaprivkey1...     # IOTA validator account
IKA_PRIVATE_KEY_1=suiprivkey1...     # address holding the commission cap of IKA validator 1
IKA_PRIVATE_KEY_2=suiprivkey1...     # ... of IKA validator 2
WALRUS_PRIVATE_KEY=suiprivkey1...    # commission receiver of the Walrus storage node
```

Export keys from the CLI keystores with `sui keytool export --key-identity <address>` and
`iota keytool export <address>`. The label after `_PRIVATE_KEY_` selects an account:
`--account 2` uses `IKA_PRIVATE_KEY_2`. mcli refuses to read a `.env` that other users can read.

A Walrus storage pool names three parties: the operator (holds the `StorageNodeCap`), the
governance-authorized party and the commission receiver. Only the commission receiver can collect, so
`WALRUS_PRIVATE_KEY` is its key; when the receiver is the `StorageNodeCap` itself, use the key that
holds the cap. `mcli walrus status --address 0x…` shows which role an address has.

## Interactive menu

```
$ mcli
? What do you want to do?
> IKA validators
  Walrus nodes
  SUI
  IOTA
  Accounts
  Quit
```

Pick a chain, an action, the account and the details; arrows move, Enter picks, Esc goes back.
Before anything is signed the transaction is simulated and you confirm it.

## Commands

```
mcli ika status                                  # validators, commission ready to collect, gas
mcli ika collect                                 # collect everything from all validators
mcli ika collect --validator Staketab-1 --to 0x…  # one validator, IKA sent to another address
mcli ika collect --amount 1000                   # 1000 IKA from each validator
mcli ika send --account 1 --to 0x… --all

mcli walrus status                               # nodes, commission ready to collect, gas
mcli walrus collect                              # collect from every node the keys receive commission of
mcli walrus collect --node Staketab --to 0x…
mcli walrus send --to 0x… --amount 100           # or --all

mcli sui balance
mcli sui withdraw-all                            # all active stakes, rewards included
mcli sui withdraw-all --to 0x…
mcli sui merge-all
mcli sui send --to 0x… --amount 12.5

mcli iota balance
mcli iota withdraw-all
mcli iota merge-all
mcli iota send --to 0x… --amount 12.5            # or --all
```

Amounts are in whole coins (`12.5` IKA), not in MIST/NANOS. Commands that send a transaction accept:

| Flag | Meaning |
| --- | --- |
| `--dry-run` | simulate and show the result; nothing is signed or sent |
| `--address 0x…` | work with an address that has no key: read-only commands and dry runs |
| `--account <label\|address>` | use one account when `.env` has several for the chain |
| `-y`, `--yes` | do not ask for confirmation (for scripts) |
| `--gas-budget <base units>` | set the gas budget instead of estimating it |

## Safety

- Every transaction is simulated first: you see the balance changes and the gas fee, then confirm.
  Without a terminal mcli refuses to sign unless `--yes` is given.
- The RPC node resolves (SUI) or assembles (IOTA) transactions, so mcli checks the bytes before signing:
  the sender pays, only the framework, IKA and Walrus packages are called, only the expected shared
  objects are used, funds go only to the account itself or the recipient you chose, and the gas budget
  stays under 5 SUI / 25 IOTA unless you pass `--gas-budget`.
- Keys stay in `.env` (mode 600) and are never printed.

## Configuration

`~/.mcli/config.toml` is optional; every setting has a mainnet default:

```toml
[sui]
grpc_url = "https://fullnode.mainnet.sui.io:443"   # gRPC: Mysten nodes no longer serve JSON-RPC
# gas_budget = 50000000                            # MIST; estimated by default

[iota]
rpc_url = "https://api.mainnet.iota.cafe"
# gas_budget = 50000000                            # NANOS; estimated by default

[ika]
system_object_id = "0x215de95d27454d102d6f82ff9c54d8071eb34d5706be85b5c73cbd8173013c80"
common_package_id = "0x9e1e9f8e4e51ee2421a8e7c0c6ab3ef27c337025d15333461b72b1b813c44175"
coin_type = "0x7262fb2f7a3a14c888c438a3cd9b912469a58cf60f367352c46584262e8299aa::ika::IKA"
# recipient = "0x…"                                # default destination of collected commission

[walrus]
staking_object_id = "0x10b9d30c28448939ce6c4d6c6e0ffce4a7f8a4ada8248bdad09ef8b70e4a3904"
coin_type = "0x356a26eb9e012a68958082340d4c4116e7f55615cf27affcff209cf0ae544f59::wal::WAL"
# recipient = "0x…"                                # default destination of collected commission
```

`MCLI_HOME` or `--home` points mcli at another directory.

## How it works

- **SUI** goes through gRPC and the official Mysten Rust SDK. Transactions are resolved by simulation on
  the node. `withdraw-all` calls `request_withdraw_stake_non_entry` for every stake, joins the balances
  and sends them to the address balance, so no coin object is created per stake.
- **IKA** runs on Sui. Commission accrues inside each validator's staking pool and
  `ika_system::system::collect_commission` pays it out to whoever presents the validator's
  `ValidatorCommissionCap`. The current `ika_system` package is read from the IKA system object, so IKA
  upgrades need no config change.
- **Walrus** runs on Sui too. Commission accrues inside each storage node's staking pool and
  `walrus::staking::collect_commission` pays it to the pool's commission receiver, authenticated as the
  transaction sender or with the `StorageNodeCap`. mcli finds the nodes by reading every pool (about 140)
  and comparing the receiver with your accounts. Commission added at the start of an epoch stays locked
  until the epoch's voting ends; `status` shows that part separately.
- **IOTA** keeps JSON-RPC. The node's builder methods assemble the transaction, mcli checks and signs it.
  IOTA derives Ed25519 addresses from the bare public key (Sui hashes the scheme flag too).

## Migrating from 1.x (Go)

- `send --amount` now takes whole coins; 1.x expected MIST/NANOS.
- `send` executes the transfer; 1.x only printed a command to run.
- `~/.mcli-config/config.toml` is no longer read; addresses come from the keys in `.env`.
- The Go sources under `cmd/` still build with `make go-build` until they are removed.

## Development

```
make test       # unit tests
make lint       # rustfmt and clippy
```

Pushing to `main` builds Linux and macOS binaries and publishes a release using the version in `releases`.
