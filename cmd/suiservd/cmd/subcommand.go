package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

func init() {
	RootCmd.AddCommand(mergecoin)
	RootCmd.AddCommand(withdraw)
}

var mergecoin = &cobra.Command{
	Use:   "merge-coins",
	Short: "Merging all sui::SUI objects to PRIMARY_COIN",
	Long:  "Merging all sui::SUI objects to PRIMARY_COIN",
	Run: func(cmd *cobra.Command, args []string) {
		// Behavior when the subcommand is executed
		fmt.Println("Executing subcommand...")
	},
}

var withdraw = &cobra.Command{
	Use:   "withdraw",
	Short: "Withdrawing all sui::SuiStaked objects",
	Long:  "Withdrawing all sui::SuiStaked objects",
	Run: func(cmd *cobra.Command, args []string) {
		// Behavior when the subcommand is executed
		fmt.Println("Executing subcommand...")
	},
}
