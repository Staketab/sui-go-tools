package cmd

type DatabaseConfig struct {
	Rpc           string `toml:"rpc"`
	SuiBinaryPath int    `toml:"sui_binary_path"`
	Address       string `toml:"address"`
	GasObjToPay   string `toml:"gas_odject_to_pay"`
	PrimaryCoin   string `toml:"primary_coin"`
}
