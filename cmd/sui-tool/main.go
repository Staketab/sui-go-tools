package main

import (
	"fmt"
	"os"

	cmd "github.com/staketab/sui-go-tools/cmd/sui-tool/cmd"
)

func main() {
	if err := cmd.RootCmd.Execute(); err != nil {
		fmt.Println(err)
		os.Exit(1)
	}
}
