package config

import (
	"encoding/json"
	"os"
	"path/filepath"

	"github.com/mitchellh/go-homedir"
)

type Config struct {
	DownloadPath string `json:"download_path"`
}

func Load() (*Config, error) {
	configPath, err := getConfigPath()
	if err != nil {
		return nil, err
	}

	// Default config
	config := &Config{}

	// Check if config file exists
	if _, err := os.Stat(configPath); os.IsNotExist(err) {
		// Create config directory
		configDir := filepath.Dir(configPath)
		if err := os.MkdirAll(configDir, 0755); err != nil {
			return config, nil // Return default if can't create
		}
		return config, nil
	}

	// Load existing config
	data, err := os.ReadFile(configPath)
	if err != nil {
		return config, nil // Return default if can't read
	}

	if err := json.Unmarshal(data, config); err != nil {
		return &Config{}, nil // Return empty if can't parse
	}

	return config, nil
}

func (c *Config) Save() error {
	configPath, err := getConfigPath()
	if err != nil {
		return err
	}

	// Create config directory
	configDir := filepath.Dir(configPath)
	if err := os.MkdirAll(configDir, 0755); err != nil {
		return err
	}

	// Marshal config
	data, err := json.MarshalIndent(c, "", "  ")
	if err != nil {
		return err
	}

	// Write config file
	return os.WriteFile(configPath, data, 0644)
}

func getConfigPath() (string, error) {
	home, err := homedir.Dir()
	if err != nil {
		return "", err
	}
	return filepath.Join(home, ".config", "anna-dl-go", "config.json"), nil
}
