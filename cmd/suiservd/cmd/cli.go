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

func mergeCoins(slice []string, gas, primaryobj string) {
	config, err := ReadConfigFile(configFilePath)
	if err != nil {
		fmt.Println(err)
		return
	}
	for _, value := range slice[1:] {
		fmt.Println("Array value to merge:", value)
		if value != "" {
			fmt.Println("RPC:", config.Default.Rpc)
			fmt.Println("SUI binary path:", config.Default.SuiBinaryPath)
			fmt.Println("Primary coin:", primaryobj)
			fmt.Println("Array value to merge:", value)
			fmt.Println("Gas Budget:", gas)

			primaryCoin := primaryobj
			coinToMerge := value
			gasBudget := gas

			cmd := exec.Command(config.Default.SuiBinaryPath, "client", "merge-coin",
				"--primary-coin", primaryCoin,
				"--coin-to-merge", coinToMerge,
				"--gas-budget="+gasBudget)
			cmd.Stdout = os.Stdout
			cmd.Stderr = os.Stderr

			runs := cmd.Run()
			if runs != nil {
				log.Fatal(runs)
			}
		} else {
			log.Fatal("All coins merged.")
		}
	}
}

func withdrawStakes(slice []string, gas, primaryobj string) {
	config, err := ReadConfigFile(configFilePath)
	if err != nil {
		fmt.Println(err)
		return
	}
	for _, value := range slice[1:] {
		if value != "" {
			fmt.Println("RPC:", config.Default.Rpc)
			fmt.Println("SUI binary path:", config.Default.SuiBinaryPath)
			fmt.Println("Primary coin:", primaryobj)
			fmt.Println("Array value to withdraw:", value)
			fmt.Println("Gas Budget:", gas)
			fmt.Println("Gas odject to pay:", primaryobj)

			gasBudget := gas
			stakesId := value

			cmd := exec.Command(config.Default.SuiBinaryPath, "client", "call",
				"--package", config.Default.Package,
				"--module", config.Default.Module,
				"--function", config.Default.Function,
				"--args", config.Default.Args,
				stakesId,
				"--gas-budget="+gasBudget,
				"--gas", primaryobj)
			cmd.Stdout = os.Stdout
			cmd.Stderr = os.Stderr

			runs := cmd.Run()
			if runs != nil {
				log.Fatal(runs)
			}
		} else {
			fmt.Println("All coins merged.")
		}
	}
}
