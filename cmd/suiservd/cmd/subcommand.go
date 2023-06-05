package cmd

import (
	"github.com/spf13/cobra"
)

func init() {
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

	},
}

var mergecoinsCommand = &cobra.Command{
	Use:   "merge-all",
	Short: "Merging all sui::SUI objects to PRIMARY_COIN",
	Long:  "Merging all sui::SUI objects to PRIMARY_COIN",
	Run: func(cmd *cobra.Command, args []string) {
		getMergeData()
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
