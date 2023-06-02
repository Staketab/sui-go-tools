package main

import (
	"fmt"
	"os"

	"github.com/staketab/sui-go-tools/cmd/suiservd/cmd"
)

func main() {
	if err := cmd.RootCmd.Execute(); err != nil {
		fmt.Println(err)
		os.Exit(1)
	}
}
