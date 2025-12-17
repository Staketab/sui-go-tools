package cmd

import (
	"fmt"
	"log"
	"os"
)

// ANSI color codes
const (
	colorReset  = "\033[0m"
	colorRed    = "\033[31m"
	colorGreen  = "\033[32m"
	colorYellow = "\033[33m"
	colorBlue   = "\033[34m"
	colorCyan   = "\033[36m"
	colorWhite  = "\033[37m"
	colorBold   = "\033[1m"
)

// Supported blockchain networks
const (
	ChainSUI  = "sui"
	ChainIOTA = "iota"
)

var (
	infoLog        = log.New(os.Stdout, fmt.Sprintf("%s[INFO]%s ", colorCyan, colorReset), log.Ldate|log.Ltime)
	errorLog       = log.New(os.Stderr, fmt.Sprintf("%s[ERROR]%s ", colorRed, colorReset), log.Ldate|log.Ltime|log.Lshortfile)
	successLog     = log.New(os.Stdout, fmt.Sprintf("%s[SUCCESS]%s ", colorGreen, colorReset), log.Ldate|log.Ltime)
	warnLog        = log.New(os.Stdout, fmt.Sprintf("%s[WARN]%s ", colorYellow, colorReset), log.Ldate|log.Ltime)
	binary         = "mcli"
	version        = "1.3.0"
	configPath     = ".mcli-config/"
	configFilePath = ".mcli-config/config.toml"

	// activeChain holds the currently selected blockchain
	activeChain string
)
