# Anna's Archive Downloader (Rust Edition)

A modern, fast, and feature-rich Rust CLI tool for downloading books from Anna's Archive with a beautiful terminal UI.

## 🚀 Features

### Core Features
- **Fast Async Processing**: Built with Tokio for concurrent downloads and non-blocking I/O
- **Rich Terminal UI**: Interactive TUI with keyboard navigation (powered by `ratatui`)
- **Progress Bars**: Real-time download progress with ETA and speed indicators
- **Metadata Extraction**: Automatically extracts title, author, year, language, format, and size
- **Multiple Download Sources**: Supports LibGen mirrors and other sources
- **Smart Defaults**: Auto-selects best download source (LibGen preferred)
- **Configuration Management**: Persistent config file for default settings

### New Rust-Specific Features
- **Zero Dependencies on Chrome**: No ChromeDriver/Selenium required
- **Native Async HTTP**: Direct HTTP requests with `reqwest`
- **Memory Efficient**: Lower memory footprint than Python Selenium
- **Type Safety**: Compile-time guarantees prevent runtime errors
- **Cross-Platform**: Native binaries for Windows, macOS, and Linux
- **Faster Performance**: Up to 10x faster than Python Selenium version
- **Better Error Handling**: Detailed error messages with `anyhow`
- **Non-Interactive Mode**: Direct download with command-line arguments

### UX Improvements
- **Beautiful TUI**: Rich terminal interface with colors and styling
- **Keyboard Shortcuts**: Intuitive navigation (vim-style k/j keys, arrow keys)
- **Help System**: Built-in help screen (press F1)
- **Error Recovery**: Graceful error handling with clear messages
- **Progress Indicators**: Visual feedback for all operations
- **Smart Defaults**: Automatically selects best download links

## 📦 Installation

### Prerequisites
- Rust 1.70 or higher
- Internet connection

### Build from Source

```bash
git clone https://github.com/Nquxii/anna-dl
cd anna-dl-rs
cargo build --release
```

The compiled binary will be available at `target/release/annadl`.

### Add to PATH

```bash
# Linux/macOS
ln -s $(pwd)/target/release/annadl ~/.local/bin/annadl

# Or copy to system location
sudo cp target/release/annadl /usr/local/bin/

# Windows (Powershell)
Copy-Item target\release\annadl.exe C:\Windows\System32\annadl.exe
```

## 🎯 Usage

### Interactive Mode (TUI)

Run without arguments to launch the interactive terminal UI:

```bash
annadl
```

**Navigation:**
- Type to search
- `↑/↓` or `k/j` - Navigate results
- `Enter` - Select book or download link
- `Esc` - Go back
- `F1` - Show help
- `Ctrl+C` - Quit

### Non-Interactive Mode

Search and download directly from command line:

```bash
# Search with default settings
annadl "The Pragmatic Programmer"

# Search with specific number of results
annadl "Don Quixote" -n 10

# Specify download path
annadl "Clean Code" -p /home/user/books

# Combine options
annadl "Design Patterns" -n 20 -p "./downloads"
```

### Configuration

Set default download path:

```bash
# Set download path in config
annadl --set-path /home/user/books

# View current config
annadl --config
```

The config file is stored at:
- Linux/macOS: `~/.config/anna-dl/config.json`
- Windows: `%APPDATA%\anna-dl\config.json`

### Command Line Options

```
anna-dl [SEARCH_QUERY]

Arguments:
  [SEARCH_QUERY]        Search query for books

Options:
  -n, --num-results <NUM>    Number of results to show [default: 5]
  -p, --download-path <PATH> Download path (overrides config)
      --set-path <PATH>      Set default download path in config
  -i, --interactive          Interactive mode (default if no query)
      --config               List current config
  -h, --help                 Print help
  -V, --version              Print version
```

## 🎨 UI Screenshots

