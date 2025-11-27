package main

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/Nquxii/anna-dl-go/config"
)

func TestConfigLoading(t *testing.T) {
	// Create a temporary config directory
	tempDir := t.TempDir()
	configDir := filepath.Join(tempDir, ".config", "anna-dl-go")
	os.MkdirAll(configDir, 0755)

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
	cfg := &config.Config{
		DownloadPath: "~/Downloads/test",
	}

	// Test saving (this will use the actual config path)
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
