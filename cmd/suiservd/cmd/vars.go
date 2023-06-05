package cmd

import (
	"log"
	"os"
)

var (
	infoLog        = log.New(os.Stdout, "INFO: ", log.Ldate|log.Ltime)
	errorLog       = log.New(os.Stderr, "ERROR: ", log.Ldate|log.Ltime|log.Lshortfile)
	binary         = "sui-tool"
	version        = "1.2.0"
	configPath     = ".sui-config/"
	configFilePath = ".sui-config/config.toml"
)
