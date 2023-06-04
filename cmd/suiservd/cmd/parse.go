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

	fmt.Println(jsonStr)

	var result Result
	err2 := json.Unmarshal([]byte(jsonStr), &result)
	if err2 != nil {
		log.Fatal(err2)
	}
	var coinObjectIds []string
	for _, data := range result.Result.Data {
		coinObjectIds = append(coinObjectIds, data.CoinObjectId)
	}

	coinObjectIdVar := coinObjectIds[0]
	coinObjectIdVar2 := coinObjectIds[1]

	fmt.Println("Coin Object IDs array:", coinObjectIds)
	fmt.Println("Coin Object ID 1 is:", coinObjectIdVar)
	fmt.Println("Coin Object ID 2 is:", coinObjectIdVar2)
}