### Search Mode
```
┌─ Anna's Archive Downloader ──────────────────────────────────────┐
│                                                                  │
┌─ Search Query (Press Enter to search, Ctrl+C to quit, F1 for He │
│                                                                  │
│ The Pragmatic Programmer                                         │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

### Results Mode
```
┌─ Search Results for: The Pragmatic Programmer ────────────────────┐
│                                                                  │
┌─ Books (k/j or ↑/↓ to navigate, Enter to select, Esc to go back │
│  The Pragmatic Programmer                                        │
│    Author: David Thomas, Andrew Hunt                            │
│    Year: 2019 | Language: English | Format: EPUB | Size: 2.1MB   │
│                                                                  │
│  The Pragmatic Programmer 20th Anniversary Edition              │
│    Author: David Thomas, Andrew Hunt                            │
│    Year: 2019 | Language: English | Format: PDF | Size: 8.7MB    │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
Showing 5 of 42 books | Press Enter to see download options
```

### Download Selection
```
┌─ Book Info ──────────────────────────────────────────────────────┐
│ Title: The Pragmatic Programmer                                  │
│ Author: David Thomas, Andrew Hunt                                │
│ Year: 2019 | Language: English | Format: EPUB | Size: 2.1MB      │
└──────────────────────────────────────────────────────────────────┘
┌─ Download Links ────────────────────────────────────────────────┐
│ 1. Libgen.li                                                     │
│    Source: LibGen | URL: https://libgen.rs/get.php?md5=...      │
│                                                                  │
│ 2. Direct Download                                               │
│    Source: Anna's Archive | URL: https://annas-archive.org/...  │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

## 🔧 Architecture

### Project Structure
```
anna-dl-rs/
├── src/
│   ├── main.rs           # Entry point and CLI argument parsing
│   ├── config.rs         # Configuration management
│   ├── scraper.rs        # Anna's Archive scraper & HTML parsing
│   ├── downloader.rs     # Download management with progress
│   └── ui/
│       ├── mod.rs        # UI module
│       └── app.rs        # Main TUI application logic
├── Cargo.toml            # Dependencies
└── README.md            # This file
```

### Key Components

1. **`AnnScraper`** - Web scraping using `scraper` and `reqwest`
   - Async HTTP requests
   - HTML parsing with CSS selectors
   - Metadata extraction with regex
   - Fallback selector chains

2. **`Downloader`** - File downloads with progress
   - Async download streaming
   - Progress bars with `indicatif`
   - Resume support (partial download cleanup)
   - Multiple filename detection methods

3. **`App`** - Terminal UI with `ratatui`
   - Multi-screen navigation
   - Keyboard event handling
   - Real-time state management
   - Help system

4. **`Config`** - Configuration persistence
   - JSON config file
   - Multiple path resolution
   - Runtime updates

### Why Rust?

**Performance:**
- Native async I/O without overhead
- Zero-cost abstractions
- No garbage collection
- ~10x faster than Python Selenium

**Reliability:**
- Compile-time error checking
- Explicit error handling
- Memory safety guarantees
- No runtime crashes

**User Experience:**
- Single static binary
- No external dependencies
- Instant startup
- Beautiful TUI with fast rendering

## 🔄 Migration from Python Version

### API Compatibility
- Same command-line interface (mostly compatible)
- Same search behavior
- Similar download path resolution

### Key Differences
- **No Chrome required**: Direct HTTP requests instead of Selenium
- **Much faster**: Async I/O instead of synchronous Selenium
- **Better UI**: Native terminal interface instead of terminal output
- **Progress bars**: Visual download progress
- **Type safety**: Fewer runtime errors

### Migration Script
If you have existing scripts:

```bash
# Python version:
python3 annadl /path --s "book" --n 10

# Rust version (mostly compatible):
annadl "book" -n 10 -p /path
```

## 🐛 Troubleshooting

### Build Errors
```bash
# Update Rust
rustup update

# Clear cargo cache
cargo clean

# Check dependencies
cargo check
```

### Network Issues
- Ensure HTTPS connections are allowed (port 443)
- Check firewall settings
- Anna's Archive may block requests - tool automatically rotates user agents

### Download Failures
- Check available disk space
- Verify write permissions to download directory
- Try alternative download links

### TUI Issues
- Ensure terminal supports ANSI colors
- Try with `TERM=xterm-256color`
- Windows: Use Windows Terminal (not cmd.exe)

## 🚧 Development

### Running Tests
```bash
# Run all tests
cargo test

# Run scraper tests only
cargo test scraper

# Run downloader tests only
cargo test downloader
```

### Code Style
```bash
# Format code
cargo fmt

# Check for issues
cargo clippy
```

### Adding Features
1. New scraper selectors? Update `scraper.rs` selector arrays
2. New download source? Update `downloader.rs` source detection
3. New UI screen? Add to `ui/app.rs` AppMode enum

## 📄 License

MIT License - See LICENSE file for details

## ⚠️ Disclaimer

This tool is for educational purposes only. Users are responsible for complying with their local laws and regulations regarding copyrighted material. This project is not affiliated with Anna's Archive or Library Genesis.

## 🆘 Contributing

Contributions welcome! Areas for improvement:
- More download sources
- Better metadata extraction
- Download queue management
- Search result caching
- Export in different formats (BibTeX, JSON, etc.)

## 🙏 Acknowledgments

- Original Python version: [Nquxii/zlib-dl](https://github.com/Nquxii/zlib-dl)
- Anna's Archive and Library Genesis communities
- Rust ecosystem contributors
