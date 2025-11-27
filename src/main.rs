mod config;
mod downloader;
mod scraper;
mod ui;

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "annadl")]
#[command(about = "A Rust CLI tool for downloading books from Anna's Archive", long_about = None)]
#[command(version)]
struct Cli {
    search_query: Option<String>,
    
    #[arg(short = 'n', long, default_value = "5", help = "Number of results to show")]
    num_results: usize,
    
    #[arg(short = 'p', long, help = "Download path (overrides config)")]
    download_path: Option<PathBuf>,
    
    #[arg(long, help = "Set default download path in config")]
    set_path: Option<PathBuf>,
    
    #[arg(short = 'i', long, help = "Interactive mode (default if no query provided)")]
    interactive: bool,
    
    #[arg(long, help = "List current config")]
    config: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    let mut config = config::Config::load()
        .context("Failed to load configuration")?;
    
    if cli.config {
        println!("Current configuration:");
        println!("  Download path: {}", 
            config.download_path.as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "Not set (uses ./assets)".to_string())
        );
        return Ok(());
    }
    
    if let Some(path) = cli.set_path {
        config.set_download_path(path)?;
        println!("Download path updated successfully!");
        return Ok(());
    }
    
    let download_path = config.download_path(cli.download_path.clone());
    
    if let Some(query) = cli.search_query {
        if cli.interactive {
            run_tui(config, download_path).await?;
        } else {
            run_non_interactive(query, cli.num_results, download_path).await?;
        }
    } else {
        // No query provided, run TUI
        run_tui(config, download_path).await?;
    }
    
    Ok(())
}

async fn run_tui(config: config::Config, download_path: PathBuf) -> Result<()> {
    setup_terminal()?;
    
    let result = run_app(config, download_path).await;
    
    restore_terminal()?;
    
    result
}

async fn run_app(config: config::Config, download_path: PathBuf) -> Result<()> {
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    
    let mut app = ui::App::new(config, download_path);
    
    // Process commands in background
    let mut command_rx = {
        let app = &mut app;
        std::mem::replace(
            &mut app.command_rx,
            tokio::sync::mpsc::unbounded_channel().1,
        )
    };
    
    // Main loop
    loop {
        terminal.draw(|f| app.draw(f))?;
        
        // Check for commands
        if let Ok(command) = command_rx.try_recv() {
            match command {
                ui::AppCommand::Search(query, num_results) => {
                    let scraper = scraper::AnnaScraper::new()?;
                    match scraper.search(&query, num_results).await {
                        Ok(books) => {
                            app.books = books;
                            app.mode = ui::AppMode::Results;
                            app.selected_book_index = 0;
                        }
                        Err(e) => {
                            app.error_message = format!("Search error: {}", e);
                            app.mode = ui::AppMode::Error(app.error_message.clone());
                        }
                    }
                }
                ui::AppCommand::FetchDownloadLinks(book_url) => {
                    let scraper = scraper::AnnaScraper::new()?;
                    match scraper.get_book_details(&book_url).await {
                        Ok(links) => {
                            app.download_links = links;
                            app.mode = ui::AppMode::DownloadSelection;
                            app.download_link_index = 0;
                        }
                        Err(e) => {
                            app.error_message = format!("Error fetching links: {}", e);
                            app.mode = ui::AppMode::Error(app.error_message.clone());
                        }
                    }
                }
                ui::AppCommand::Download(url, _link_index) => {
                    let downloader = downloader::Downloader::new(app.download_path.clone())?;
                    match downloader.download(&url, None).await {
                        Ok(path) => {
                            app.downloading_message = format!("Download complete: {}", path.display());
                            app.mode = ui::AppMode::Search;
                            app.query.clear();
                            app.books.clear();
                            app.download_links.clear();
                        }
                        Err(e) => {
                            app.error_message = format!("Download failed: {}", e);
                            app.mode = ui::AppMode::Error(app.error_message.clone());
                        }
                    }
                }
                ui::AppCommand::ShowError(msg) => {
                    app.error_message = msg;
                    app.mode = ui::AppMode::Error(app.error_message.clone());
                }
                ui::AppCommand::CompleteDownload(path) => {
                    app.downloading_message = format!("✓ Downloaded to: {}", path.display());
                    app.mode = ui::AppMode::Search;
                }
            }
        }
        
        // Handle input
        if let Event::Key(key) = crossterm::event::read()? {
            match app.handle_keypress(key).await? {
                ui::ControlFlow::Exit => break,
                ui::ControlFlow::Continue => continue,
            }
        }
    }
    
    Ok(())
}

fn setup_terminal() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    Ok(())
}

fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

