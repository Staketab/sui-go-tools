package cmd

import (
	"bytes"
	"fmt"
	"io/ioutil"
	"log"
	"net/http"
	"os"
	"os/user"
	"path/filepath"
	"strings"

	toml "github.com/pelletier/go-toml"
)

func isRpcWorking() {
	config, err := ReadConfigFile(configFilePath)
	if err != nil {
		fmt.Println(err)
		return
	}
	url := config.Default.Rpc

	payload := []byte(`{
		"jsonrpc": "2.0",
		"id": "1",
		"method": "sui_getChainIdentifier",
		"params": []
	}`)

	request, err := http.NewRequest("POST", url, bytes.NewBuffer(payload))
	if err != nil {
		log.Fatal(err)
	}

	request.Header.Set("Content-Type", "application/json")

	response, err := http.DefaultClient.Do(request)
	if err != nil {
		log.Fatal(err)
	}
	defer response.Body.Close()

	_, err = ioutil.ReadAll(response.Body)
	if err != nil {
		log.Fatal(err)
	}

	statusCode := response.StatusCode

	if statusCode == 200 {
		fmt.Println("RPC is working:", statusCode)
	} else {
		fmt.Println("RPC is not working:", statusCode)
		log.Fatal("Exit...", statusCode)
	}
}

func createDirectory(path string) error {
	usr, err := user.Current()
	if err != nil {
		return fmt.Errorf("failed to get current user: %s", err)
	}

	fullPath := filepath.Join(usr.HomeDir, path)

	err = os.MkdirAll(fullPath, os.ModePerm)
	if err != nil {
		return fmt.Errorf("failed to create directory: %s", err)
	}

	fmt.Printf("Directory created: %s\n", fullPath)
	return nil
}

func createConfigFile(path string) error {
	usr, err := user.Current()
	if err != nil {
		return fmt.Errorf("failed to get current user: %s", err)
	}
	filePath := filepath.Join(usr.HomeDir, path)
	content := []byte(`[DEFAULT]
# SUI Fullnode RPC
rpc = "https://rpc-testnet.suiscan.xyz:443"

# SUI binary PATH
sui_binary_path = "/root/sui/target/debug/sui"

# SUI address
address = "0x5427ad0ec547c505f5ec466a1a31e21c6b0ea07689ee8a4ee0afd94512da3c10"

# ID of the gas object for gas payment, in 20 bytes Hex string If not provided, a gas object with at least gas_budget value will be selected
gas_odject_to_pay = "0x4ee24f7c513ae2cabcbeaeb14f4b332047990066b89def6460d8ae047e3a4cbe"

# Coin to merge into, in 20 bytes Hex string
primary_coin = "0x4ee24f7c513ae2cabcbeaeb14f4b332047990066b89def6460d8ae047e3a4cbe"

# Array of Coins to be merged, in 20 bytes Hex string [0x4ee24f7c513ae2cabcbeaeb14f4b332047990066b89def6460d8ae047e3a4cbe 0x5dc3340bd32407e78e5e8fd98766e8bfd63e4981ac42f94789d2075cf1c72330]
coins_to_merge = []

# Gas budget for this call
gas_budget = "35000000"
`)

	err2 := ioutil.WriteFile(filePath, content, 0644)
	if err2 != nil {
		return fmt.Errorf("failed to create config file: %s", err)
	}

	fmt.Printf("Config file created: %s\n", filePath)
	return nil
}

func ReadConfigFile(path string) (Config, error) {
	config := Config{}
	tomlFile, err := toml.LoadFile(path)
	if err != nil {
		return config, fmt.Errorf("failed to load config file: %s", err)
	}

	err = tomlFile.Unmarshal(&config)
	if err != nil {
		return config, fmt.Errorf("failed to unmarshal config: %s", err)
	}

	return config, nil
}

func sendRequest(url string, payload string) (string, error) {
	request, err := http.NewRequest("POST", url, strings.NewReader(payload))
	if err != nil {
		return "", err
	}

	request.Header.Add("Content-Type", "application/json")

	response, err := http.DefaultClient.Do(request)
	if err != nil {
		return "", err
	}
	defer response.Body.Close()

	body, err := ioutil.ReadAll(response.Body)
	if err != nil {
		return "", err
	}

	return string(body), nil
}
