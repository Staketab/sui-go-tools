package cmd

// ChainConfig holds configuration for a specific blockchain network
type ChainConfig struct {
	Rpc        string `toml:"rpc"`
	BinaryPath string `toml:"binary_path"`
	Address    string `toml:"address"`
	GasBudget  string `toml:"gas_budget"`
	Package    string `toml:"package"`
	Module     string `toml:"module"`
	Function   string `toml:"function"`
	Args       string `toml:"args"`
}

// Config holds configuration for all supported chains
type Config struct {
	SUI  ChainConfig `toml:"SUI"`
	IOTA ChainConfig `toml:"IOTA"`
}

type Result struct {
	Result struct {
		Data []struct {
			CoinObjectId string `json:"coinObjectId"`
		} `json:"data"`
	} `json:"result"`
}

type Response struct {
	Result []struct {
		Stakes []struct {
			StakedSuiID  string `json:"stakedSuiId"`
			StakedIotaID string `json:"stakedIotaId"`
			Status       string `json:"status"`
		} `json:"stakes"`
	} `json:"result"`
	ID string `json:"id"`
}

type MergeResponse struct {
	Digest      string `json:"digest"`
	Transaction struct {
		Data struct {
			MessageVersion string `json:"messageVersion"`
			Transaction    struct {
				Kind   string `json:"kind"`
				Inputs []struct {
					Type       string `json:"type"`
					ObjectType string `json:"objectType"`
					ObjectID   string `json:"objectId"`
					Version    string `json:"version"`
					Digest     string `json:"digest"`
				} `json:"inputs"`
				Transactions []struct {
					MoveCall struct {
						Package       string   `json:"package"`
						Module        string   `json:"module"`
						Function      string   `json:"function"`
						TypeArguments []string `json:"type_arguments"`
						Arguments     []struct {
							Input int `json:"Input"`
						} `json:"arguments"`
					} `json:"MoveCall"`
				} `json:"transactions"`
			} `json:"transaction"`
			Sender  string `json:"sender"`
			GasData struct {
				Payment []struct {
					ObjectID string `json:"objectId"`
					Version  int    `json:"version"`
					Digest   string `json:"digest"`
				} `json:"payment"`
				Owner  string `json:"owner"`
				Price  string `json:"price"`
				Budget string `json:"budget"`
			} `json:"gasData"`
		} `json:"data"`
		TxSignatures []string `json:"txSignatures"`
	} `json:"transaction"`
	Effects struct {
		MessageVersion string `json:"messageVersion"`
		Status         struct {
			Status string `json:"status"`
		} `json:"status"`
		ExecutedEpoch string `json:"executedEpoch"`
		GasUsed       struct {
			ComputationCost         string `json:"computationCost"`
			StorageCost             string `json:"storageCost"`
			StorageRebate           string `json:"storageRebate"`
			NonRefundableStorageFee string `json:"nonRefundableStorageFee"`
		} `json:"gasUsed"`
		ModifiedAtVersions []struct {
			ObjectID       string `json:"objectId"`
			SequenceNumber string `json:"sequenceNumber"`
		} `json:"modifiedAtVersions"`
		TransactionDigest string `json:"transactionDigest"`
		Mutated           []struct {
			Owner struct {
				AddressOwner string `json:"AddressOwner"`
			} `json:"owner"`
			Reference struct {
				ObjectID string `json:"objectId"`
				Version  int    `json:"version"`
				Digest   string `json:"digest"`
			} `json:"reference"`
		} `json:"mutated"`
		Deleted []struct {
			ObjectID string `json:"objectId"`
			Version  int    `json:"version"`
			Digest   string `json:"digest"`
		} `json:"deleted"`
		GasObject struct {
			Owner struct {
				AddressOwner string `json:"AddressOwner"`
			} `json:"owner"`
			Reference struct {
				ObjectID string `json:"objectId"`
				Version  int    `json:"version"`
				Digest   string `json:"digest"`
			} `json:"reference"`
		} `json:"gasObject"`
		Dependencies []string `json:"dependencies"`
	} `json:"effects"`
	Events        []interface{} `json:"events"`
	ObjectChanges []struct {
		Type   string `json:"type"`
		Sender string `json:"sender"`
		Owner  struct {
			AddressOwner string `json:"AddressOwner"`
		} `json:"owner"`
		ObjectType      string `json:"objectType"`
		ObjectID        string `json:"objectId"`
		Version         string `json:"version"`
		PreviousVersion string `json:"previousVersion"`
		Digest          string `json:"digest"`
	} `json:"objectChanges"`
	BalanceChanges []struct {
		Owner struct {
			AddressOwner string `json:"AddressOwner"`
		} `json:"owner"`
		CoinType string `json:"coinType"`
		Amount   string `json:"amount"`
	} `json:"balanceChanges"`
	ConfirmedLocalExecution bool `json:"confirmedLocalExecution"`
}
