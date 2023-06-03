package cmd

import (
	"fmt"

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
