# AGENTS.md - Rust Development Guide

## Project Overview

**Rust Rewrite of anna-dl**
- **Objective**: Replace Python Selenium tool with pure Rust CLI using reqwest for direct HTTP
- **Key Advantage**: No Chrome dependency, ~10x faster, native terminal UI with ratatui
- **Architecture**: Async tokio runtime with modular components

## Essential Commands

```bash
# Development
cargo check                    # Quick type check
cargo clippy                  # Lint and suggestions
cargo fmt                     # Format code
cargo test                    # Run tests
cargo build --release         # Production binary

# Running
cargo run                     # Interactive mode
cargo run -- "book title"    # Non-interactive
cargo run -- --help          # Show help

# Installing locally
cargo install --path .
```

## Architecture & Code Organization

```
src/
├── main.rs          # CLI entry + tokio async runtime
├── config.rs        # JSON config persistence (dirs crate)
├── scraper.rs       # HTTP + HTML parsing (reqwest + scraper)
├── downloader.rs    # Async download streams + indicatif progress
└── ui/
    ├── mod.rs      # UI module exports
    └── app.rs      # Main TUI with ratatui + crossterm
```

### Module Responsibilities

**`main.rs` (200 lines)**
- CLI parsing with clap derive
- tokio runtime initialization
- Terminal setup/cleanup (crossterm)
- Two modes: TUI vs non-interactive

**`scraper.rs` (350 lines)**
- **Core**: Direct HTTP requests (no Selenium!)
- **Flow**: reqwest → scraper (HTML parsing) → regex extraction
- **Key Function**: `search(query, max_results) → Result<Vec<Book>>`
- **Fallback system**: Array of CSS selectors + XPath patterns
- **User-Agents**: Rotated automatically via `fake-user-agent`

**`downloader.rs` (200 lines)**
- **Async Streaming**: `response.bytes_stream()` + tokio::fs::File
- **Progress**: indicatif ProgressBar with ETA
- **Filename Detection**: URL + Content-Disposition + fallback
- **Partial Downloads**: .part/.crdownload cleanup

**`ui/app.rs` (500 lines)**
- **Pattern**: Stateful TUI with `AppMode` enum
- **Navigation**: Event loop in `run_app()` → `handle_keypress()` → `draw()`
- **Screens**: Search → Results → DownloadSelection → Downloading → Error → Help
- **Key Maps**: vim-style (k/j), arrows, Enter, Esc, F1, Ctrl+C
- **Scrolling**: Custom logic with results_scroll offset

### Async Flow Pattern

**Command Channel Pattern:**
```rust
// UI thread → spawn task → send Command back → UI processes
enum AppCommand {
    Search(String, usize),
    FetchDownloadLinks(String),
    Download(String, usize),
    ShowError(String),
    CompleteDownload(PathBuf),
}

// In main loop:
if let Ok(cmd) = rx.try_recv() { match cmd { ... } }
```

**Search Flow:**
1. User types + press Enter in Search mode
2. `perform_search()` → spawns async task
3. Task: `AnnaScraper::search()` → sends Results
4. UI: Receives command → switches to Results mode

**Download Flow:**
1. User selects book → spawns `fetch_download_links()`
2. Gets links → switches to DownloadSelection
3. User picks link → spawns `perform_download()`
4. `Downloader::download()` streams with progress
5. Sends CompleteDownload → returns to Search

## Error Handling Strategy

**Pattern**: `anyhow::Result<T>` everywhere + `thiserror` for domain errors

```rust
// In scraper/downloader: ? operator propagates
let html = self.client.get(url).send().await?.text().await?;

// In main/UI:
match scraper.search().await {
    Ok(books) => { /* update state */ }
    Err(e) => { app.mode = Error(e.to_string()) }
}
```

**User-Facing Errors**:
- Show in Error screen with red styling
- Include action: "Press ESC to return to search"
- Log with tracing (if enabled in debug builds)

## Code Patterns

### Selector Fallbacks (scraper.rs)
```rust
let selectors = [
    "a.js-vim-focus.custom-a",
    "a[href*='md5']",
    ".book-title a",
];
for selector_str in &selectors {
    if let Ok(selector) = Selector::parse(selector_str) {
        let elements: Vec<_> = document.select(&selector).collect();
        if !elements.is_empty() { return elements; }
    }
}
```

### Regex Extraction
```rust
// Compile once in fn (not const to handle errors gracefully)
let re = regex::Regex::new(r"\b(19|20)\d{2}\b").ok()?;
re.find(text).map(|m| m.as_str().to_string())
```

### TUI State Management
```rust
// AppMode enum drives drawing + input handling
match app.mode {
    Search => draw_search(f),
    Results => draw_results(f),
    Error(ref msg) => draw_error(f, msg),
}
```

### Progress Bars
```rust
let pb = ProgressBar::new(total_size);
pb.set_style(ProgressStyle::default_bar()
    .template("{spinner} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec})")
    .unwrap());

while let Some(chunk) = stream.next().await {
    file.write_all(&chunk).await?;
    pb.inc(chunk.len() as u64);
}
```

## Dependencies & Crates

