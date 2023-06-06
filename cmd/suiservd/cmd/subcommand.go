package cmd

import (
	"github.com/spf13/cobra"
)

func init() {
	mergeCoinCommand.Flags().StringP("primary-coin", "p", "", "The primary coin for merging, in 20 bytes Hex string")
	mergeCoinCommand.Flags().StringSliceP("coin-to-merge", "c", []string{}, "Coin to be merged, in 20 bytes Hex string")

	// Set the usage description for the command
	mergeCoinCommand.SetUsageTemplate(`Usage:
  merge [flags]

Flags:
  -p, --primary-coin string   The primary coin for merging, in 20 bytes Hex string
  -c, --coin-to-merge string   Coin to be merged, in 20 bytes Hex string`)

	RootCmd.AddCommand(initCommand)
	RootCmd.AddCommand(mergeCoinCommand)
	RootCmd.AddCommand(mergeCoinsCommand)
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

var mergeCoinCommand = &cobra.Command{
	Use:   "merge",
	Short: "Merge sui::SUI object to PRIMARY_COIN",
	Long:  "Merge sui::SUI object to PRIMARY_COIN",
	Run: func(cmd *cobra.Command, args []string) {
		primaryCoin, _ := cmd.Flags().GetString("primary-coin")
		coinsToMerge, _ := cmd.Flags().GetStringSlice("coin-to-merge")

		// Вызов вашей функции с передачей аргументов
		mergeCoin(coinsToMerge, primaryCoin)
	},
}

var mergeCoinsCommand = &cobra.Command{
	Use:   "merge-all",
	Short: "Merging all sui::SUI objects to PRIMARY_COIN",
	Long:  "Merging all sui::SUI objects to PRIMARY_COIN",
	Run: func(cmd *cobra.Command, args []string) {
		getMergeAll()
	},
}

var withdrawCommand = &cobra.Command{
	Use:   "withdraw-all",
	Short: "Withdrawing all sui::SuiStaked objects",
	Long:  "Withdrawing all sui::SuiStaked objects",
	Run: func(cmd *cobra.Command, args []string) {
		getPayObj()
	},
}

var versionCommand = &cobra.Command{
	Use:   "version",
	Short: "Print the CLI version",
	Long:  "Print the CLI version",
	Run: func(cmd *cobra.Command, args []string) {
		getVersion()
	},
}
