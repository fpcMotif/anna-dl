use anyhow::{Context, Result};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(Self::random_user_agent())
            .build()
            .context("Failed to create HTTP client")?;
        
        Ok(Self { client })
    }
    
    pub async fn search(&self, query: &str, max_results: usize) -> Result<Vec<Book>> {
        let search_url = format!("https://annas-archive.org/search?q={}", 
            urlencoding::encode(query));
        
        let html = self.fetch_html(&search_url).await?;
        self.parse_search_results(&html, max_results).await
    }
    
    pub async fn get_book_details(&self, book_url: &str) -> Result<Vec<DownloadLink>> {
        let html = self.fetch_html(book_url).await?;
        self.parse_download_links(&html).await
    }
    
    async fn fetch_html(&self, url: &str) -> Result<String> {
        let response = self.client
            .get(url)
            .send()
            .await
            .context("Failed to fetch URL")?;
        
        if !response.status().is_success() {
            anyhow::bail!("HTTP error: {}", response.status());
        }
        
        response.text().await.context("Failed to read response body")
    }
    
    async fn parse_search_results(&self, html: &str, max_results: usize) -> Result<Vec<Book>> {
        let document = Html::parse_document(html);
        
        // Multiple fallback selectors for book links
        let selectors = [
            "a.js-vim-focus.custom-a",
            "a[href*='md5']",
            ".book-title a",
            "a[href*='book']",
        ];
        
        let mut books = Vec::new();
        
        for selector_str in &selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                let elements: Vec<_> = document.select(&selector).take(max_results * 2).collect();
                
                if !elements.is_empty() {
                    for element in elements.iter().take(max_results) {
                        if let Some(book) = self.extract_book_info(element, &document) {
                            books.push(book);
                        }
                    }
                    break;
                }
            }
        }
        
        Ok(books)
    }
    
    async fn parse_download_links(&self, html: &str) -> Result<Vec<DownloadLink>> {
        let document = Html::parse_document(html);
        let mut links = Vec::new();
        
        // Look for external download section
        let section_selectors = [
            "#external-downloads",
            ".external-downloads",
            "[data-section='downloads']",
        ];
        
        for selector_str in &section_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Some(section) = document.select(&selector).next() {
                    links.extend(self.extract_links_from_section(&section));
                }
            }
        }
        
        // Fallback: search all download links on page
        if links.is_empty() {
            let link_selectors = [
                "a[href*='libgen']",
                "a[href*='download']",
                "a[href*='mirror']",
                "a[href*='get.php']",
                ".download-link",
            ];
            
            for selector_str in &link_selectors {
                if let Ok(selector) = Selector::parse(selector_str) {
                    for element in document.select(&selector) {
                        if let Some(link) = self.extract_download_link(element) {
                            links.push(link);
                        }
                    }
                }
            }
        }
        
        Ok(links)
    }
    
    fn extract_book_info(&self, element: &scraper::ElementRef, document: &Html) -> Option<Book> {
        let href = element.value().attr("href")?.to_string();
        let title = element.text().collect::<String>().trim().to_string();
        
        if title.is_empty() {
            return None;
        }
        
        // Find parent container for metadata
        let container = self.find_book_container(element.value(), document)?;
        let container_text = container.text().collect::<String>();
        
        Some(Book {
            title,
            author: self.extract_author(&container_text),
            year: self.extract_year(&container_text),
            language: self.extract_language(&container_text),
            format: self.extract_format(&container_text),
            size: self.extract_size(&container_text),
            url: format!("https://annas-archive.org{}", href),
        })
    }
    
    fn find_book_container(&self, element: &scraper::Node, document: &Html) -> Option<scraper::ElementRef> {
        // Try to find container with metadata by going up the tree
        let container_selectors = [
            "div.flex",
            ".book-item",
            "[class*='border']",
            "div[class*='pt-3']",
        ];
        
        for selector_str in &container_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Some(container) = document.select(&selector).next() {
                    return Some(container);
                }
            }
        }
        
        None
    }
    
    fn extract_author(&self, text: &str) -> Option<String> {
        // Look for author patterns in text
        let lines: Vec<&str> = text.lines().collect();
        for line in lines {
            let line = line.trim();
            // Author usually appears as a name without brackets or special chars
            if line.len() < 50 && !line.starts_with('[') && !line.contains("http") {
                if line.chars().all(|c| c.is_alphabetic() || c.is_whitespace() || c == ',' || c == '.') {
                    return Some(line.to_string());
                }
            }
        }
        None
    }
    
    fn extract_year(&self, text: &str) -> Option<String> {
        let re = regex::Regex::new(r"\b(19|20)\d{2}\b").ok()?;
        re.find(text).map(|m| m.as_str().to_string())
    }
    
    fn extract_language(&self, text: &str) -> Option<String> {
        let re = regex::Regex::new(r"(\w+)\s+\[([a-z]{2})\]").ok()?;
        re.captures(text).and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
    }
    
    fn extract_format(&self, text: &str) -> Option<String> {
        let re = regex::Regex::new(r"\b(EPUB|PDF|MOBI|AZW3|TXT|DOC|DOCX)\b").ok()?;
        re.find(text).map(|m| m.as_str().to_string())
    }
    
    fn extract_size(&self, text: &str) -> Option<String> {
        let re = regex::Regex::new(r"(\d+\.?\d*\s*[MKG]B)").ok()?;
        re.find(text).map(|m| m.as_str().to_string())
    }
    
    fn extract_links_from_section(&self, section: &scraper::ElementRef) -> Vec<DownloadLink> {
        let mut links = Vec::new();
        
        let link_selectors = [
            "a[href*='libgen']",
            "a[href*='download']",
            "a.download-link",
            "a[href*='mirror']",
        ];
        
        for selector_str in &link_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in section.select(&selector) {
                    if let Some(link) = self.extract_download_link(element) {
                        links.push(link);
                    }
                }
            }
        }
        
        links
    }
    
    fn extract_download_link(&self, element: scraper::ElementRef) -> Option<DownloadLink> {
        let href = element.value().attr("href")?.to_string();
        let text = element.text().collect::<String>().trim().to_string();
        
        Some(DownloadLink {
            text,
            url: href,
            source: self.detect_source(&href),
        })
    }
    
    fn detect_source(&self, href: &str) -> String {
        if href.contains("libgen") {
            "LibGen".to_string()
        } else if href.contains("annas") {
            "Anna's Archive".to_string()
        } else if href.contains("mirror") {
            "Mirror".to_string()
        } else {
            "Unknown".to_string()
        }
    }
    
    fn random_user_agent() -> String {
        use rand::seq::SliceRandom;
        
        let user_agents = [
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        ];
        
        user_agents.choose(&mut rand::thread_rng()).unwrap().to_string()
    }
}

#[derive(Debug, Clone)]
pub struct DownloadLink {
    pub text: String,
    pub url: String,
    pub source: String,
}

impl DownloadLink {
    pub fn is_reliable(&self) -> bool {
        self.source == "LibGen" && self.text.to_lowercase().contains("libgen")
    }
}
