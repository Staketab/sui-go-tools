package cmd

import (
	"encoding/json"
	"fmt"
	"log"
	"os"
	"os/exec"
	"os/user"
	"path/filepath"

	"github.com/spf13/cobra"
)

var RootCmd = &cobra.Command{
	Use:   binary,
	Short: "Multi-chain CLI for SUI and IOTA blockchains",
	Long:  "MCLI - Multi-chain CLI to merge-coins, withdraw stakes and send transactions for SUI and IOTA blockchains.",
	Run: func(cmd *cobra.Command, args []string) {
		cmd.Help()
	},
}

func initConfig() {
	err := createDirectory(configPath)
	if err != nil {
		errorLog.Fatal(err)
	}
}

func initConfigFile() {
	err := createConfigFile(configFilePath)
	if err != nil {
		errorLog.Fatal(err)
	}
}

func readConfig() {
	usr, err := user.Current()
	if err != nil {
		errorLog.Println(err)
		return
	}
	filePath := filepath.Join(usr.HomeDir, configFilePath)
	config, err := ReadConfigFile(filePath)
	if err != nil {
		errorLog.Println(err)
		return
	}

	fmt.Printf("\n%s%s=== SUI Configuration ===%s\n", colorBold, colorBlue, colorReset)
	fmt.Printf("  RPC:         %s\n", config.SUI.Rpc)
	fmt.Printf("  Binary:      %s\n", config.SUI.BinaryPath)
	fmt.Printf("  Address:     %s\n", config.SUI.Address)
	fmt.Printf("  Gas budget:  %s\n", config.SUI.GasBudget)

	fmt.Printf("\n%s%s=== IOTA Configuration ===%s\n", colorBold, colorCyan, colorReset)
	fmt.Printf("  RPC:         %s\n", config.IOTA.Rpc)
	fmt.Printf("  Binary:      %s\n", config.IOTA.BinaryPath)
	fmt.Printf("  Address:     %s\n", config.IOTA.Address)
	fmt.Printf("  Gas budget:  %s\n\n", config.IOTA.GasBudget)
}

func mergeCoin(slice []string, primaryobj string) {
	chainConfig, err := GetActiveChainConfig()
	if err != nil {
		fmt.Println(err)
		return
	}

	for _, value := range slice {
		if value != "" {
			primaryCoin := primaryobj
			coinToMerge := value
			gasBudget := chainConfig.GasBudget

			infoLog.Printf("Chain: %s", GetChainName())
			infoLog.Println("RPC:", chainConfig.Rpc)
			infoLog.Println("Binary path:", chainConfig.BinaryPath)
			infoLog.Println("Primary coin:", primaryCoin)
			infoLog.Println("Array value to merge:", coinToMerge)
			infoLog.Println("Gas Budget:", gasBudget)

			cmd := exec.Command(chainConfig.BinaryPath, "client", "merge-coin",
				"--primary-coin", primaryCoin,
				"--coin-to-merge", coinToMerge,
				"--gas-budget="+gasBudget,
				"--json")

			cmd.Stdout = nil

			usr, _ := user.Current()
			outputFile := "output.txt"
			filePathOutput := filepath.Join(usr.HomeDir, configPath, outputFile)
			file, err := os.Create(filePathOutput)
			if err != nil {
				log.Fatal(err)
			}
			defer file.Close()

			cmd.Stdout = file
			cmd.Stderr = os.Stderr

			err = cmd.Run()
			if err != nil {
				log.Fatal(err)
			}

			outputBytes, err := os.ReadFile(filePathOutput)
			if err != nil {
				log.Fatal(err)
			}

			var result MergeResponse
			err = json.Unmarshal(outputBytes, &result)
			if err != nil {
				log.Fatal(err)
			}
			printTxStatus(result.Effects.Status.Status)
		} else {
			successLog.Println("Coin ID merged.")
		}
	}
}

func printTxStatus(status string) {
	fmt.Println("--------------------")
	if status == "success" {
		fmt.Printf("TX Status: %s%s%s\n", colorGreen, status, colorReset)
	} else {
		fmt.Printf("TX Status: %s%s%s\n", colorRed, status, colorReset)
	}
	fmt.Println("--------------------")
}

