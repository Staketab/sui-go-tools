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
	Short: "SUI tool to merge-coins, withdraw stakes and others.",
	Long:  "SUI tool to merge-coins, withdraw stakes and others.",
	Run: func(cmd *cobra.Command, args []string) {
		// Default behavior when the root command is executed
		infoLog.Println("Welcome to:", binary)
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
	config, err := ReadConfigFile(configFilePath)
	if err != nil {
		errorLog.Println(err)
		return
	}

	infoLog.Println("RPC:", config.Default.Rpc)
	infoLog.Println("SUI binary path:", config.Default.SuiBinaryPath)
	infoLog.Println("Address:", config.Default.Address)
	infoLog.Println("Gas object to pay:", config.Default.GasObjToPay)
	infoLog.Println("Primary coin:", config.Default.PrimaryCoin)
}
func mergeCoin(slice []string, primaryobj string) {
	config, err := ReadConfigFile(configFilePath)
	if err != nil {
		fmt.Println(err)
		return
	}
	for _, value := range slice {
		if value != "" {
			primaryCoin := primaryobj
			coinToMerge := value
			gasBudget := config.Default.GasBudget

			infoLog.Println("RPC:", config.Default.Rpc)
			infoLog.Println("SUI binary path:", config.Default.SuiBinaryPath)
			infoLog.Println("Primary coin:", primaryCoin)
			infoLog.Println("Array value to merge:", coinToMerge)
			infoLog.Println("Gas Budget:", gasBudget)

			cmd := exec.Command(config.Default.SuiBinaryPath, "client", "merge-coin",
				"--primary-coin", primaryCoin,
				"--coin-to-merge", coinToMerge,
				"--gas-budget="+gasBudget,
				"--json")

			cmd.Stdout = nil

			outputFile := "output.txt"
			file, err := os.Create(configPath + outputFile)
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

			outputBytes, err := os.ReadFile(configPath + outputFile)
			if err != nil {
				log.Fatal(err)
			}

			var result MergeResponse
			err = json.Unmarshal(outputBytes, &result)
			if err != nil {
				log.Fatal(err)
			}
			infoLog.Println("--------------------")
			infoLog.Println("TX Status:", result.Effects.Status.Status)
			infoLog.Println("--------------------")
		} else {
			infoLog.Println("Coin ID merged.")
		}
	}
}

func mergeCoins(slice []string, gas, primaryobj string) {
	usr, err := user.Current()
	if err != nil {
		fmt.Errorf("failed to get current user: %s", err)
	}
	filePath := filepath.Join(usr.HomeDir, configFilePath)
	config, err := ReadConfigFile(filePath)
	if err != nil {
		fmt.Println(err)
		return
	}
	for _, value := range slice[1:] {
		if value != "" {
			primaryCoin := primaryobj
			coinToMerge := value
			gasBudget := gas

			infoLog.Println("RPC:", config.Default.Rpc)
			infoLog.Println("SUI binary path:", config.Default.SuiBinaryPath)
			infoLog.Println("Primary coin:", primaryCoin)
			infoLog.Println("Array value to merge:", coinToMerge)
			infoLog.Println("Gas Budget:", gasBudget)

			cmd := exec.Command(config.Default.SuiBinaryPath, "client", "merge-coin",
				"--primary-coin", primaryCoin,
				"--coin-to-merge", coinToMerge,
				"--gas-budget="+gasBudget,
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
				log.Fatal("Run failed", err)
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
			infoLog.Println("--------------------")
			infoLog.Println("TX Status:", result.Effects.Status.Status)
			infoLog.Println("--------------------")
		} else {
			infoLog.Println("All coins merged.")
		}
	}
}

func withdrawStakes(slice []string, gas, primaryobj string) {
	usr, err := user.Current()
	if err != nil {
		fmt.Errorf("failed to get current user: %s", err)
	}
	filePath := filepath.Join(usr.HomeDir, configFilePath)
	config, err := ReadConfigFile(filePath)
	if err != nil {
		fmt.Println(err)
		return
	}
	for _, value := range slice[0:] {
		if value != "" {
			infoLog.Println("RPC:", config.Default.Rpc)
			infoLog.Println("SUI binary path:", config.Default.SuiBinaryPath)
			infoLog.Println("Primary coin:", primaryobj)
			infoLog.Println("Array value to withdraw:", value)
			infoLog.Println("Gas Budget:", gas)
			infoLog.Println("Gas odject to pay:", primaryobj)

			gasBudget := gas
			stakesId := value

			cmd := exec.Command(config.Default.SuiBinaryPath, "client", "call",
				"--package", config.Default.Package,
				"--module", config.Default.Module,
				"--function", config.Default.Function,
				"--args", config.Default.Args,
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
			infoLog.Println("--------------------")
			infoLog.Println("TX Status:", result.Effects.Status.Status)
			infoLog.Println("--------------------")
		} else {
			infoLog.Println("Successful withdraw all sui::SuiStaked objects.")
		}
	}
}

//func sendSui(slice []string, recepient, amount string) {
//	usr, err := user.Current()
//	if err != nil {
//		fmt.Errorf("failed to get current user: %s", err)
//	}
//	filePath := filepath.Join(usr.HomeDir, configFilePath)
//	config, err := ReadConfigFile(filePath)
//	if err != nil {
//		fmt.Println(err)
//		return
//	}
//	for _, value := range slice[0:] {
//		infoLog.Println("RPC:", config.Default.Rpc)
//		infoLog.Println("SUI binary path:", config.Default.SuiBinaryPath)
//		infoLog.Println("Primary coin:", primaryobj)
//		infoLog.Println("Array value to withdraw:", value)
//		infoLog.Println("Gas Budget:", gas)
//		infoLog.Println("Gas odject to pay:", primaryobj)
//
//		gasBudget := gas
//		inputCoins := value
//
//		cmd := exec.Command(config.Default.SuiBinaryPath, "client", "pay-sui",
//			"--recipients", recepient,
//			"--amounts", amount,
//			"--input-coins", inputCoins,
//			"--gas-budget="+gasBudget,
//			"--json")
//
//		cmd.Stdout = os.Stdout
//		cmd.Stderr = os.Stderr
//
//		runs := cmd.Run()
//		if runs != nil {
//			log.Fatal(runs)
//		}
//	}
//}
