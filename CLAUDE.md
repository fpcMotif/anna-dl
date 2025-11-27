# CLAUDE.md - AI Assistant Development Guide

## Project Overview

**anna-dl** is a modern Rust CLI tool for downloading books from Anna's Archive with a rich terminal user interface (TUI). This is a complete rewrite from the original Python/Selenium version, providing ~10x performance improvement and eliminating the Chrome/ChromeDriver dependency.

### Key Facts
- **Language**: Rust (edition 2021)
- **Binary Name**: `annadl`
- **Total Lines**: ~1595 lines of Rust code
- **Main Dependencies**: tokio, reqwest, ratatui, scraper, clap
- **License**: MIT
- **Repository**: https://github.com/Nquxii/anna-dl

### Project Goals
1. Fast, async book search and download from Anna's Archive
2. Beautiful terminal UI with keyboard navigation
3. Zero external dependencies (no Chrome, no Selenium)
4. Cross-platform support (Linux, macOS, Windows)
5. Single static binary distribution

## Codebase Structure

```
anna-dl/
├── src/
│   ├── main.rs              # Entry point, CLI parsing, terminal setup (~200 lines)
│   ├── config.rs            # Configuration management (~70 lines)
│   ├── scraper.rs           # HTTP + HTML parsing (~350 lines)
│   ├── downloader.rs        # Async downloads with progress (~200 lines)
│   └── ui/
│       ├── mod.rs          # UI module exports (~5 lines)
│       └── app.rs          # Main TUI application logic (~770 lines)
├── Cargo.toml               # Rust dependencies and build config
├── README.md                # User documentation
├── AGENTS.md                # Detailed Rust development guide
├── WARP.md                  # Historical Python version info
├── .github/workflows/       # CI/CD automation
│   └── rust.yml            # Build and test workflow
├── annadl                   # Legacy Python script (deprecated)
├── config.json              # Default config file
└── requirements.in          # Legacy Python requirements (deprecated)
```

## Architecture

### Module Responsibilities

#### `main.rs` - Entry Point
- CLI argument parsing using `clap` with derive macros
- Tokio async runtime initialization (`#[tokio::main]`)
- Terminal setup/cleanup with `crossterm`
- Mode dispatch: TUI vs non-interactive mode
- Config loading and path resolution

**Key Pattern:**
```rust
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = config::Config::load()?;

    if cli.interactive || cli.search_query.is_none() {
        run_tui(config, cli).await?;
    } else {
        run_non_interactive(config, cli).await?;
    }
}
```

#### `config.rs` - Configuration Management
- JSON config persistence using `serde` and `serde_json`
- Config location: `~/.config/anna-dl/config.json` (Linux/macOS)
- Download path resolution priority: CLI arg > config file > `./assets/`
- Auto-creates config directory if missing

**Key Methods:**
- `Config::load()` - Load or create default config
- `Config::save()` - Persist config to disk
- `Config::download_path(cli_path)` - Resolve final download path
- `Config::set_download_path(path)` - Update and save

#### `scraper.rs` - Web Scraping
- Direct HTTP requests using `reqwest` (no browser required)
- HTML parsing with `scraper` crate (CSS selectors)
- Regex-based metadata extraction
- Fallback selector chains for robustness
- Random user-agent rotation

**Key Components:**
```rust
pub struct Book {
    pub title: String,
    pub author: Option<String>,
    pub year: Option<String>,
    pub language: Option<String>,
    pub format: Option<String>,
    pub size: Option<String>,
    pub url: String,
}

pub struct AnnaScraper {
    client: reqwest::Client,
}

impl AnnaScraper {
    pub async fn search(&self, query: &str, max_results: usize) -> Result<Vec<Book>>
    pub async fn get_book_details(&self, book_url: &str) -> Result<Vec<DownloadLink>>
}
```

**Fallback Strategy:**
- Multiple CSS selectors tried in sequence
- Graceful degradation when metadata missing
- User-agent rotation to avoid rate limiting

#### `downloader.rs` - File Downloads
- Async streaming downloads with `tokio`
- Progress bars using `indicatif`
- Filename detection from URL and Content-Disposition header
- Automatic download directory creation
- Progress tracking with ETA and speed

**Key Methods:**
```rust
pub struct Downloader {
    client: reqwest::Client,
    download_path: PathBuf,
}

impl Downloader {
    pub async fn download(&self, url: &str, filename: Option<&str>) -> Result<PathBuf>
}
```

