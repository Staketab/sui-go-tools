package cmd

import (
	"github.com/spf13/cobra"
)

// Chain parent commands
var suiCmd = &cobra.Command{
	Use:   "sui",
	Short: "SUI blockchain operations",
	Long:  "Operations for SUI blockchain: merge, withdraw, send",
	PersistentPreRun: func(cmd *cobra.Command, args []string) {
		activeChain = ChainSUI
	},
}

var iotaCmd = &cobra.Command{
	Use:   "iota",
	Short: "IOTA blockchain operations",
	Long:  "Operations for IOTA blockchain: merge, withdraw, send",
	PersistentPreRun: func(cmd *cobra.Command, args []string) {
		activeChain = ChainIOTA
	},
}

// createMergeCommand creates a merge command for a specific chain
func createMergeCommand() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "merge",
		Short: "Merge coin objects to PRIMARY_COIN",
		Long:  "Merge coin objects to PRIMARY_COIN",
		Run: func(cmd *cobra.Command, args []string) {
			primaryCoin, _ := cmd.Flags().GetString("primary-coin")
			coinsToMerge, _ := cmd.Flags().GetStringSlice("coins-to-merge")
			mergeCoin(coinsToMerge, primaryCoin)
		},
	}
	cmd.Flags().StringP("primary-coin", "p", "", "The primary coin for merging, in 20 bytes Hex string")
	cmd.Flags().StringSliceP("coins-to-merge", "c", []string{}, "Coins to be merged, in 20 bytes Hex string")
	cmd.SetUsageTemplate(`Usage:
  merge [flags]

Flags:
  -p, --primary-coin string   The primary coin for merging, in 20 bytes Hex string
  -c, --coins-to-merge string, array   Coins to be merged, in 20 bytes Hex string

`)
	return cmd
}

// createMergeAllCommand creates a merge-all command for a specific chain
func createMergeAllCommand() *cobra.Command {
	return &cobra.Command{
		Use:   "merge-all",
		Short: "Merge all coin objects to PRIMARY_COIN",
		Long:  "Merge all coin objects to PRIMARY_COIN",
		Run: func(cmd *cobra.Command, args []string) {
			getMergeAll()
		},
	}
}

// createWithdrawAllCommand creates a withdraw-all command for a specific chain
func createWithdrawAllCommand() *cobra.Command {
	return &cobra.Command{
		Use:   "withdraw-all",
		Short: "Withdraw all staked objects",
		Long:  "Withdraw all staked objects",
		Run: func(cmd *cobra.Command, args []string) {
			getPayObj()
		},
	}
}

// createSendCommand creates a send command for a specific chain
func createSendCommand() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "send",
		Short: "Send coins to an address",
		Long:  "Send specified amount of coins to a recipient address",
		Run: func(cmd *cobra.Command, args []string) {
			recipient, _ := cmd.Flags().GetString("recipient")
			amount, _ := cmd.Flags().GetString("amount")
			getInputObj(recipient, amount)
		},
	}
	cmd.Flags().StringP("recipient", "r", "", "Recipient address")
	cmd.Flags().StringP("amount", "a", "", "Amount to send")
	cmd.MarkFlagRequired("recipient")
	cmd.MarkFlagRequired("amount")
	cmd.SetUsageTemplate(`Usage:
  send [flags]

Flags:
  -r, --recipient string   Recipient address
  -a, --amount string     Amount to send
`)
	return cmd
}

var initCommand = &cobra.Command{
	Use:   "init",
	Short: "Initialize config",
	Long:  "Initialize config file with default settings for all supported chains",
	Run: func(cmd *cobra.Command, args []string) {
		initConfig()
		initConfigFile()
		readConfig()
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

func init() {
	// Add subcommands to SUI parent
	suiCmd.AddCommand(createMergeCommand())
	suiCmd.AddCommand(createMergeAllCommand())
	suiCmd.AddCommand(createWithdrawAllCommand())
	suiCmd.AddCommand(createSendCommand())

	// Add subcommands to IOTA parent
	iotaCmd.AddCommand(createMergeCommand())
	iotaCmd.AddCommand(createMergeAllCommand())
	iotaCmd.AddCommand(createWithdrawAllCommand())
	iotaCmd.AddCommand(createSendCommand())

	// Add chain commands to root
	RootCmd.AddCommand(suiCmd)
	RootCmd.AddCommand(iotaCmd)

	// Add global commands to root
	RootCmd.AddCommand(initCommand)
	RootCmd.AddCommand(versionCommand)
}
