use crate::config::Config;
use crate::scraper::{Book, DownloadLink};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame, Terminal,
};
use std::io;
use std::path::PathBuf;
use tokio::sync::mpsc;

pub enum AppMode {
    Search,
    Results,
    DownloadSelection,
    Downloading,
    Error(String),
    Help,
}

pub struct App {
    pub config: Config,
    pub mode: AppMode,
    pub query: String,
    pub books: Vec<Book>,
    pub selected_book_index: usize,
    pub download_links: Vec<DownloadLink>,
    pub download_link_index: usize,
    pub download_path: PathBuf,
    pub error_message: String,
    pub results_scroll: usize,
    pub help_scroll: usize,
    pub command_tx: mpsc::UnboundedSender<AppCommand>,
    pub command_rx: mpsc::UnboundedReceiver<AppCommand>,
    pub downloading_message: String,
}

#[derive(Debug, Clone)]
pub enum AppCommand {
    Search(String, usize),
    FetchDownloadLinks(String),
    Download(String, usize),
}

impl App {
    pub fn new(config: Config, download_path: PathBuf) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        
        Self {
            config,
            mode: AppMode::Search,
            query: String::new(),
            books: Vec::new(),
            selected_book_index: 0,
            download_links: Vec::new(),
            download_link_index: 0,
            download_path,
            error_message: String::new(),
            results_scroll: 0,
            help_scroll: 0,
            command_tx: tx,
            command_rx: rx,
            downloading_message: String::new(),
        }
    }

    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            terminal.draw(|f| self.draw(f))?;

            if let Event::Key(key) = event::read()? {
                match self.handle_keypress(key).await? {
                    ControlFlow::Continue => continue,
                    ControlFlow::Exit => return Ok(()),
                }
            }
        }
    }

    pub async fn handle_keypress(&mut self, key: KeyEvent) -> Result<ControlFlow> {
        match self.mode {
            AppMode::Search => self.handle_search_input(key).await,
            AppMode::Results => self.handle_results_navigation(key).await,
            AppMode::DownloadSelection => self.handle_download_selection(key).await,
            AppMode::Error(_) => self.handle_error(key).await,
            AppMode::Downloading => self.handle_downloading(key).await,
            AppMode::Help => self.handle_help(key).await,
        }
    }

    async fn handle_search_input(&mut self, key: KeyEvent) -> Result<ControlFlow> {
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(ControlFlow::Exit);
            }
            KeyCode::Enter => {
                if !self.query.is_empty() {
                    self.perform_search().await?;
                }
            }
            KeyCode::Char(c) => {
                self.query.push(c);
            }
            KeyCode::Backspace => {
                self.query.pop();
            }
            KeyCode::Esc => {
                return Ok(ControlFlow::Exit);
            }
            KeyCode::F(1) => {
                self.mode = AppMode::Help;
            }
            _ => {}
        }
        Ok(ControlFlow::Continue)
    }

    async fn handle_results_navigation(&mut self, key: KeyEvent) -> Result<ControlFlow> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_book_index < self.books.len().saturating_sub(1) {
                    self.selected_book_index += 1;
                    if self.selected_book_index >= self.results_scroll + 10 {
                        self.results_scroll += 1;
                    }
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_book_index > 0 {
                    self.selected_book_index = self.selected_book_index.saturating_sub(1);
                    if self.selected_book_index < self.results_scroll {
                        self.results_scroll = self.selected_book_index;
                    }
                }
            }
            KeyCode::Enter => {
                if !self.books.is_empty() {
                    self.fetch_download_links().await?;
                }
            }
            KeyCode::Esc => {
                self.mode = AppMode::Search;
                self.query.clear();
                self.books.clear();
                self.selected_book_index = 0;
                self.results_scroll = 0;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(ControlFlow::Exit);
            }
            KeyCode::F(1) => {
                self.mode = AppMode::Help;
            }
            _ => {}
        }
        Ok(ControlFlow::Continue)
    }

    async fn handle_download_selection(&mut self, key: KeyEvent) -> Result<ControlFlow> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                if self.download_link_index < self.download_links.len().saturating_sub(1) {
                    self.download_link_index += 1;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.download_link_index = self.download_link_index.saturating_sub(1);
            }
            KeyCode::Enter => {
                if !self.download_links.is_empty() {
                    self.perform_download().await?;
                }
            }
            KeyCode::Esc => {
                self.mode = AppMode::Results;
                self.download_links.clear();
                self.download_link_index = 0;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(ControlFlow::Exit);
            }
            KeyCode::F(1) => {
                self.mode = AppMode::Help;
            }
            _ => {}
        }
        Ok(ControlFlow::Continue)
    }

    async fn handle_error(&mut self, key: KeyEvent) -> Result<ControlFlow> {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.mode = AppMode::Search;
                self.error_message.clear();
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(ControlFlow::Exit);
            }
            _ => {}
        }
        Ok(ControlFlow::Continue)
    }

    async fn handle_downloading(&mut self, key: KeyEvent) -> Result<ControlFlow> {
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(ControlFlow::Exit);
            }
            _ => {}
        }
        Ok(ControlFlow::Continue)
    }

    async fn handle_help(&mut self, key: KeyEvent) -> Result<ControlFlow> {
        match key.code {
            KeyCode::Esc | KeyCode::F(1) => {
                self.mode = AppMode::Search;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.help_scroll += 1;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.help_scroll = self.help_scroll.saturating_sub(1);
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(ControlFlow::Exit);
            }
            _ => {}
        }
        Ok(ControlFlow::Continue)
    }

    pub fn draw(&mut self, f: &mut Frame) {
        match &self.mode {
            AppMode::Search => self.draw_search(f),
            AppMode::Results => self.draw_results(f),
            AppMode::DownloadSelection => self.draw_download_selection(f),
            AppMode::Error(msg) => self.draw_error(f, msg),
            AppMode::Downloading => self.draw_downloading(f),
            AppMode::Help => self.draw_help(f),
        }
    }

    fn draw_search(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(f.size());

        let title = Paragraph::new("Anna's Archive Downloader")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center);
        f.render_widget(title, chunks[0]);

        let input = Paragraph::new(self.query.as_str())
            .block(Block::default().borders(Borders::ALL).title("Search Query (Press Enter to search, Ctrl+C to quit, F1 for Help)"))
            .style(Style::default().fg(Color::White));
        f.render_widget(input, chunks[1]);
    }

    fn draw_results(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .split(f.size());

        let header = Paragraph::new(format!("Search Results for: {}", self.query))
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center);
        f.render_widget(header, chunks[0]);

        let results_area = chunks[1];
        let items: Vec<ListItem> = self.books.iter()
            .skip(self.results_scroll)
            .take(10)
            .enumerate()
            .map(|(i, book)| {
                let real_index = self.results_scroll + i;
                let style = if real_index == self.selected_book_index {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                let lines = vec![
                    Line::from(vec![
                        Span::styled(format!("{}. ", real_index + 1), style),
                        Span::styled(&book.title, style.add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(vec![
                        Span::raw("  Author: "),
                        Span::raw(book.author.as_deref().unwrap_or("Unknown")),
                    ]),
                    Line::from(vec![
                        Span::raw("  Year: "),
                        Span::raw(book.year.as_deref().unwrap_or("Unknown")),
                        Span::raw(" | Language: "),
                        Span::raw(book.language.as_deref().unwrap_or("Unknown")),
                        Span::raw(" | Format: "),
                        Span::raw(book.format.as_deref().unwrap_or("Unknown")),
                        Span::raw(" | Size: "),
                        Span::raw(book.size.as_deref().unwrap_or("Unknown")),
                    ]),
                    Line::from(""),
                ];

                ListItem::new(Text::from(lines))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Books (k/j or ↑/↓ to navigate, Enter to select, Esc to go back, F1 for Help)"))
            .highlight_style(Style::default().bg(Color::DarkGray));
        
        let mut list_state = ListState::default();
        list_state.select(Some(self.selected_book_index.saturating_sub(self.results_scroll)));
        f.render_stateful_widget(list, results_area, &mut list_state);

        let footer_text = format!(
            "Showing {} of {} books | Press Enter to see download options",
            self.books.len().min(self.results_scroll + 10) - self.results_scroll,
            self.books.len()
        );
        let footer = Paragraph::new(footer_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        f.render_widget(footer, chunks[2]);
    }

    fn draw_download_selection(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),
                Constraint::Min(10),
            ])
            .split(f.size());

        let book = &self.books[self.selected_book_index];
        let book_info = vec![
            Line::from(vec![Span::raw("Title: "), Span::styled(&book.title, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))]),
            Line::from(vec![Span::raw("Author: "), Span::raw(book.author.as_deref().unwrap_or("Unknown"))]),
            Line::from(vec![Span::raw("Year: "), Span::raw(book.year.as_deref().unwrap_or("Unknown"))]),
            Line::from(vec![Span::raw("Language: "), Span::raw(book.language.as_deref().unwrap_or("Unknown"))]),
            Line::from(vec![Span::raw("Format: "), Span::raw(book.format.as_deref().unwrap_or("Unknown"))]),
            Line::from(vec![Span::raw("Size: "), Span::raw(book.size.as_deref().unwrap_or("Unknown"))]),
        ];

        let info_panel = Paragraph::new(Text::from(book_info))
            .block(Block::default().borders(Borders::ALL).title("Book Info"))
            .style(Style::default().fg(Color::White));
        f.render_widget(info_panel, chunks[0]);

        let items: Vec<ListItem> = self.download_links.iter()
            .enumerate()
            .map(|(i, link)| {
                let style = if i == self.download_link_index {
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                let lines = vec![
                    Line::from(vec![
                        Span::styled(format!("{}. ", i + 1), style),
                        Span::styled(&link.text, style),
                    ]),
                    Line::from(vec![
                        Span::raw("  Source: "),
                        Span::raw(&link.source),
                        Span::raw(" | URL: "),
                        Span::raw(&link.url[..50.min(link.url.len())]),
                    ]),
                    Line::from(""),
                ];

                ListItem::new(Text::from(lines))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Download Links (k/j to navigate, Enter to download, Esc to go back)"))
            .highlight_style(Style::default().bg(Color::DarkGray));
        f.render_widget(list, chunks[1]);
    }

    fn draw_error(&self, f: &mut Frame, error: &str) {
        let block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Red));

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40),
                Constraint::Percentage(20),
                Constraint::Percentage(40),
            ])
            .split(f.size());

        let error_text = vec![
            Line::from(""),
            Line::from(Span::styled("ERROR", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(error),
            Line::from(""),
            Line::from("Press ESC or Enter to return to search"),
        ];

        let error_paragraph = Paragraph::new(Text::from(error_text))
            .block(block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        f.render_widget(error_paragraph, chunks[1]);
    }

    fn draw_downloading(&self, f: &mut Frame) {
        let block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Yellow))
            .title("Downloading");

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40),
                Constraint::Percentage(20),
                Constraint::Percentage(40),
            ])
            .split(f.size());

        let status = vec![
            Line::from(""),
            Line::from(Span::styled(self.downloading_message.as_str(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from("Download in progress..."),
            Line::from(""),
            Line::from("Press Ctrl+C to force quit"),
        ];

        let status_paragraph = Paragraph::new(Text::from(status))
            .block(block)
            .alignment(Alignment::Center);
        f.render_widget(status_paragraph, chunks[1]);
    }

    fn draw_help(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(f.size());

        let title = Paragraph::new("Help - Anna's Archive Downloader")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center);
        f.render_widget(title, chunks[0]);

        let help_text = vec![
            Line::from(""),
            Line::from(vec![Span::raw("• Search Mode: "), Span::styled("Type to search", Style::default().fg(Color::Green))]),
            Line::from(vec![Span::raw("• Navigate Results: "), Span::styled("↑/↓ or k/j", Style::default().fg(Color::Green))]),
            Line::from(vec![Span::raw("• Select Book: "), Span::styled("Enter", Style::default().fg(Color::Green))]),
            Line::from(vec![Span::raw("• Select Download: "), Span::styled("Enter", Style::default().fg(Color::Green))]),
            Line::from(vec![Span::raw("• Go Back: "), Span::styled("Esc", Style::default().fg(Color::Green))]),
            Line::from(vec![Span::raw("• Help: "), Span::styled("F1", Style::default().fg(Color::Green))]),
            Line::from(vec![Span::raw("• Quit: "), Span::styled("Ctrl+C", Style::default().fg(Color::Red))]),
            Line::from(""),
            Line::from(vec![Span::raw("Key Bindings:")]),
            Line::from(vec![Span::raw("  k/↑ - Move up")]),
            Line::from(vec![Span::raw("  j/↓ - Move down")]),
            Line::from(vec![Span::raw("  Enter - Confirm/Select")]),
            Line::from(vec![Span::raw("  Esc - Go back/Cancel")]),
            Line::from(vec![Span::raw("  F1 - Toggle help")]),
            Line::from(vec![Span::raw("  Ctrl+C - Force quit")]),
            Line::from(""),
            Line::from(vec![Span::raw("Features:")]),
            Line::from(vec![Span::raw("  • Rich terminal UI")]),
            Line::from(vec![Span::raw("  • Progress bars for downloads")]),
            Line::from(vec![Span::raw("  • Metadata extraction")]),
            Line::from(vec![Span::raw("  • Multiple download sources")]),
            Line::from(vec![Span::raw("  • Smart error handling")]),
        ];

        let help_paragraph = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL).title("Help (Press F1 or Esc to close)"))
            .scroll((self.help_scroll as u16, 0));
        
        f.render_widget(help_paragraph, chunks[1]);
    }

    async fn perform_search(&mut self) -> Result<()> {
        self.mode = AppMode::Downloading;
        self.downloading_message = "Searching...".to_string();
        self.command_tx.send(AppCommand::Search(self.query.clone(), 20))
            .map_err(|e| anyhow::anyhow!("Failed to send command: {}", e))?;
        Ok(())
    }

    async fn fetch_download_links(&mut self) -> Result<()> {
        self.mode = AppMode::Downloading;
        self.downloading_message = "Fetching download links...".to_string();
        let book_url = self.books[self.selected_book_index].url.clone();
        self.command_tx.send(AppCommand::FetchDownloadLinks(book_url))
            .map_err(|e| anyhow::anyhow!("Failed to send command: {}", e))?;
        Ok(())
    }

    async fn perform_download(&mut self) -> Result<()> {
        self.mode = AppMode::Downloading;
        let link = &self.download_links[self.download_link_index];
        let filename = format!(
            "{} - {}.{}",
            self.books[self.selected_book_index].title
                .chars()
                .take(50)
                .collect::<String>(),
            self.books[self.selected_book_index].author.as_deref().unwrap_or("Unknown"),
            self.books[self.selected_book_index].format.as_deref().unwrap_or("unknown")
        );
        
        self.downloading_message = format!("Downloading: {}", filename);
        
        self.command_tx.send(AppCommand::Download(link.url.clone(), self.download_link_index))
             .map_err(|e| anyhow::anyhow!("Failed to send command: {}", e))?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ControlFlow {
    Continue,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DownloadState {
    Idle,
    FetchingLinks,
    Selecting,
    Downloading,
    Complete,
    Error,
}
