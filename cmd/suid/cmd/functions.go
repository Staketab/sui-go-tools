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

// GetActiveChainConfig returns the configuration for the currently active chain
func GetActiveChainConfig() (ChainConfig, error) {
	usr, err := user.Current()
	if err != nil {
		return ChainConfig{}, fmt.Errorf("failed to get current user: %s", err)
	}
	filePath := filepath.Join(usr.HomeDir, configFilePath)
	config, err := ReadConfigFile(filePath)
	if err != nil {
		return ChainConfig{}, err
	}

	switch activeChain {
	case ChainSUI:
		return config.SUI, nil
	case ChainIOTA:
		return config.IOTA, nil
	default:
		return ChainConfig{}, fmt.Errorf("unknown chain: %s", activeChain)
	}
}

// GetChainName returns the display name for the active chain
func GetChainName() string {
	switch activeChain {
	case ChainSUI:
		return "SUI"
	case ChainIOTA:
		return "IOTA"
	default:
		return strings.ToUpper(activeChain)
	}
}

func isRpcWorking() {
	infoLog.Printf("Start checking %s RPC status.", GetChainName())

	chainConfig, err := GetActiveChainConfig()
	if err != nil {
		errorLog.Println(err)
		return
	}

	url := chainConfig.Rpc

	// Determine RPC method based on chain
	method := "sui_getChainIdentifier"
	if activeChain == ChainIOTA {
		method = "iota_getChainIdentifier"
	}

	payload := []byte(fmt.Sprintf(`{
		"jsonrpc": "2.0",
		"id": "1",
		"method": "%s",
		"params": []
	}`, method))

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
		successLog.Printf("%s RPC is working (status: %d)", GetChainName(), statusCode)
		time.Sleep(2 * time.Second)
	} else {
		errorLog.Printf("%s RPC is not working (status: %d)", GetChainName(), statusCode)
		errorLog.Fatal("Exit...")
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

	successLog.Printf("Directory created: %s", fullPath)
	return nil
}

func createConfigFile(path string) error {
	usr, err := user.Current()
	if err != nil {
		return fmt.Errorf("failed to get current user: %s", err)
	}
	filePath := filepath.Join(usr.HomeDir, path)

	content := []byte(`# SUI blockchain configuration
[SUI]
rpc = "https://rpc-mainnet.suiscan.xyz:443"
binary_path = "/opt/homebrew/bin/sui"
address = ""
gas_budget = "20000000"
package = "0x3"
module = "sui_system"
function = "request_withdraw_stake"
args = "0x5"

# IOTA blockchain configuration
[IOTA]
rpc = "https://api.mainnet.iota.cafe"
binary_path = "/opt/homebrew/bin/iota"
address = ""
gas_budget = "20000000"
package = "0x3"
module = "iota_system"
function = "request_withdraw_stake"
args = "0x5"
`)

	err2 := os.WriteFile(filePath, content, 0644)
	if err2 != nil {
		return fmt.Errorf("failed to create config file: %s", err)
	}

	successLog.Printf("Config file created: %s", filePath)
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
