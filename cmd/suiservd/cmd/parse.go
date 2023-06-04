package cmd

import (
	"encoding/json"
)

func getMergeData() {
	isRpcWorking()
	config, err := ReadConfigFile(configFilePath)
	if err != nil {
		errorLog.Println(err)
		return
	}
	url := config.Default.Rpc
	addr := config.Default.Address
	payload := `{
	    "jsonrpc": "2.0",
	    "id": "1",
	    "method": "suix_getCoins",
	    "params": {
	        "owner": "` + addr + `"
	    },
	    "coin_type": "0x2::sui::SUI"
	}`

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
	infoLog.Println("Coin Object IDs array:", coinObjectIds)
	if len(coinObjectIds) != 1 {
		a := coinObjectIds
		b := config.Default.GasBudget
		c := coinObjectIds[0]

		mergeCoins(a, b, c)
	} else {
		infoLog.Println("All coins merged.")
	}
	// if config.Default.Address == "" {

	// } else {

	// }
}

func getPayObj() {
	config, err := ReadConfigFile(configFilePath)
	if err != nil {
		errorLog.Println(err)
		return
	}
	url := config.Default.Rpc
	addr := config.Default.Address
	payload := `{
	    "jsonrpc": "2.0",
	    "id": "1",
	    "method": "suix_getCoins",
	    "params": {
	        "owner": "` + addr + `"
	    },
	    "coin_type": "0x2::sui::SUI"
	}`

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
	infoLog.Println("Coin Object IDs array:", coinObjectIds)

	// if config.Default.Address == "" {

	// } else {

	// }
	a := coinObjectIds[0]

	getWithdrawData(a)
}

func getWithdrawData(obj string) {
	isRpcWorking()
	config, err := ReadConfigFile(configFilePath)
	if err != nil {
		errorLog.Println(err)
		return
	}
	url := config.Default.Rpc
	addr := config.Default.Address
	payload := `{
	    "jsonrpc": "2.0",
	    "id": "1",
	    "method": "suix_getStakes",
	    "params": {
			"owner": "` + addr + `"
		},
		"controller": {}
	}`

	jsonStr, err := sendRequest(url, payload)
	if err != nil {
		errorLog.Fatal(err)
	}

	var response Response
	err2 := json.Unmarshal([]byte(jsonStr), &response)
	if err2 != nil {
		errorLog.Fatal(err2)
	}
	var stakedSuiIds []string

	for _, result := range response.Result {
		for _, stake := range result.Stakes {
			stakedSuiIds = append(stakedSuiIds, stake.StakedSuiID)
		}
	}
	infoLog.Println("Staked Object IDs array:", stakedSuiIds)

	a := stakedSuiIds
	b := config.Default.GasBudget
	c := obj

	withdrawStakes(a, b, c)
}