**Download Flow:**
1. GET request to URL
2. Extract content length for progress bar
3. Determine filename (URL → Content-Disposition → fallback)
4. Stream response to file with progress updates
5. Return final file path

#### `ui/app.rs` - Terminal UI
- Built with `ratatui` and `crossterm`
- State-driven architecture using `AppMode` enum
- Event loop pattern: input → state update → render
- Keyboard navigation (vim keys + arrows)
- Multiple screens: Search, Results, DownloadSelection, Downloading, Error, Help

**App States:**
```rust
enum AppMode {
    Search,                    // Initial search input
    Results,                   // Book search results list
    DownloadSelection,         // Download link selection
    Downloading,               // Active download progress
    Error(String),             // Error message display
    Help,                      // Help screen (F1)
}
```

**Key Navigation:**
- `↑/↓` or `k/j` - Navigate lists
- `Enter` - Select item
- `Esc` - Go back
- `F1` - Toggle help
- `Ctrl+C` - Quit

### Async Communication Pattern

**Channel-Based Architecture:**
```rust
enum AppCommand {
    Search(String, usize),
    FetchDownloadLinks(String),
    Download(String, usize),
    ShowError(String),
    CompleteDownload(PathBuf),
}
```

**Flow:**
1. UI thread handles keyboard input
2. Spawns tokio task for heavy operations (search, download)
3. Task sends `AppCommand` back via channel
4. UI receives command and updates state
5. Next frame renders new state

**Critical Rule:** Terminal drawing MUST happen on main thread. All async operations (HTTP, file I/O) MUST run in spawned tasks.

## Development Workflow

### Essential Commands

```bash
# Quick development checks
cargo check                    # Fast compile check
cargo clippy                  # Linting with suggestions
cargo fmt                     # Format code to Rust standards

# Testing
cargo test                    # Run all tests
cargo test scraper           # Run scraper tests only
cargo test downloader        # Run downloader tests only

# Building
cargo build                   # Debug build
cargo build --release         # Optimized release build (~10MB binary)

# Running
cargo run                     # Launch TUI mode
cargo run -- "book title"    # Non-interactive search
cargo run -- --help          # Show help
cargo run -- --config        # Show current config

# Installation
cargo install --path .        # Install to ~/.cargo/bin/
```

### Build Optimization

The `Cargo.toml` includes aggressive optimization for release builds:

```toml
[profile.release]
opt-level = "z"        # Optimize for size
lto = true             # Link-time optimization
codegen-units = 1      # Single codegen unit for better optimization
strip = true           # Strip debug symbols
```

This produces a ~10MB static binary with excellent performance.

### Testing Strategy

