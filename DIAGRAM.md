# Architecture and Test Coverage

```mermaid
graph TD
    User((User))
    CLI[CLI (Cobra)]
    TUI[TUI (Bubble Tea)]
    Scraper[Scraper (GoQuery)]
    Downloader[Downloader]
    Config[Config]
    FS[(File System)]
    Web((Anna's Archive))

    User --> CLI
    CLI --> TUI
    CLI --> Config
    TUI --> Scraper
    TUI --> Downloader
    TUI --> Config

    Scraper -->|HTTP GET| Web
    Downloader -->|HTTP GET| Web
    Downloader -->|Write| FS
    Config -->|Read/Write| FS

    subgraph Tests
        ST[Scraper Tests]
        DT[Downloader Tests]
        CT[Config Tests]

        ST -.->|Mock| Web
        ST --> Scraper

        DT -.->|Mock| Web
        DT --> Downloader

        CT --> Config
        CT -.->|Temp Dir| FS
    end
```

## Implementation Details

### Scraper
The `scraper` package handles communication with Anna's Archive.
- **Tests**: Mocked HTTP server simulates search results and download pages to verify parsing logic (CSS selectors, regex extraction).
- **Improvements**: Made `BaseURL` configurable for testing. Fixed bugs in `extractAuthor`, `extractLanguage`, and `parseSearchResults`.

### Downloader
The `downloader` package manages file downloads.
- **Tests**: Verified filename sanitization, Content-Disposition parsing, and file content verification using a mock server.
- **Improvements**: Added preference for Content-Disposition filename over URL filename.

### Config
The `config` package manages user configuration.
- **Tests**: Verified load/save flows.
- **Improvements**: Added `ConfigPathOverride` to allow safe testing without touching user's home directory.

### UI / CLI
The UI interacts with these components via `tea.Cmd`.
- **Note**: `ui` tests are currently manual due to TUI complexity, but the underlying business logic in `scraper` and `downloader` is now fully covered.
