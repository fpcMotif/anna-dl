package main

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/Nquxii/anna-dl-go/config"
	"github.com/Nquxii/anna-dl-go/ui"
	"github.com/spf13/cobra"
)

var (
	version = "0.1.0"
	cfg     *config.Config
)

var rootCmd = &cobra.Command{
	Use:     "annadl [search query]",
	Short:   "A beautiful TUI for downloading books from Anna's Archive",
	Long:    `A Go CLI tool with Bubble Tea TUI for searching and downloading books from Anna's Archive`,
	Version: version,
	Args:    cobra.MaximumNArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		query := ""
		if len(args) > 0 {
			query = args[0]
		}

		interactive, _ := cmd.Flags().GetBool("interactive")
		numResults, _ := cmd.Flags().GetInt("num-results")
		downloadPath, _ := cmd.Flags().GetString("download-path")

		// Handle download path configuration
		if path := cfg.DownloadPath; path != "" && downloadPath == "" {
			downloadPath = path
		}
		if downloadPath == "" {
			home, _ := os.UserHomeDir()
			downloadPath = filepath.Join(home, "Downloads", "anna-dl")
		}

		// Run TUI for interactive mode or when no query is provided
		if interactive || query == "" {
			if err := ui.RunTUI(query, downloadPath, numResults); err != nil {
				fmt.Fprintf(os.Stderr, "Error running TUI: %v\n", err)
				os.Exit(1)
			}
		} else {
			// Run non-interactive mode
			if err := ui.RunNonInteractive(query, downloadPath, numResults); err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}
		}
	},
}

var configCmd = &cobra.Command{
	Use:   "config",
	Short: "Manage configuration",
}

var setPathCmd = &cobra.Command{
	Use:   "set-path [path]",
	Short: "Set default download path",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		path := args[0]
		absPath, err := filepath.Abs(path)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Invalid path: %v\n", err)
			os.Exit(1)
		}

		cfg.DownloadPath = absPath
		if err := cfg.Save(); err != nil {
			fmt.Fprintf(os.Stderr, "Failed to save config: %v\n", err)
			os.Exit(1)
		}
		fmt.Printf("Download path set to: %s\n", absPath)
	},
}

var showConfigCmd = &cobra.Command{
	Use:   "show",
	Short: "Show current configuration",
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Println("Current configuration:")
		fmt.Printf("  Download path: %s\n", cfg.DownloadPath)
	},
}

func init() {
	// Initialize config
	var err error
	cfg, err = config.Load()
	if err != nil {
		fmt.Fprintf(os.Stderr, "Warning: Failed to load config: %v\n", err)
		cfg = &config.Config{}
	}

	// Add flags
	rootCmd.Flags().BoolP("interactive", "i", false, "Interactive mode (default when no query provided)")
	rootCmd.Flags().IntP("num-results", "n", 10, "Number of results to show")
	rootCmd.Flags().StringP("download-path", "p", "", "Download path (overrides config)")

	// Add config subcommands
	configCmd.AddCommand(setPathCmd)
	configCmd.AddCommand(showConfigCmd)
	rootCmd.AddCommand(configCmd)
}

func main() {
	if err := rootCmd.Execute(); err != nil {
		os.Exit(1)
	}
}
