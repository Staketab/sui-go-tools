package cmd

import (
	"encoding/json"
	"fmt"
	"os/user"
	"path/filepath"
	"time"
)

func getMergeAll() {
	isRpcWorking()
	usr, err := user.Current()
	if err != nil {
		fmt.Errorf("failed to get current user: %s", err)
	}
	filePath := filepath.Join(usr.HomeDir, configFilePath)
	config, err := ReadConfigFile(filePath)
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

		time.Sleep(2 * time.Second)
		mergeCoins(a, b, c)
	} else {
		infoLog.Println("No coins objects found for merge.")
	}
}
