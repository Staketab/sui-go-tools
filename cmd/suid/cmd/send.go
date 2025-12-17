package cmd

import (
	"bufio"
	"encoding/json"
	"fmt"
	"net/url"
	"os"
	"os/exec"
	"os/user"
	"path/filepath"
	"strconv"
	"strings"
)

func getInputObj(recipient string, amount string) {
	isRpcWorking()

	chainConfig, err := GetActiveChainConfig()
	if err != nil {
		errorLog.Println(err)
		return
	}

	rpcUrl := chainConfig.Rpc
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

	jsonStr, err := sendRequest(rpcUrl, payload)
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
		errorLog.Printf("[%s] No coins found for sending", GetChainName())
		return
	}

	infoLog.Printf("[%s] Found coins: %v", GetChainName(), coinObjectIds)
	sendCoins(coinObjectIds, recipient, amount)
}

func sendCoins(coins []string, recipient string, amount string) {
	chainConfig, err := GetActiveChainConfig()
	if err != nil {
		errorLog.Println(err)
		return
	}

	infoLog.Printf("[%s] Sending %s to %s", GetChainName(), amount, recipient)
	infoLog.Printf("[%s] Using coins: %v", GetChainName(), coins)

	// Convert amount string to uint64
	amountInt, err := strconv.ParseUint(amount, 10, 64)
	if err != nil {
		errorLog.Printf("Failed to parse amount: %v", err)
		return
	}

	// Convert gas budget string to uint64
	gasBudgetInt, err := strconv.ParseUint(chainConfig.GasBudget, 10, 64)
	if err != nil {
		errorLog.Printf("Failed to parse gas budget: %v", err)
		return
	}

	// Add debug logging for the payload
	infoLog.Println("Debug - Coins:", coins)
	infoLog.Println("Debug - Amount:", amountInt)
	infoLog.Println("Debug - Gas Budget:", gasBudgetInt)

	// Create params structure
	type Params struct {
		JSONRPC string        `json:"jsonrpc"`
		ID      string        `json:"id"`
		Method  string        `json:"method"`
		Params  []interface{} `json:"params"`
	}

	// Determine RPC method based on chain
	payMethod := "unsafe_paySui"
	if activeChain == ChainIOTA {
		payMethod = "unsafe_payIota"
	}

	// Use unsafe_paySui/unsafe_payIota which is designed to use the same coin for payment and gas:
	// [signer, input_coins, recipients, amounts, gas_budget]
	params := Params{
		JSONRPC: "2.0",
		ID:      "1",
		Method:  payMethod,
		Params: []interface{}{
			chainConfig.Address,   // signer address
			[]string{coins[0]},    // input coins (using first coin)
			[]string{recipient},   // recipients addresses
			[]string{amount},      // amounts as strings
			chainConfig.GasBudget, // gas budget as string
		},
	}

	// Marshal the entire payload to JSON
	payloadBytes, err := json.Marshal(params)
	if err != nil {
		errorLog.Printf("Failed to marshal payload: %v", err)
		return
	}

	payload := string(payloadBytes)
	infoLog.Println("Debug - Full payload:", payload)
	infoLog.Println("Sending RPC request to:", chainConfig.Rpc)

	// Send RPC request
	jsonStr, err := sendRequest(chainConfig.Rpc, payload)
	if err != nil {
		errorLog.Printf("Failed to send RPC request: %v", err)
		return
	}

	// Define response structure for transaction bytes
	type TransactionBlockBytes struct {
		Gas          []interface{} `json:"gas"`
		InputObjects []interface{} `json:"inputObjects"`
		TxBytes      string        `json:"txBytes"`
	}

	type RPCResponseTxBytes struct {
		JSONRPC string                `json:"jsonrpc"`
		ID      string                `json:"id"`
		Result  TransactionBlockBytes `json:"result"`
		Error   struct {
			Code    int    `json:"code"`
			Message string `json:"message"`
		} `json:"error"`
	}

	// Parse the TransactionBlockBytes response
	var responseTxBytes RPCResponseTxBytes
	err = json.Unmarshal([]byte(jsonStr), &responseTxBytes)
	if err != nil {
		errorLog.Printf("Failed to parse response: %v", err)
		return
	}

	// Check for RPC error
	if responseTxBytes.Error.Message != "" {
		errorLog.Printf("RPC Error: %s", responseTxBytes.Error.Message)
		return
	}

	// If we got this far, we have transaction bytes that need to be signed and executed
	// Since we can't sign them programmatically without the private key,
	// we'll save them to a file for the user to sign manually

	// Get the user's home directory
	var currentUsr *user.User
	currentUsr, err = user.Current()
	if err != nil {
		errorLog.Printf("Failed to get user directory: %v", err)
		return
	}

	// Create a file to save the transaction bytes
	txBytesFile := filepath.Join(currentUsr.HomeDir, configPath, "transaction_to_sign.txt")

	// Clean the tx bytes string - remove any URL encoding or invalid characters
	txBytesStr := responseTxBytes.Result.TxBytes
	// Remove any trailing '%' which might be from incomplete URL encoding
	txBytesStr = strings.TrimSuffix(txBytesStr, "%")

	// Try to URL-decode if it's URL encoded
	decodedBytes, err := url.QueryUnescape(txBytesStr)
	if err == nil {
		// If decoding succeeded, use the decoded string
		txBytesStr = decodedBytes
	}

	err = os.WriteFile(txBytesFile, []byte(txBytesStr), 0644)
	if err != nil {
		errorLog.Printf("Failed to write transaction bytes to file: %v", err)
		return
	}

	infoLog.Println("--------------------")
	infoLog.Println("Transaction bytes generated successfully!")
	infoLog.Printf("[%s] Transaction details: Send %s to %s", GetChainName(), amount, recipient)
	infoLog.Printf("Transaction bytes saved to: %s", txBytesFile)

	// Ask for confirmation to execute the transaction
	reader := bufio.NewReader(os.Stdin)
	fmt.Print("Do you want to execute this transaction now? (y/n): ")
	confirmation, _ := reader.ReadString('\n')
	confirmation = strings.TrimSpace(strings.ToLower(confirmation))

	if confirmation == "y" || confirmation == "yes" {
		infoLog.Printf("Checking if %s CLI is available...", GetChainName())

		// First check if CLI is available with a simple command
		checkCmd := exec.Command(chainConfig.BinaryPath, "--version")
		var checkStdout, checkStderr strings.Builder
		checkCmd.Stdout = &checkStdout
		checkCmd.Stderr = &checkStderr

		err = checkCmd.Run()
		if err != nil {
			errorLog.Printf("Error: %s CLI is not available: %v", GetChainName(), err)
			errorLog.Printf("Error details: %s", checkStderr.String())
			infoLog.Printf("Please make sure the %s CLI is installed at: %s", GetChainName(), chainConfig.BinaryPath)
			return
		}

		infoLog.Printf("%s CLI is available: %s", GetChainName(), strings.TrimSpace(checkStdout.String()))

		// Make sure we have at least one coin
		if len(coins) == 0 {
			errorLog.Println("No coins available for transfer")
			return
		}

		// Use the first coin as the source coin for the transfer
		coinObjectID := coins[0]

		// Generate command for user to execute manually
		cmdString := fmt.Sprintf("%s client pay-sui --gas-budget 3000000 --recipients %s --amounts %s --input-coins %s",
			chainConfig.BinaryPath, recipient, amount, coinObjectID)

		// Create a script file with the command
		scriptFile := filepath.Join(currentUsr.HomeDir, configPath, "execute_transfer.sh")
		scriptContent := "#!/bin/sh\n" + cmdString + "\n"
		err = os.WriteFile(scriptFile, []byte(scriptContent), 0755)
		if err != nil {
			errorLog.Printf("Failed to create script file: %v", err)
			return
		}

		infoLog.Println("--------------------")
		infoLog.Println("Transaction prepared successfully!")
		infoLog.Printf("[%s] To execute the transaction to send %s to %s:", GetChainName(), amount, recipient)
		infoLog.Printf("1. A script has been created at: %s", scriptFile)
		infoLog.Printf("2. You can run it directly with: sh %s", scriptFile)
		infoLog.Printf("3. Or you can copy and paste this command into your terminal:")
		infoLog.Printf("   %s", cmdString)
		infoLog.Println("--------------------")

	} else {
		// Make sure we have at least one coin for the example
		coinObjectID := "<COIN_OBJECT_ID>"
		if len(coins) > 0 {
			coinObjectID = coins[0]
		}

		infoLog.Println("Transaction not executed. You can manually execute it later using:")
		infoLog.Printf("%s client pay-sui --gas-budget 3000000 --recipients %s --amounts %s --input-coins %s",
			chainConfig.BinaryPath, recipient, amount, coinObjectID)
	}

	infoLog.Println("--------------------")
}
