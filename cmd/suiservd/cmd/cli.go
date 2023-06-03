package cmd

import (
	"fmt"
	"log"
	"os"
	"os/exec"

	"github.com/spf13/cobra"
)

var RootCmd = &cobra.Command{
	Use:   binary,
	Short: "SUI tool to merge-coins, withdraw stakes and others.",
	Long:  "SUI tool to merge-coins, withdraw stakes and others.",
	Run: func(cmd *cobra.Command, args []string) {
		// Default behavior when the root command is executed
		fmt.Println("Welcome to My:", binary)
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
		fmt.Println(err)
		return
	}

	fmt.Println("RPC:", config.Default.Rpc)
	fmt.Println("SUI binary path:", config.Default.SuiBinaryPath)
	fmt.Println("Address:", config.Default.Address)
	fmt.Println("Gas object to pay:", config.Default.GasObjToPay)
	fmt.Println("Primary coin:", config.Default.PrimaryCoin)
}

func mergeCoins() {
	getCoins()
	config, err := ReadConfigFile(configFilePath)
	if err != nil {
		fmt.Println(err)
		return
	}
	path := config.Default.SuiBinaryPath
	primaryCoin := config.Default.PrimaryCoin
	coinToMerge := ""
	gasBudget := "2000000"
	gasObjectToPay := config.Default.GasObjToPay

	cmd := exec.Command(path, "client", "merge-coin",
		"--primary-coin", primaryCoin,
		"--coin-to-merge", coinToMerge,
		"--gas-budget="+gasBudget,
		"--gas", gasObjectToPay)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr

	runs := cmd.Run()
	if runs != nil {
		log.Fatal(runs)
	}
}
