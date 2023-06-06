# SUI TOOL
Sui tool to easily Merge and Withdraw object IDs.

First Release includes:
- Easy Merge sui::SUI objects to Primary coin passing one or an array of objects as a flag
- Merging all sui::SUI objects to Primary coin automatically
- Withdrawing all sui::SuiStaked objects automatically

## Clone the repository:
```
git clone https://github.com/Staketab/sui-go-tools.git
```

## Build the binary:
```
cd sui-go-tools
make install
```

## USAGE:
```
SUI tool to merge-coins, withdraw stakes and others.

Usage:
  suid [flags]
  suid [command]

Available Commands:
  help         Help about any command
  init         Initialize config
  merge        Merge sui::SUI objects to PRIMARY_COIN
  merge-all    Merging all sui::SUI objects to PRIMARY_COIN
  version      Print the CLI version
  withdraw-all Withdrawing all sui::SuiStaked objects

Flags:
  -h, --help   help for suid
```

## 1. Init config:
This command will create a config along the path `/home/USER/.sui-config/config.toml`
```
suid init
```
Example:
```
[DEFAULT]
rpc = "https://rpc-mainnet.suiscan.xyz:443"
sui_binary_path = "/home/USER/target/debug/sui"
address = ""
gas_budget = "20000000"
package = "0x3"
module = "sui_system"
function = "request_withdraw_stake"
args = "0x5"
```
Specify the in the config:
- SUI endpoint RPC `rpc = "https://rpc-mainnet.suiscan.xyz:443"`
- SUI binary PATH `sui_binary_path = "/home/USER/target/debug/sui"`
- Your address `address = ""`

## 2. Merge:
```
Merge sui::SUI objects to PRIMARY_COIN

Usage:
  merge [flags]

Flags:
  -p, --primary-coin string   The primary coin for merging, in 20 bytes Hex string
  -c, --coins-to-merge string, array   Coins to be merged, in 20 bytes Hex string
```
Use one sui::SUI object in `--coins-to-merge` or several separated by commas.
```
suid merge --primary-coin 0x0743d13c6b0cc38aa6f2241b9c33f173200a5166be2df3eaf636775cd39bec33 --coins-to-merge 0x08fe065cc37c0394fe8d4320389b0016c2b903888af656b1e998d68a8dec56eb,0x0c466a8adc360dd4c596cfc190d59f02354b295c54048d75374e38dff7810f64
```

## 3. Merge All:
Merging all sui::SUI objects to Primary coin automatically.
```
suid merge-all
```
## 4. Withdraw All:
Withdrawing all sui::SuiStaked objects automatically.
```
suid withdraw-all
```

More soon.