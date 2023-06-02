package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

func init() {
	RootCmd.AddCommand(subCommand)
}

var subCommand = &cobra.Command{
	Use:   "merge-coins",
	Short: "Merging all sui::SUI objects to PRIMARY_COIN",
	Long:  "Merging all sui::SUI objects to PRIMARY_COIN",
	Run: func(cmd *cobra.Command, args []string) {
		// Behavior when the subcommand is executed
		fmt.Println("Executing subcommand...")
	},
}
