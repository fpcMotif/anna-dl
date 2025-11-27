package scraper

import (
	"fmt"
	"net/http"
	"net/url"
	"regexp"
	"strings"
	"time"

	"github.com/PuerkitoBio/goquery"
)

type Book struct {
	Title    string
	Author   string
	Year     string
	Language string
	Format   string
	Size     string
	URL      string
}

type DownloadLink struct {
	Text   string
	URL    string
	Source string
}

type AnnaScraper struct {
	client  *http.Client
	BaseURL string
}

func New() *AnnaScraper {
	return &AnnaScraper{
		client: &http.Client{
			Timeout: 30 * time.Second,
			Transport: &http.Transport{
				MaxIdleConns:        10,
				IdleConnTimeout:     30 * time.Second,
				DisableCompression:  false,
			},
		},
		BaseURL: "https://annas-archive.org",
	}
}

func (s *AnnaScraper) Search(query string, maxResults int) ([]Book, error) {
	searchURL := fmt.Sprintf("%s/search?q=%s", s.BaseURL, url.QueryEscape(query))
	
	req, err := http.NewRequest("GET", searchURL, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to create request: %w", err)
	}
	
	req.Header.Set("User-Agent", s.randomUserAgent())
	
	resp, err := s.client.Do(req)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch search results: %w", err)
	}
	defer resp.Body.Close()
	
	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("HTTP error: %s", resp.Status)
	}
	
	doc, err := goquery.NewDocumentFromReader(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("failed to parse HTML: %w", err)
	}
	
	return s.parseSearchResults(doc, maxResults)
}

func (s *AnnaScraper) GetBookDetails(bookURL string) ([]DownloadLink, error) {
	req, err := http.NewRequest("GET", bookURL, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to create request: %w", err)
	}
	
	req.Header.Set("User-Agent", s.randomUserAgent())
	
	resp, err := s.client.Do(req)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch book details: %w", err)
	}
	defer resp.Body.Close()
	
	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("HTTP error: %s", resp.Status)
	}
	
	doc, err := goquery.NewDocumentFromReader(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("failed to parse HTML: %w", err)
	}
	
	return s.parseDownloadLinks(doc)
}

func (s *AnnaScraper) parseSearchResults(doc *goquery.Document, maxResults int) ([]Book, error) {
	var books []Book
	var foundWithSelector bool
	
	// Define fallbacks for different page structures
	selectors := []string{"a.js-vim-focus.custom-a", "a[href*='md5']", ".book-title a"}
	
	for _, selector := range selectors {
		doc.Find(selector).Each(func(i int, item *goquery.Selection) {
			if len(books) >= maxResults {
				return
			}
			
			book := s.extractBookInfo(item, doc)
			if book.Title != "" {
				books = append(books, book)
				foundWithSelector = true
			}
		})
		
		if foundWithSelector {
			break
		}
	}
	
	return books, nil
}

func (s *AnnaScraper) extractBookInfo(item *goquery.Selection, doc *goquery.Document) Book {
	title := strings.TrimSpace(item.Text())
	if title == "" {
		return Book{}
	}
	
	// Get URL
	url, exists := item.Attr("href")
	if !exists || !strings.HasPrefix(url, "/") {
		return Book{}
	}
	url = s.BaseURL + url
	
	// Extract metadata from containing element
	container := item.Closest("div")
	containerText := container.Text()
	
	return Book{
		Title:    title,
		Author:   s.extractAuthor(containerText),
		Year:     s.extractYear(containerText),
		Language: s.extractLanguage(containerText),
		Format:   s.extractFormat(containerText),
		Size:     s.extractSize(containerText),
		URL:      url,
	}
}

