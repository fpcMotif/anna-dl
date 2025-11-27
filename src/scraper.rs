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
            
            let mut seen_urls = std::collections::HashSet::new();

            for selector_str in &link_selectors {
                if let Ok(selector) = Selector::parse(selector_str) {
                    for element in document.select(&selector) {
                        if let Some(link) = self.extract_download_link(element) {
                            if seen_urls.insert(link.url.clone()) {
                                links.push(link);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(links)
    }
    
    fn extract_book_info(&self, element: &scraper::ElementRef, _document: &Html) -> Option<Book> {
        let href = element.value().attr("href")?.to_string();
        let title = element.text().collect::<String>().trim().to_string();
        
        if title.is_empty() {
            return None;
        }
        
        // Find parent container for metadata
        let container = self.find_book_container(*element)?;
        let container_text = container.text().collect::<String>();
        
        Some(Book {
            title: title.clone(),
            author: self.extract_author(&container_text, &title),
            year: self.extract_year(&container_text),
            language: self.extract_language(&container_text),
            format: self.extract_format(&container_text),
            size: self.extract_size(&container_text),
            url: format!("https://annas-archive.org{}", href),
        })
    }
    
    fn find_book_container<'a>(&self, element: scraper::ElementRef<'a>) -> Option<scraper::ElementRef<'a>> {
        let mut current = element;
        
        // Walk up up to 5 levels to find container
        for _ in 0..5 {
            if let Some(parent) = current.parent().and_then(scraper::ElementRef::wrap) {
                let element = parent.value();
                // Check for key classes
                if element.classes().any(|c| c == "book-item" || c == "flex" || c.contains("border") || c.contains("pt-3")) {
                    return Some(parent);
                }
                current = parent;
            } else {
                break;
            }
        }
        
        None
    }
    
    fn extract_author(&self, text: &str, exclude: &str) -> Option<String> {
        // Look for author patterns in text
        let lines: Vec<&str> = text.lines().collect();
        for line in lines {
            let line = line.trim();
            if line.is_empty() || line == exclude { continue; }
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
        let mut seen_urls = std::collections::HashSet::new();
        
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
                        if seen_urls.insert(link.url.clone()) {
                            links.push(link);
                        }
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
            url: href.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_year() {
        let scraper = AnnaScraper::new().unwrap();
        assert_eq!(scraper.extract_year("Some Book (2023)"), Some("2023".to_string()));
        assert_eq!(scraper.extract_year("Old Book [1999]"), Some("1999".to_string()));
        assert_eq!(scraper.extract_year("No Year Here"), None);
    }

    #[test]
    fn test_extract_language() {
        let scraper = AnnaScraper::new().unwrap();
        assert_eq!(scraper.extract_language("English [en]"), Some("English".to_string()));
        assert_eq!(scraper.extract_language("Russian [ru]"), Some("Russian".to_string()));
        assert_eq!(scraper.extract_language("No Lang"), None);
    }

    #[test]
    fn test_extract_format() {
        let scraper = AnnaScraper::new().unwrap();
        assert_eq!(scraper.extract_format("File.PDF"), Some("PDF".to_string()));
        assert_eq!(scraper.extract_format("Book in EPUB format"), Some("EPUB".to_string()));
        assert_eq!(scraper.extract_format("Unknown format"), None);
    }

    #[test]
    fn test_extract_size() {
        let scraper = AnnaScraper::new().unwrap();
        assert_eq!(scraper.extract_size("Size: 1.5MB"), Some("1.5MB".to_string()));
        assert_eq!(scraper.extract_size("100KB"), Some("100KB".to_string()));
        assert_eq!(scraper.extract_size("No size"), None);
    }

    #[test]
    fn test_detect_source() {
        let scraper = AnnaScraper::new().unwrap();
        assert_eq!(scraper.detect_source("http://libgen.rs/book"), "LibGen");
        assert_eq!(scraper.detect_source("https://annas-archive.org/md5/..."), "Anna's Archive");
        assert_eq!(scraper.detect_source("http://example.com/mirror/1"), "Mirror");
        assert_eq!(scraper.detect_source("http://unknown.com"), "Unknown");
    }

    #[tokio::test]
    async fn test_parse_search_results() {
        let scraper = AnnaScraper::new().unwrap();
        let html = r#"
        <html>
            <body>
                <div class="book-item">
                    <a href="/md5/12345" class="js-vim-focus custom-a">Test Book</a>
                    <div class="text-sm">
                        Unknown Author
                        2023
                        English [en]
                        PDF
                        1.5MB
                    </div>
                </div>
                <div class="book-item">
                     <a href="/md5/67890" class="js-vim-focus custom-a">Another Book</a>
                     <div class="text-sm">
                        John Doe
                        2020
                        EPUB
                     </div>
                </div>
            </body>
        </html>
        "#;

        let books = scraper.parse_search_results(html, 10).await.unwrap();
        // Depending on selector implementation, it might find books or not.
        // Existing selectors: "a.js-vim-focus.custom-a"
        // This matches our mock.

        assert_eq!(books.len(), 2);

        assert_eq!(books[0].title, "Test Book");
        assert_eq!(books[0].url, "https://annas-archive.org/md5/12345");
        // extract_year regex: r"\b(19|20)\d{2}\b"
        assert_eq!(books[0].year.as_deref(), Some("2023"));
        // extract_language regex: r"(\w+)\s+\[([a-z]{2})\]" -> matches "English [en]"
        assert_eq!(books[0].language.as_deref(), Some("English"));
        assert_eq!(books[0].format.as_deref(), Some("PDF"));
        assert_eq!(books[0].size.as_deref(), Some("1.5MB"));

        assert_eq!(books[1].title, "Another Book");
        assert_eq!(books[1].author.as_deref(), Some("John Doe"));
        assert_eq!(books[1].format.as_deref(), Some("EPUB"));
    }

    #[tokio::test]
    async fn test_parse_download_links() {
        let scraper = AnnaScraper::new().unwrap();
        let html = r#"
        <html>
            <body>
                <div id="external-downloads">
                    <a href="http://libgen.li/ads" class="download-link">Libgen.li</a>
                    <a href="https://annas-archive.org/slow" class="download-link">Slow Download</a>
                </div>
            </body>
        </html>
        "#;
        // selector "#external-downloads" matches.
        // inside, "a.download-link" matches.

        let links = scraper.parse_download_links(html).await.unwrap();
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].text, "Libgen.li");
        assert_eq!(links[0].source, "LibGen");
        assert_eq!(links[1].source, "Anna's Archive");
    }
}
