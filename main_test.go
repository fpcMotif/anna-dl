package main

import (
	"path/filepath"
	"testing"

	"github.com/Nquxii/anna-dl-go/config"
)

func TestConfigLoading(t *testing.T) {
	// Create a temporary config directory
	tempDir := t.TempDir()
	configFile := filepath.Join(tempDir, "config.json")
	config.ConfigPathOverride = configFile
	defer func() { config.ConfigPathOverride = "" }()

	// Test loading non-existent config (should return default)
	cfg, err := config.Load()
	if err != nil {
		t.Fatalf("Failed to load config: %v", err)
	}

	if cfg == nil {
		t.Fatal("Expected config, got nil")
	}
}

func TestConfigSaving(t *testing.T) {
	tempDir := t.TempDir()
	configFile := filepath.Join(tempDir, "config.json")
	config.ConfigPathOverride = configFile
	defer func() { config.ConfigPathOverride = "" }()

	cfg := &config.Config{
		DownloadPath: "~/Downloads/test",
	}

	// Test saving
	err := cfg.Save()
	if err != nil {
		t.Fatalf("Failed to save config: %v", err)
	}

	// Reload and verify
	cfg2, err := config.Load()
	if err != nil {
		t.Fatalf("Failed to reload config: %v", err)
	}

	if cfg2.DownloadPath != cfg.DownloadPath {
		t.Errorf("Config mismatch: got %s, want %s", cfg2.DownloadPath, cfg.DownloadPath)
	}
}
