package cmd

import (
	"fmt"
	"os"
	"os/exec"
	"os/user"
	"path/filepath"
	"strings"
)

func isRpcWorking() bool {
	infoLog.Println("Start checking node status")
	cmd := exec.Command("/bin/bash", "-c", "docker exec -i mina mina client status --json | jq -r .sync_status")
	out, err := cmd.Output()
	if err != nil {
		errorLog.Println("Failed to get node status:", err)
		return false
	}

	status := strings.TrimSpace(string(out))
	if status != "Synced" {
		errorLog.Println("Node is not synced, status:", status)
		return false
	}

	infoLog.Println("Node is synced, status:", status)
	return true
}

func createDirectory(path string) error {
	usr, err := user.Current()
	if err != nil {
		return fmt.Errorf("failed to get current user: %s", err)
	}

	fullPath := filepath.Join(usr.HomeDir, path)

	err = os.MkdirAll(fullPath, os.ModePerm)
	if err != nil {
		return fmt.Errorf("failed to create directory: %s", err)
	}

	fmt.Printf("Directory created: %s\n", fullPath)
	return nil
}