func (s *AnnaScraper) parseDownloadLinks(doc *goquery.Document) ([]DownloadLink, error) {
	var links []DownloadLink
	
	// Define fallbacks for different page structures
	selectors := []string{"#external-downloads a", ".external-downloads a"}
	
	for _, selector := range selectors {
		doc.Find(selector).Each(func(i int, item *goquery.Selection) {
			link := s.extractDownloadLink(item)
			if link.Text != "" {
				links = append(links, link)
			}
		})
		
		if len(links) > 0 {
			break
		}
	}
	
	// Fallback
	if len(links) == 0 {
		doc.Find("a[href*='libgen'], a[href*='download'], .download-link").Each(func(i int, item *goquery.Selection) {
			link := s.extractDownloadLink(item)
			if link.Text != "" {
				links = append(links, link)
			}
		})
	}
	
	return links, nil
}

func (s *AnnaScraper) extractDownloadLink(item *goquery.Selection) DownloadLink {
	url, exists := item.Attr("href")
	if !exists {
		return DownloadLink{}
	}
	
	text := strings.TrimSpace(item.Text())
	source := s.detectSource(url)
	
	return DownloadLink{
		Text:   text,
		URL:    url,
		Source: source,
	}
}

func (s *AnnaScraper) extractAuthor(text string) string {
	// Try to find "Author [lang]" pattern first (common in search results)
	// Example: "Author Name [en], epub, ..."
	re := regexp.MustCompile(`([^\[\n]+)\s\[[a-z]{2}\]`)
	matches := re.FindStringSubmatch(text)
	if len(matches) > 1 {
		author := strings.TrimSpace(matches[1])
		// Sometimes title is included, we might want to filter it if possible,
		// but with just text it's hard.
		// However, the regex `([^\[\n]+)` stops at `[` so it captures everything before ` [lang]`.
		// If the text is "Title\nAuthor [en]", it captures "Title\nAuthor".
		// We should take the last line?
		lines := strings.Split(author, "\n")
		if len(lines) > 0 {
			return strings.TrimSpace(lines[len(lines)-1])
		}
		return author
	}

	lines := strings.Split(text, "\n")
	for _, line := range lines {
		line = strings.TrimSpace(line)
		if len(line) < 50 && !strings.HasPrefix(line, "[") && !strings.Contains(line, "http") {
			matched, _ := regexp.MatchString(`^[A-Za-z\s,\.]+$`, line)
			if matched && line != "" {
				return line
			}
		}
	}
	return "Unknown"
}

func (s *AnnaScraper) extractYear(text string) string {
	re := regexp.MustCompile(`\b(19|20)\d{2}\b`)
	matches := re.FindStringSubmatch(text)
	if len(matches) > 0 {
		return matches[0]
	}
	return "Unknown"
}

func (s *AnnaScraper) extractLanguage(text string) string {
	re := regexp.MustCompile(`(\w+)\s+\[([a-z]{2})\]`)
	matches := re.FindStringSubmatch(text)
	if len(matches) > 2 {
		return matches[2]
	}
	return "Unknown"
}

func (s *AnnaScraper) extractFormat(text string) string {
	re := regexp.MustCompile(`(?i)\b(EPUB|PDF|MOBI|AZW3|TXT|DOC|DOCX)\b`)
	matches := re.FindStringSubmatch(text)
	if len(matches) > 0 {
		return strings.ToUpper(matches[0])
	}
	return "Unknown"
}

func (s *AnnaScraper) extractSize(text string) string {
	re := regexp.MustCompile(`(\d+\.?\d*\s*[MKG]B)`)
	matches := re.FindStringSubmatch(text)
	if len(matches) > 0 {
		return matches[0]
	}
	return "Unknown"
}

func (s *AnnaScraper) detectSource(href string) string {
	if strings.Contains(href, "libgen") {
		return "LibGen"
	} else if strings.Contains(href, "annas") {
		return "Anna's Archive"
	} else if strings.Contains(href, "mirror") {
		return "Mirror"
	}
	return "Unknown"
}

func (s *AnnaScraper) randomUserAgent() string {
	userAgents := []string{
		"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
		"Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
		"Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
	}
	return userAgents[0] // Simple for now, could add random selection
}
