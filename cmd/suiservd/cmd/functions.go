package cmd

import (
	"fmt"
	"io/ioutil"
	"os"
	"os/exec"
	"os/user"
	"path/filepath"
	"strings"

	toml "github.com/pelletier/go-toml"
)

func isRpcWorking() bool {
	infoLog.Println("Start checking node status")
	cmd := exec.Command("/bin/bash", "-c", "docker exec -i mina mina client status --json | jq -r .sync_status")
	out, err := cmd.Output()
	if err != nil {
		errorLog.Println("Failed to get node status:", err)
		return false
	}

	status := strings.TrimSpace(string(out))
	if status != "Synced" {
		errorLog.Println("Node is not synced, status:", status)
		return false
	}

	infoLog.Println("Node is synced, status:", status)
	return true
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
rpc = "https://rpc-mainnet.suiscan.xyz:443"
sui_binary_path = "/home/sui/sui/target/debug/sui"
address = ""
gas_odject_to_pay = ""
primary_coin = ""
`)

	err2 := ioutil.WriteFile(filePath, content, 0644)
	if err2 != nil {
		return fmt.Errorf("failed to create config file: %s", err)
	}

	fmt.Printf("Config file created: %s\n", filePath)
	return nil
}

func ReadConfigFile(path string) (DatabaseConfig, error) {
	config := DatabaseConfig{}
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
