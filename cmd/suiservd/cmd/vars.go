package cmd

import (
	"log"
	"os"
)

var (
	infoLog  = log.New(os.Stdout, "INFO: ", log.Ldate|log.Ltime)
	errorLog = log.New(os.Stderr, "ERROR: ", log.Ldate|log.Ltime|log.Lshortfile)
	binary   = "suiservd"
	version  = "v1.0.0"
)
