package main

import (
	"fmt"

	"github.com/spf13/cobra"
)

func init() {
	RootCmd.AddCommand(subCommand)
}

var subCommand = &cobra.Command{
	Use:   "merge",
	Short: "A subcommand example",
	Long:  "An example subcommand for My CLI",
	Run: func(cmd *cobra.Command, args []string) {
		// Behavior when the subcommand is executed
		fmt.Println("Executing subcommand...")
	},
}
