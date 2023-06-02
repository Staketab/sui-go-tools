package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

var RootCmd = &cobra.Command{
	Use:   "suiservd",
	Short: "SUI tool to merge-coins, withdraw stakets and others.",
	Long:  "SUI tool to merge-coins, withdraw stakets and others.",
	Run: func(cmd *cobra.Command, args []string) {
		// Default behavior when the root command is executed
		fmt.Println("Welcome to My CLI!")
	},
}