async fn run_non_interactive(query: String, num_results: usize, download_path: PathBuf) -> Result<()> {
    println!("🔍 Searching for: {}", query);
    
    let scraper = scraper::AnnaScraper::new()
        .context("Failed to create scraper")?;
    
    let books = scraper.search(&query, num_results)
        .await
        .context("Search failed")?;
    
    if books.is_empty() {
        println!("❌ No results found");
        return Ok(());
    }
    
    println!("\n📚 Found {} results:\n", books.len());
    
    for (i, book) in books.iter().enumerate() {
        println!("  {}. {}", i + 1, book.title);
        println!("     Author: {}", book.author.as_deref().unwrap_or("Unknown"));
        println!("     Year: {} | Language: {} | Format: {} | Size: {}",
            book.year.as_deref().unwrap_or("Unknown"),
            book.language.as_deref().unwrap_or("Unknown"),
            book.format.as_deref().unwrap_or("Unknown"),
            book.size.as_deref().unwrap_or("Unknown")
        );
        println!();
    }
    
    println!("Select a book to download (1-{}), or press Ctrl+C to cancel:", books.len());
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    let selection: usize = input.trim().parse()
        .context("Invalid selection")?;
    
    if selection < 1 || selection > books.len() {
        anyhow::bail!("Selection out of range");
    }
    
    let selected_book = &books[selection - 1];
    println!("\n🔗 Fetching download links for '{}'...", selected_book.title);
    
    let download_links = scraper.get_book_details(&selected_book.url)
        .await
        .context("Failed to fetch download links")?;
    
    if download_links.is_empty() {
        println!("❌ No download links found");
        return Ok(());
    }
    
    println!("\n📥 Available download links:\n");
    
    for (i, link) in download_links.iter().enumerate() {
        println!("  {}. {}", i + 1, link.text);
        println!("     Source: {} | URL: {}", link.source, &link.url[..50.min(link.url.len())]);
    }
    
    // Try to auto-select LibGen link
    let selected_link = download_links.iter()
        .find(|l| l.text.to_lowercase().contains("libgen"))
        .or_else(|| download_links.first())
        .ok_or_else(|| anyhow::anyhow!("No download link available"))?;
    
    println!("\n⬇️  Downloading from: {}...", selected_link.text);
    
    let downloader = downloader::Downloader::new(download_path)
        .context("Failed to create downloader")?;
    
    let filename = format!(
        "{} - {}",
        selected_book.title.chars().take(50).collect::<String>(),
        selected_book.author.as_deref().unwrap_or("Unknown")
    );
    
    let path = downloader.download(&selected_link.url, Some(&filename))
        .await
        .context("Download failed")?;
    
    println!("\n✅ Download complete: {}", path.display());
    
    Ok(())
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn test_cli_parse_no_args() {
        let cli = Cli::try_parse_from(&["annadl"]).unwrap();
        assert!(cli.search_query.is_none());
        assert_eq!(cli.num_results, 5);
        assert!(!cli.interactive);
        assert!(!cli.config);
    }

    #[test]
    fn test_cli_parse_search_query() {
        let cli = Cli::try_parse_from(&["annadl", "rust programming"]).unwrap();
        assert_eq!(cli.search_query, Some("rust programming".to_string()));
        assert_eq!(cli.num_results, 5);
    }

    #[test]
    fn test_cli_parse_num_results_short() {
        let cli = Cli::try_parse_from(&["annadl", "test", "-n", "10"]).unwrap();
        assert_eq!(cli.num_results, 10);
    }

    #[test]
    fn test_cli_parse_num_results_long() {
        let cli = Cli::try_parse_from(&["annadl", "test", "--num-results", "20"]).unwrap();
        assert_eq!(cli.num_results, 20);
    }

    #[test]
    fn test_cli_parse_download_path_short() {
        let cli = Cli::try_parse_from(&["annadl", "-p", "/tmp/books"]).unwrap();
        assert_eq!(cli.download_path, Some(PathBuf::from("/tmp/books")));
    }

    #[test]
    fn test_cli_parse_download_path_long() {
        let cli = Cli::try_parse_from(&["annadl", "--download-path", "/home/user/downloads"]).unwrap();
        assert_eq!(cli.download_path, Some(PathBuf::from("/home/user/downloads")));
    }

    #[test]
    fn test_cli_parse_set_path() {
        let cli = Cli::try_parse_from(&["annadl", "--set-path", "/new/path"]).unwrap();
        assert_eq!(cli.set_path, Some(PathBuf::from("/new/path")));
    }

    #[test]
    fn test_cli_parse_interactive_short() {
        let cli = Cli::try_parse_from(&["annadl", "-i"]).unwrap();
        assert!(cli.interactive);
    }

    #[test]
    fn test_cli_parse_interactive_long() {
        let cli = Cli::try_parse_from(&["annadl", "--interactive"]).unwrap();
        assert!(cli.interactive);
    }

    #[test]
    fn test_cli_parse_config_flag() {
        let cli = Cli::try_parse_from(&["annadl", "--config"]).unwrap();
        assert!(cli.config);
    }

    #[test]
    fn test_cli_parse_combined_flags() {
        let cli = Cli::try_parse_from(&[
            "annadl",
            "rust book",
            "-n", "15",
            "-p", "/downloads",
            "-i"
        ]).unwrap();

        assert_eq!(cli.search_query, Some("rust book".to_string()));
        assert_eq!(cli.num_results, 15);
        assert_eq!(cli.download_path, Some(PathBuf::from("/downloads")));
        assert!(cli.interactive);
    }

    #[test]
    fn test_cli_version_info() {
        let cmd = Cli::command();
        assert!(cmd.get_name() == "annadl");
        assert!(cmd.get_version().is_some());
    }

    #[test]
    fn test_cli_help_message() {
        let cmd = Cli::command();
        let about = cmd.get_about();
        assert!(about.is_some());
    }

    #[test]
    fn test_cli_invalid_num_results() {
        let result = Cli::try_parse_from(&["annadl", "-n", "not-a-number"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cli_default_num_results() {
        let cli = Cli::try_parse_from(&["annadl"]).unwrap();
        assert_eq!(cli.num_results, 5); // Default value
    }
}
