package cmd

import (
	"encoding/json"
	"fmt"
	"os/user"
	"path/filepath"
	"time"
)

func getPayObj() {
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
		//infoLog.Printf("Found Coin Object:", data)
		coinObjectIds = append(coinObjectIds, data.CoinObjectId)
	}

	if len(coinObjectIds) == 0 {
		errorLog.Println("No coin objects found for withdrawal.")
		return
	}

	a := coinObjectIds[0]
	infoLog.Println("Coin Object ID:", a)
	getWithdrawData(a)
}

func getWithdrawData(obj string) error {
	usr, err := user.Current()
	if err != nil {
		fmt.Errorf("failed to get current user: %s", err)
	}
	filePath := filepath.Join(usr.HomeDir, configFilePath)
	config, err := ReadConfigFile(filePath)
	if err != nil {
		errorLog.Println(err)
		return nil
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
	var pendingCount, nonPendingCount int

	for _, result := range response.Result {
		for _, stake := range result.Stakes {
			if stake.Status == "Pending" {
				pendingCount++
			} else {
				stakedSuiIds = append(stakedSuiIds, stake.StakedSuiID)
				nonPendingCount++
			}
		}
	}

	if nonPendingCount != 0 {
		infoLog.Println("Found", nonPendingCount, "non-pending Staked object IDs:", stakedSuiIds)
		infoLog.Println("Pending staked object IDs count:", pendingCount)
		a := stakedSuiIds
		b := config.Default.GasBudget
		c := obj

		time.Sleep(2 * time.Second)
		withdrawStakes(a, b, c)
	} else {
		infoLog.Println("No non-pending Staked object IDs found for withdrawal.")
		infoLog.Println("Pending staked object IDs count:", pendingCount)
	}
	return nil
}
