package cmd

import (
	"encoding/json"
	"fmt"
)

func getVersion() {
	data := struct {
		Version string `json:"version"`
	}{
		Version: version,
	}

	jsonData, err := json.Marshal(data)
	if err != nil {
		errorLog.Println("Error:", err)
		return
	}

	fmt.Println(string(jsonData))
}
