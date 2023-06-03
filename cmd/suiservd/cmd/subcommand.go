package cmd

import (
	"encoding/json"
	"fmt"

	"github.com/spf13/cobra"
)

func init() {
	RootCmd.AddCommand(initCommand)
	RootCmd.AddCommand(mergecoinCommand)
	RootCmd.AddCommand(withdrawCommand)
	RootCmd.AddCommand(versionCommand)
}

var initCommand = &cobra.Command{
	Use:   "init",
	Short: "Initialize config",
	Long:  "Initialize config",
	Run: func(cmd *cobra.Command, args []string) {
		initConfig()
		initConfigFile()
		readConfig()
	},
}

var mergecoinCommand = &cobra.Command{
	Use:   "merge-coins",
	Short: "Merging all sui::SUI objects to PRIMARY_COIN",
	Long:  "Merging all sui::SUI objects to PRIMARY_COIN",
	Run: func(cmd *cobra.Command, args []string) {
		mergeCoins()
	},
}

var withdrawCommand = &cobra.Command{
	Use:   "withdraw",
	Short: "Withdrawing all sui::SuiStaked objects",
	Long:  "Withdrawing all sui::SuiStaked objects",
	Run: func(cmd *cobra.Command, args []string) {
		// Behavior when the subcommand is executed
		fmt.Println("Executing subcommand...")
	},
}

var versionCommand = &cobra.Command{
	Use:   "version",
	Short: "Print the CLI version",
	Long:  "Print the CLI version",
	Run: func(cmd *cobra.Command, args []string) {
		data := struct {
			Version string `json:"version"`
		}{
			Version: version,
		}

		jsonData, err := json.Marshal(data)
		if err != nil {
			fmt.Println("Error:", err)
			return
		}

		fmt.Println(string(jsonData))
	},
}