func mergeCoins(slice []string, gas, primaryobj string) {
	chainConfig, err := GetActiveChainConfig()
	if err != nil {
		fmt.Println(err)
		return
	}

	usr, err := user.Current()
	if err != nil {
		fmt.Errorf("failed to get current user: %s", err)
		return
	}

	for _, value := range slice[1:] {
		if value != "" {
			primaryCoin := primaryobj
			coinToMerge := value
			gasBudget := gas

			infoLog.Printf("Chain: %s", GetChainName())
			infoLog.Println("RPC:", chainConfig.Rpc)
			infoLog.Println("Binary path:", chainConfig.BinaryPath)
			infoLog.Println("Primary coin:", primaryCoin)
			infoLog.Println("Array value to merge:", coinToMerge)
			infoLog.Println("Gas Budget:", gasBudget)

			cmd := exec.Command(chainConfig.BinaryPath, "client", "merge-coin",
				"--primary-coin", primaryCoin,
				"--coin-to-merge", coinToMerge,
				"--gas-budget="+gasBudget,
				"--json")

			outputFile := "output.txt"
			filePathOutput := filepath.Join(usr.HomeDir, configPath, outputFile)
			file, err := os.Create(filePathOutput)
			if err != nil {
				log.Fatal(err)
			}
			defer file.Close()

			cmd.Stdout = file
			cmd.Stderr = os.Stderr

			err = cmd.Run()
			if err != nil {
				log.Fatal("Run failed - ", err)
			}

			outputBytes, err := os.ReadFile(filePathOutput)
			if err != nil {
				log.Fatal("Read bytes failed", err)
			}

			var result MergeResponse
			err = json.Unmarshal(outputBytes, &result)
			if err != nil {
				log.Fatal("Unmarshal error", err)
			}
			printTxStatus(result.Effects.Status.Status)
		} else {
			successLog.Println("All coins merged.")
		}
	}
}

func withdrawStakes(slice []string, gas, primaryobj string) {
	chainConfig, err := GetActiveChainConfig()
	if err != nil {
		fmt.Println(err)
		return
	}

	usr, err := user.Current()
	if err != nil {
		fmt.Errorf("failed to get current user: %s", err)
	}

	for _, value := range slice[0:] {
		if value != "" {
			infoLog.Printf("Chain: %s", GetChainName())
			infoLog.Println("RPC:", chainConfig.Rpc)
			infoLog.Println("Binary path:", chainConfig.BinaryPath)
			infoLog.Println("Primary coin:", primaryobj)
			infoLog.Println("Array value to withdraw:", value)
			infoLog.Println("Gas Budget:", gas)
			infoLog.Println("Gas object to pay:", primaryobj)

			gasBudget := gas
			stakesId := value

			cmd := exec.Command(chainConfig.BinaryPath, "client", "call",
				"--package", chainConfig.Package,
				"--module", chainConfig.Module,
				"--function", chainConfig.Function,
				"--args", chainConfig.Args,
				stakesId,
				"--gas-budget="+gasBudget,
				"--gas", primaryobj,
				"--json")

			cmd.Stdout = nil

			outputFile := "output.txt"
			filePathOutput := filepath.Join(usr.HomeDir, configPath, outputFile)
			file, err := os.Create(filePathOutput)
			if err != nil {
				log.Fatal(err)
			}
			defer file.Close()

			cmd.Stdout = file
			cmd.Stderr = os.Stderr

			err = cmd.Run()
			if err != nil {
				log.Fatal(err)
			}

			outputBytes, err := os.ReadFile(filePathOutput)
			if err != nil {
				log.Fatal(err)
			}

			var result MergeResponse
			err = json.Unmarshal(outputBytes, &result)
			if err != nil {
				log.Fatal(err)
			}
			printTxStatus(result.Effects.Status.Status)
		} else {
			successLog.Printf("Successful withdraw all %s staked objects.", GetChainName())
		}
	}
}
