package cmd

import (
	"github.com/spf13/cobra"
)

func init() {
	mergecoinCommand.Flags().StringP("primary-coin", "p", "", "The primary coin for merging, in 20 bytes Hex string")
	mergecoinCommand.Flags().StringSliceP("coin-to-merge", "c", []string{}, "Coin to be merged, in 20 bytes Hex string")
	mergecoinCommand.Flags().StringP("gas", "g", "", "ID of the gas object for gas payment")

	// Set the usage description for the command
	mergecoinCommand.SetUsageTemplate(`Usage:
  merge [flags]

Flags:
  -p, --primary-coin string   The primary coin for merging, in 20 bytes Hex string
  -c, --coin-to-merge string   Coin to be merged, in 20 bytes Hex string
      --gas string    ID of the gas object for gas payment`)

	RootCmd.AddCommand(initCommand)
	RootCmd.AddCommand(mergecoinCommand)
	RootCmd.AddCommand(mergecoinsCommand)
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
	Use:   "merge",
	Short: "Merging all sui::SUI objects to PRIMARY_COIN",
	Long:  "Merging all sui::SUI objects to PRIMARY_COIN",
	Run: func(cmd *cobra.Command, args []string) {
		primaryCoin, _ := cmd.Flags().GetString("primary-coin")
		coinsToMerge, _ := cmd.Flags().GetStringSlice("coin-to-merge")
		gas, _ := cmd.Flags().GetString("gas")

		// Вызов вашей функции с передачей аргументов
		mergeCoin(coinsToMerge, gas, primaryCoin)
	},
}

var mergecoinsCommand = &cobra.Command{
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
