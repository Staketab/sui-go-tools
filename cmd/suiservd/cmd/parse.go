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
	// if url != "" {
	// 	url = config.Default.Rpc
	// } else {
	// 	getCoins()
	// }
	payload := `{
	    "jsonrpc": "2.0",
	    "id": "1",
	    "method": "suix_getCoins",
	    "params": {
	        "owner": "` + config.Default.Address + `"
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

	coinObjectIdVar := result.Result.Data[0].CoinObjectId

	fmt.Println("Coin Object ID is:", coinObjectIdVar)
}
