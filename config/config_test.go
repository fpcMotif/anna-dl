package config

import (
	"os"
	"path/filepath"
	"testing"
)

func TestConfigFlow(t *testing.T) {
	// Setup temp config file
	tempDir := t.TempDir()
	configFile := filepath.Join(tempDir, "config.json")
	ConfigPathOverride = configFile
	defer func() { ConfigPathOverride = "" }()

	// Test 1: Load non-existent (should return default)
	cfg, err := Load()
	if err != nil {
		t.Fatalf("Load failed: %v", err)
	}
	if cfg.DownloadPath != "" {
		t.Errorf("Expected empty default path, got %s", cfg.DownloadPath)
	}

	// Test 2: Save config
	cfg.DownloadPath = "/tmp/downloads"
	if err := cfg.Save(); err != nil {
		t.Fatalf("Save failed: %v", err)
	}

	// Verify file exists
	if _, err := os.Stat(configFile); os.IsNotExist(err) {
		t.Fatalf("Config file not created")
	}

	// Test 3: Load existing
	cfg2, err := Load()
	if err != nil {
		t.Fatalf("Load re-read failed: %v", err)
	}
	if cfg2.DownloadPath != "/tmp/downloads" {
		t.Errorf("Expected '/tmp/downloads', got '%s'", cfg2.DownloadPath)
	}
}
