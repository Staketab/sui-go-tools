package cmd

import (
	"bytes"
	"fmt"
	"io"
	"io/ioutil"
	"net/http"
	"os"
	"os/user"
	"path/filepath"
	"strings"
	"time"

	toml "github.com/pelletier/go-toml"
)

func isRpcWorking() {
	infoLog.Println("Start checking RPC status.")
	config, err := ReadConfigFile(configFilePath)
	if err != nil {
		errorLog.Println(err)
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
		errorLog.Fatal(err)
	}

	request.Header.Set("Content-Type", "application/json")

	response, err := http.DefaultClient.Do(request)
	if err != nil {
		errorLog.Fatal(err)
	}
	defer response.Body.Close()

	_, err = io.ReadAll(response.Body)
	if err != nil {
		errorLog.Fatal(err)
	}

	statusCode := response.StatusCode

	if statusCode == 200 {
		infoLog.Println("RPC is working, status code:", statusCode)
		time.Sleep(2 * time.Second)
	} else {
		infoLog.Println("RPC is not working, status code:", statusCode)
		errorLog.Fatal("Exit...", statusCode)
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

	infoLog.Printf("Directory created: %s\n", fullPath)
	return nil
}

func createConfigFile(path string) error {
	usr, err := user.Current()
	if err != nil {
		return fmt.Errorf("failed to get current user: %s", err)
	}
	filePath := filepath.Join(usr.HomeDir, path)
	content := []byte(`[DEFAULT]
rpc = "https://rpc-mainnet.suiscan.xyz:443"
sui_binary_path = "/root/sui/target/debug/sui"
address = "0x5427ad0ec547c505f5ec466a1a31e21c6b0ea07689ee8a4ee0afd94512da3c10"
gas_budget = "20000000"
package = "0x3"
module = "sui_system"
function = "request_withdraw_stake"
args = "0x5"
`)

	err2 := os.WriteFile(filePath, content, 0644)
	if err2 != nil {
		return fmt.Errorf("failed to create config file: %s", err)
	}

	infoLog.Printf("Config file created: %s\n", filePath)
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