**Core Stack**:
- `reqwest` - HTTP client (rustls-tls)
- `scraper` - HTML parsing with CSS selectors
- `tokio` - Async runtime (full features)
- `ratatui` - Terminal UI framework
- `crossterm` - Lower-level terminal manipulation
- `clap` - CLI arg parsing (derive style)
- `serde` + `serde_json` - Config serialization
- `anyhow`/`thiserror` - Error handling
- `indicatif` - Progress bars
- `dirs` - Config directory discovery
- `regex` - Metadata extraction
- `urlencoding` - URL encoding/decoding
- `fake-user-agent` + `rand` - Request randomization

**Important**: No `chromedriver`, no `selenium`, no GUI dependencies.

## Configuration System

**Location**: `dirs::config_dir()` → `anna-dl/config.json`

**Priority**: CLI arg `--download-path` > config file > `./assets/`

**Structure**:
```rust
#[derive(Serialize, Deserialize, Default)]
struct Config {
    download_path: Option<PathBuf>,
}
```

**Commands**:
```bash
annadl --set-path /home/user/books    # Updates config
annadl --config                         # Shows current config
```

## Testing Strategy

**No Integration Tests** (reason: live website changes too often)

**Unit Tests Only** (in downloader.rs):
```rust
#[test]
fn test_extract_filename_from_url() { ... }
#[test]
fn test_parse_content_disposition() { ... }
```

**Manual Testing Required**:
- Run `cargo run` → Verify TUI launches
- Search "test" → Check results display
- Select book → Verify download links fetch
- Download → Verify progress bar, file saved
- Test config: `--set-path` and `--config`

## Deployment & Distribution

**Binary Details**:
```bash
cargo build --release    # Optimized binary (~10MB)
ls -lh target/release/annadl
```

**Optimization Flags** (in Cargo.toml):
```toml
[profile.release]
opt-level = "z"        # Optimize for size
lto = true             # Link-time optimization
codegen-units = 1      # Single codegen unit
strip = true           # Strip symbols
```

**Distribution**: 
- GitHub releases with pre-built binaries
- Homebrew formula (macOS/Linux)
- Scoop/Chocolatey (Windows)

## Critical Implementation Notes

**Live Website Issues**:
- CSS selectors WILL break when Anna's Archive changes UI
- Solution: Provide 3-5 fallback selectors per element
- Maintain selector arrays in `scraper.rs`

**Rate Limiting**:
- Currently: random user-agent rotation
- Future: Add sleep delays, exponential backoff
- Consider: Request throttling per domain

**Partial Downloads**:
- `.part` and `.crdownload` files auto-cleaned on startup
- TODO: Implement resume capability (Range headers)

**Error Recovery**:
- Network errors → Show error screen → allow retry
- Parsing errors → Try next selector → show warning
- Download errors → Try next link → log error

## Performance Considerations

**Async Patterns**:
- DON'T: Block on reqwest in sync context
- DO: Use tokio::spawn for parallel downloads
- DO: Use channels for UI updates from tasks

**Memory**:
- Large HTML pages: stream with bytes_stream() (already done)
- Download chunks: 8KB buffer size implicit in tokio
- Vec growth: pre-allocate with `.with_capacity()` if known

**Benchmarks** (observed):
- Python Selenium: ~15s per search + 30s per download
- Rust reqwest: ~2s per search + 5s per download

## Gotchas

**TUI Threading**:
- Terminal draw() MUST happen on main thread
- Heavy ops (download, scrape) MUST be on tokio tasks
- Channel pattern bridges the gap

**Reqwest Blocking**:
- DON'T use `reqwest::blocking::*` - deadlocks TUI
- DO use async `.await` everywhere

**Path Handling**:
- Use `std::path::PathBuf` (not Path)
- Convert early: `PathBuf::from()` not `Path::new()`
- Windows: forward slashes work, but check drive letters

**Regex Compilation**:
- DON'T compile in const (panics on error)
- DO: `regex::Regex::new().ok()?` with error fallback

## Future Improvements

**High Priority**:
- [ ] Resume interrupted downloads (Range headers)
- [ ] Search result caching (SQLite)
- [ ] Batch downloads (multiple books)
- [x] Filter by format/language/size
- [ ] Export to CSV/JSON/BibTeX

**Medium Priority**:
- [ ] Proxy support for privacy
- [ ] Configurable timeout settings
- [ ] Download speed limiting
- [ ] Dark mode TUI theme
- [ ] Search history

**Low Priority**:
- [ ] GUI mode (egui)
- [ ] Web API server mode
- [ ] Automatic format conversion (Calibre integration)
- [ ] Metadata editing

## Related Files

- `../README.md` - User documentation
- `../WARP.md` (if exists) - Project history/context
- `.github/workflows/` - CI/CD (when ready)

## Key Differences from Python Version

| Aspect | Python | Rust |
|--------|--------|------|
| Chrome | Required | Not needed (reqwest) |
| Performance | ~45s total | ~7s total |
| Memory | ~150MB | ~20MB |
| Binary size | Script + deps | ~10MB single binary |
| Startup | 2-3s | Instant |
| UI | Terminal output | Rich TUI |
| Async | No (sync selenium) | Yes (tokio) |
| Type safety | Runtime | Compile-time |
| Error handling | Try/except | Result<T, E> |

When coding: Prefer Result combinators (.map, .and_then) over unwrap(), use ? for early returns, and maintain the channel pattern for UI/background communication.
