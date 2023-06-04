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
	Package       string `toml:"package"`
	Module        string `toml:"module"`
	Function      string `toml:"function"`
	Args          string `toml:"args"`
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

type Response struct {
	JSONRPC string `json:"jsonrpc"`
	Result  []struct {
		ValidatorAddress string `json:"validatorAddress"`
		StakingPool      string `json:"stakingPool"`
		Stakes           []struct {
			StakedSuiID       string `json:"stakedSuiId"`
			StakeRequestEpoch string `json:"stakeRequestEpoch"`
			StakeActiveEpoch  string `json:"stakeActiveEpoch"`
			Principal         string `json:"principal"`
			Status            string `json:"status"`
			EstimatedReward   string `json:"estimatedReward"`
		} `json:"stakes"`
	} `json:"result"`
	ID string `json:"id"`
}
