package cmd

import (
	"log"
	"os"
)

var (
	infoLog        = log.New(os.Stdout, "INFO: ", log.Ldate|log.Ltime)
	errorLog       = log.New(os.Stderr, "ERROR: ", log.Ldate|log.Ltime|log.Lshortfile)
	binary         = "suid"
	version        = "1.0.4"
	configPath     = ".sui-config/"
	configFilePath = ".sui-config/config.toml"
)
