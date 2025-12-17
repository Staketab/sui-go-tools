package cmd

import (
	"encoding/json"
	"fmt"
	"time"
)

func getMergeAll() {
	isRpcWorking()

	chainConfig, err := GetActiveChainConfig()
	if err != nil {
		errorLog.Println(err)
		return
	}

	url := chainConfig.Rpc
	addr := chainConfig.Address

	// Determine coin type and RPC method based on chain
	coinType := "0x2::sui::SUI"
	rpcMethod := "suix_getCoins"
	if activeChain == ChainIOTA {
		coinType = "0x2::iota::IOTA"
		rpcMethod = "iotax_getCoins"
	}

	payload := fmt.Sprintf(`{
	    "jsonrpc": "2.0",
	    "id": "1",
	    "method": "%s",
	    "params": {
	        "owner": "%s"
	    },
	    "coin_type": "%s"
	}`, rpcMethod, addr, coinType)

	jsonStr, err := sendRequest(url, payload)
	if err != nil {
		errorLog.Fatal(err)
	}

	var result Result
	err2 := json.Unmarshal([]byte(jsonStr), &result)
	if err2 != nil {
		errorLog.Fatal(err2)
	}
	var coinObjectIds []string
	for _, data := range result.Result.Data {
		coinObjectIds = append(coinObjectIds, data.CoinObjectId)
	}
	infoLog.Printf("[%s] Coin Object IDs array: %v", GetChainName(), coinObjectIds)

	if len(coinObjectIds) != 1 {
		a := coinObjectIds
		b := chainConfig.GasBudget
		c := coinObjectIds[0]

		time.Sleep(2 * time.Second)
		mergeCoins(a, b, c)
	} else {
		infoLog.Printf("[%s] No coins objects found for merge.", GetChainName())
	}
}