**Current State:**
- Unit tests in `scraper.rs` and `downloader.rs`
- NO integration tests (Anna's Archive UI changes too frequently)
- Manual testing required for TUI functionality

**Running Tests:**
```bash
cargo test --verbose
```

**Test Coverage:**
- Filename extraction from URLs
- Content-Disposition header parsing
- Regex pattern matching for metadata
- Config file serialization/deserialization

**Manual Testing Checklist:**
1. Launch TUI: `cargo run`
2. Search for "test" and verify results display
3. Select a book and check download links
4. Download a file and verify progress bar
5. Test config: `cargo run -- --set-path /tmp/test`
6. Verify non-interactive mode: `cargo run -- "test query"`

### CI/CD Pipeline

**GitHub Actions Workflow** (`.github/workflows/rust.yml`):
- Triggers on push to `main` and pull requests
- Runs on `ubuntu-latest`
- Steps:
  1. Checkout code
  2. `cargo build --verbose`
  3. `cargo test --verbose`

**Environment:** `CARGO_TERM_COLOR: always` for readable output

## Key Conventions

### Code Style

1. **Formatting**: Always run `cargo fmt` before committing
2. **Linting**: Address all `cargo clippy` warnings
3. **Error Handling**: Use `anyhow::Result<T>` for public APIs, `?` operator for propagation
4. **Naming**:
   - `snake_case` for functions and variables
   - `PascalCase` for types and structs
   - `SCREAMING_SNAKE_CASE` for constants

### Error Handling Pattern

```rust
// In scraper/downloader: Propagate with ?
pub async fn search(&self, query: &str) -> Result<Vec<Book>> {
    let response = self.client.get(url).send().await?;
    let html = response.text().await?;
    self.parse_html(&html)
}

// In main/UI: Convert to user-friendly error
match scraper.search(query, max_results).await {
    Ok(books) => app.mode = Results { books },
    Err(e) => app.mode = Error(format!("Search failed: {}", e)),
}
```

### Async Patterns

**DO:**
```rust
// Spawn tasks for heavy operations
tokio::spawn(async move {
    let books = scraper.search(query, limit).await?;
    tx.send(AppCommand::Search(books))?;
});

// Use .await in async contexts
let html = client.get(url).send().await?.text().await?;
```

**DON'T:**
```rust
// NEVER block in async context
std::thread::sleep(Duration::from_secs(1)); // BAD!

// NEVER use blocking reqwest client
let client = reqwest::blocking::Client::new(); // BAD!

// NEVER call .unwrap() on Results
let books = scraper.search(query).await.unwrap(); // BAD!
```

### HTML Parsing Pattern

**Fallback Selector Chain:**
```rust
let selectors = [
    "a.js-vim-focus.custom-a",      // Primary selector
    "a[href*='md5']",                // Fallback 1
    ".book-title a",                 // Fallback 2
];

for selector_str in &selectors {
    if let Ok(selector) = Selector::parse(selector_str) {
        let elements: Vec<_> = document.select(&selector).collect();
        if !elements.is_empty() {
            return Ok(elements);
        }
    }
}
```

**Why:** Anna's Archive frequently changes CSS classes. Multiple fallbacks ensure robustness.

### TUI State Management

**State Update Pattern:**
```rust
// Handle keyboard event
match key.code {
    KeyCode::Enter => {
        match app.mode {
            Search => {
                // Spawn search task
                spawn_search_task(&app.search_query, &tx);
                app.mode = Loading;
            }
            Results => {
                let book = &app.books[app.selected_index];
                spawn_download_task(&book.url, &tx);
            }
        }
    }
    KeyCode::Esc => {
        app.mode = app.previous_mode();
    }
}

// Render based on state
match &app.mode {
    Search => draw_search_screen(f, &app.search_query),
    Results => draw_results_screen(f, &app.books, app.selected_index),
    Error(msg) => draw_error_screen(f, msg),
}
```

## Configuration Management

### Config File Location
- **Linux/macOS**: `~/.config/anna-dl/config.json`
- **Windows**: `%APPDATA%\anna-dl\config.json`
- **Fallback**: `./config.json` in current directory

### Config Structure
```json
{
  "download_path": "/home/user/books"
}
```

### Resolution Priority
1. CLI argument: `--download-path /path/to/dir`
2. Config file: `download_path` field
3. Default: `./assets/` in current directory

### Config Commands
```bash
annadl --set-path /home/user/books    # Set default path
annadl --config                        # View current config
```

## Dependencies Guide

### Core Dependencies

| Crate | Version | Purpose | Features |
|-------|---------|---------|----------|
| `reqwest` | 0.11 | HTTP client | json, rustls-tls, stream |
| `tokio` | 1.36 | Async runtime | full |
| `scraper` | 0.19 | HTML parsing | - |
| `ratatui` | 0.26 | Terminal UI | - |
| `crossterm` | 0.27 | Terminal control | - |
| `clap` | 4.5 | CLI parsing | derive, cargo |
| `serde` | 1.0 | Serialization | derive |
| `serde_json` | 1.0 | JSON support | - |
| `anyhow` | 1.0 | Error handling | - |
| `thiserror` | 1.0 | Error types | - |
| `indicatif` | 0.17 | Progress bars | - |
| `dirs` | 5.0 | Config paths | - |
| `regex` | 1.10 | Pattern matching | - |
| `urlencoding` | 2.1 | URL encoding | - |
| `rand` | 0.8 | Randomization | - |
| `futures` | 0.3 | Async utilities | - |

### Why These Choices?

- **reqwest**: Industry standard HTTP client, excellent async support
- **tokio**: De facto async runtime for Rust
- **ratatui**: Modern, actively maintained TUI framework (fork of tui-rs)
- **scraper**: Built on html5ever, handles malformed HTML well
- **clap**: Most popular CLI library, derive macros reduce boilerplate
- **anyhow**: Simple error handling for applications
- **indicatif**: Best terminal progress bar library

## Common Development Tasks

### Adding a New CSS Selector

When Anna's Archive changes their HTML structure:

1. Open `src/scraper.rs`
2. Find the relevant selector array (e.g., `TITLE_SELECTORS`)
3. Add new selector at the beginning (highest priority)
4. Test with `cargo run -- "test query"`

```rust
const TITLE_SELECTORS: &[&str] = &[
    "h1.new-title-class",           // NEW: Add here first
    "a.js-vim-focus.custom-a",      // Existing
    "a[href*='md5']",               // Fallback
];
```

### Adding a New App Screen

1. Add variant to `AppMode` enum in `ui/app.rs`:
```rust
enum AppMode {
    // ... existing variants
    NewScreen { data: Vec<String> },
}
```

2. Add rendering logic in `draw()` function:
```rust
match &app.mode {
    NewScreen { data } => draw_new_screen(f, data),
    // ... other matches
}
```

3. Add keyboard handling in `handle_keypress()`:
```rust
match key.code {
    KeyCode::Char('n') => {
        app.mode = NewScreen { data: vec![] };
    }
}
```

### Adding a New CLI Option

1. Add field to `Cli` struct in `main.rs`:
```rust
#[derive(Parser)]
struct Cli {
    // ... existing fields

    #[arg(long, help = "New option description")]
    new_option: bool,
}
```

2. Handle in `main()` function:
```rust
if cli.new_option {
    // Handle new option
}
```

### Debugging Network Issues

Enable request logging:
```rust
// In scraper.rs, temporarily add
let response = self.client.get(url)
    .send()
    .await?;
eprintln!("Status: {}", response.status());
eprintln!("Headers: {:?}", response.headers());
```

Or use external tools:
```bash
# Monitor HTTP traffic
sudo tcpdump -i any -A 'tcp port 443'

# Test URL directly
curl -v "https://annas-archive.org/search?q=test"
```

## Important Gotchas

### 1. TUI Threading Model
**Issue:** Terminal drawing panics if called from background thread
**Solution:** Always use channel pattern - spawn task, send result via `mpsc`, update UI on main thread

```rust
// GOOD
let tx = tx.clone();
tokio::spawn(async move {
    let result = do_work().await?;
    tx.send(Command::Result(result)).await?;
});

// BAD - will panic!
tokio::spawn(async move {
    terminal.draw(|f| { /* ... */ })?;
});
```

### 2. Selector Brittleness
**Issue:** Anna's Archive changes CSS classes frequently
**Solution:** Always provide 3-5 fallback selectors, ordered by reliability

### 3. Async Reqwest in Sync Context
**Issue:** Using `reqwest::blocking` causes deadlocks in async code
**Solution:** Always use async `reqwest::Client` with `.await`

### 4. Path Handling on Windows
**Issue:** Windows paths use backslashes, can cause issues
**Solution:** Always use `PathBuf` and `Path`, let Rust handle platform differences

```rust
// GOOD
let path = PathBuf::from("downloads").join("book.pdf");

// BAD
let path = "downloads/book.pdf"; // Fails on Windows
```

### 5. Progress Bar Cleanup
**Issue:** Progress bars can leave artifacts in terminal
**Solution:** Always call `pb.finish_with_message()` or `pb.finish_and_clear()`

### 6. Regex Compilation in Const
**Issue:** `Regex::new()` can panic, not allowed in const
**Solution:** Compile in function with error handling

```rust
// GOOD
fn extract_year(text: &str) -> Option<String> {
    let re = regex::Regex::new(r"\b(19|20)\d{2}\b").ok()?;
    re.find(text).map(|m| m.as_str().to_string())
}

// BAD
const YEAR_RE: Regex = Regex::new(r"\b(19|20)\d{2}\b"); // Won't compile
```

## Performance Considerations

### Benchmarks (Observed)
- **Python/Selenium version**: ~15s search + ~30s download = ~45s total
- **Rust version**: ~2s search + ~5s download = ~7s total
- **Speedup**: ~6-7x overall, ~10x for search operations

### Memory Usage
- **Python/Selenium**: ~150MB (Chrome browser overhead)
- **Rust**: ~20MB (native HTTP, no browser)
- **Improvement**: ~7.5x reduction

### Optimization Tips
1. **Preallocate Vectors**: Use `.with_capacity()` when size known
2. **Stream Large Downloads**: Already implemented with `bytes_stream()`
3. **Connection Pooling**: Reuse `reqwest::Client` (already done)
4. **Parallel Searches**: Can spawn multiple search tasks concurrently

## Git Workflow

### Branches
- `main` - Stable release branch
- `claude/*` - AI-generated feature branches

### Commit Messages
Follow conventional commits format:
```
feat: add resume download capability
fix: correct filename extraction from Content-Disposition
docs: update CLAUDE.md with new patterns
refactor: simplify selector fallback logic
test: add tests for URL parsing
```

### Before Pushing
```bash
cargo fmt                    # Format code
cargo clippy                 # Check for issues
cargo test                   # Run tests
cargo build --release        # Ensure release builds
```

### Creating a PR
1. Create feature branch: `git checkout -b feature/description`
2. Make changes and commit
3. Push to remote: `git push -u origin feature/description`
4. Create PR via GitHub UI
5. Ensure CI passes (GitHub Actions)

## Future Improvements

### High Priority
- [ ] Resume interrupted downloads (HTTP Range headers)
- [ ] Search result caching (SQLite or JSON)
- [ ] Batch download queue (download multiple books)
- [ ] Filter by format/language/size in TUI
- [ ] Export search results (CSV, JSON, BibTeX)

### Medium Priority
- [ ] Proxy support for privacy
- [ ] Configurable timeouts in config file
- [ ] Download speed limiting (rate control)
- [ ] Multiple theme support (dark/light)
- [ ] Search history persistence

### Low Priority
- [ ] GUI mode using `egui` or `iced`
- [ ] Web API server mode (REST API)
- [ ] Calibre integration for format conversion
- [ ] Metadata editing before download
- [ ] Multi-language support (i18n)

## Troubleshooting Guide

### Build Errors

**"linker 'cc' not found"**
```bash
# Ubuntu/Debian
sudo apt install build-essential

# macOS
xcode-select --install

# Windows
# Install Visual Studio Build Tools
```

**"openssl not found"**
```bash
# Ubuntu/Debian
sudo apt install pkg-config libssl-dev

# macOS
brew install openssl
```

### Runtime Errors

**"Failed to fetch URL: connection timeout"**
- Check internet connection
- Anna's Archive may be blocking your IP
- Try again later or use a VPN

**"Permission denied" when downloading**
- Check download directory exists and is writable
- Try: `chmod +w ~/downloads`

**TUI rendering issues**
```bash
# Try forcing xterm compatibility
TERM=xterm-256color cargo run

# On Windows, use Windows Terminal, not cmd.exe
```

### Testing Network Issues

```bash
# Test Anna's Archive connectivity
curl -I https://annas-archive.org

# Test with custom user agent
curl -A "Mozilla/5.0" https://annas-archive.org/search?q=test
```

## Related Files

- **README.md** - User-facing documentation and usage examples
- **AGENTS.md** - Detailed Rust development patterns and architecture
- **WARP.md** - Historical context (Python version info)
- **.github/workflows/rust.yml** - CI/CD pipeline configuration
- **Cargo.toml** - Dependencies and build configuration

## Quick Reference

### File Locations
| Purpose | File | Lines |
|---------|------|-------|
| CLI entry | `src/main.rs` | ~200 |
| Config | `src/config.rs` | ~70 |
| HTTP/parsing | `src/scraper.rs` | ~350 |
| Downloads | `src/downloader.rs` | ~200 |
| TUI logic | `src/ui/app.rs` | ~770 |
| Dependencies | `Cargo.toml` | ~60 |

### Key Commands
```bash
cargo build --release        # Build optimized binary
cargo run -- "query"        # Non-interactive search
cargo run                   # Launch TUI
cargo test                  # Run tests
cargo clippy               # Lint code
cargo fmt                  # Format code
```

### Important URLs
- Anna's Archive: https://annas-archive.org
- Repository: https://github.com/Nquxii/anna-dl
- Rust Docs: https://doc.rust-lang.org
- Ratatui Docs: https://ratatui.rs
- Tokio Docs: https://tokio.rs

## Contact & Support

For issues or questions:
1. Check this CLAUDE.md file
2. Review AGENTS.md for detailed patterns
3. Check README.md for user documentation
4. Search existing GitHub issues
5. Create new issue with minimal reproduction

---

**Last Updated**: 2025-11-27
**Version**: 0.1.0
**Rust Edition**: 2021
