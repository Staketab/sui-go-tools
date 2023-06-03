package cmd

type Config struct {
	Default DefaultConfig `toml:"DEFAULT"`
}

type DefaultConfig struct {
	Rpc           string `toml:"rpc"`
	SuiBinaryPath string `toml:"sui_binary_path"`
	Address       string `toml:"address"`
	GasObjToPay   string `toml:"gas_odject_to_pay"`
	PrimaryCoin   string `toml:"primary_coin"`
}
