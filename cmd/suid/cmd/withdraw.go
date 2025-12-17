package cmd

import (
	"encoding/json"
	"fmt"
	"time"
)

func getPayObj() {
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

	if len(coinObjectIds) == 0 {
		errorLog.Printf("[%s] No coin objects found for withdrawal.", GetChainName())
		return
	}

	a := coinObjectIds[0]
	infoLog.Printf("[%s] Coin Object ID: %s", GetChainName(), a)
	getWithdrawData(a)
}

func getWithdrawData(obj string) error {
	chainConfig, err := GetActiveChainConfig()
	if err != nil {
		errorLog.Println(err)
		return err
	}

	url := chainConfig.Rpc
	addr := chainConfig.Address

	// Determine RPC method based on chain
	rpcMethod := "suix_getStakes"
	if activeChain == ChainIOTA {
		rpcMethod = "iotax_getStakes"
	}

	payload := fmt.Sprintf(`{
	    "jsonrpc": "2.0",
	    "id": "1",
	    "method": "%s",
	    "params": {
			"owner": "%s"
		},
		"controller": {}
	}`, rpcMethod, addr)

	jsonStr, err := sendRequest(url, payload)
	if err != nil {
		errorLog.Fatal(err)
	}

	var response Response
	err2 := json.Unmarshal([]byte(jsonStr), &response)
	if err2 != nil {
		errorLog.Fatal(err2)
	}

	var stakedIds []string
	var pendingCount, nonPendingCount int

	for _, result := range response.Result {
		for _, stake := range result.Stakes {
			if stake.Status == "Pending" {
				pendingCount++
			} else {
				// Use the correct staked ID field based on the active chain
				stakedID := stake.StakedSuiID
				if activeChain == ChainIOTA {
					stakedID = stake.StakedIotaID
				}
				stakedIds = append(stakedIds, stakedID)
				nonPendingCount++
			}
		}
	}

	if nonPendingCount != 0 {
		infoLog.Printf("[%s] Found %d non-pending Staked object IDs: %v", GetChainName(), nonPendingCount, stakedIds)
		infoLog.Printf("[%s] Pending staked object IDs count: %d", GetChainName(), pendingCount)
		a := stakedIds
		b := chainConfig.GasBudget
		c := obj

		time.Sleep(2 * time.Second)
		withdrawStakes(a, b, c)
	} else {
		infoLog.Printf("[%s] No non-pending Staked object IDs found for withdrawal.", GetChainName())
		infoLog.Printf("[%s] Pending staked object IDs count: %d", GetChainName(), pendingCount)
	}
	return nil
}
