# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Overview
anna-dl is a Python CLI tool for downloading books from Anna's Archive (annas-archive.org). It uses Selenium with headless Chrome to automate search, metadata extraction, and downloads via LibGen mirrors.

## Development Setup
```bash
# Install dependencies
pip3 install -r requirements.in

# Run the tool
python3 annadl --s "search query"
```

## Common Commands
```bash
# Search with default 5 results
python3 annadl --s "Book Title"

# Search with custom result count (0 = all results on page)
python3 annadl --s "Book Title" --n 10

# Specify download path
python3 annadl /path/to/downloads --s "Book Title"

# Show help
python3 annadl --h
```

## Architecture
The project is a single-file Python script (`annadl`) with no extension. Key components:

1. **Configuration**: Reads `config.json` for default `download_path`. Falls back to `./assets/` if not configured.

2. **CLI Arguments** (argparse):
   - `path` (positional, optional): Download destination
   - `--s` (required): Search query
   - `--n` (default: 5): Number of results to display

3. **Selenium Automation Flow**:
   - Searches annas-archive.org and extracts book metadata (title, author, year, language, format, size)
   - User selects a result by number
   - Navigates to book detail page
   - Attempts LibGen download; falls back to showing manual download links if unavailable

4. **ChromeDriver Management**: Uses `webdriver_manager` to auto-install ChromeDriver, with fallback logic to handle path resolution issues.

## Dependencies
- `selenium>=4.8.2` - Browser automation
- `webdriver_manager>=3.8.2` - ChromeDriver auto-management

## Key Functions
- `dlwait(path)`: Monitors download directory for `.crdownload` files to detect completion

## Notes
- Requires Chrome browser installed on the system
- Runs in headless mode by default
- The `assets/` folder is the default download location and is gitignored (except for `.gitignore`)
