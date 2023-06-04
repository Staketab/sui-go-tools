package cmd

import (
	"encoding/json"
	"fmt"
	"log"
)

func getCoins() {
	isRpcWorking()
	config, err := ReadConfigFile(configFilePath)
	if err != nil {
		fmt.Println(err)
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
		log.Fatal(err)
	}

	var result Result
	err2 := json.Unmarshal([]byte(jsonStr), &result)
	if err2 != nil {
		log.Fatal(err2)
	}
	var coinObjectIds []string
	for _, data := range result.Result.Data {
		coinObjectIds = append(coinObjectIds, data.CoinObjectId)
	}
	fmt.Println("Coin Object IDs array:", coinObjectIds)

	a := coinObjectIds
	b := coinObjectIds[0]
	c := config.Default.GasBudget
	d := coinObjectIds[0]

	mergeCoins(a, b, c, d)
}
