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
	CoinToMerge   string `toml:"coins_to_merge"`
	GasBudget     string `toml:"gas_budget"`
}

type Result struct {
	Result struct {
		Data []struct {
			CoinType            string `json:"coinType"`
			CoinObjectId        string `json:"coinObjectId"`
			Version             string `json:"version"`
			Digest              string `json:"digest"`
			Balance             string `json:"balance"`
			PreviousTransaction string `json:"previousTransaction"`
		} `json:"data"`
	} `json:"result"`
}
