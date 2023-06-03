package cmd

import (
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

	payload := `{
	    "jsonrpc": "2.0",
	    "id": "1",
	    "method": "suix_getCoins",
	    "params": {
	        "owner": "` + config.Default.Address + `"
	    },
	    "coin_type": "0x2::sui::SUI"
	}`

	result, err := sendRequest(url, payload)
	if err != nil {
		log.Fatal(err)
	}

	fmt.Println(result)
}
