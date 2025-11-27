package ui

import (
	"fmt"
	"strings"
	"time"

	"github.com/Nquxii/anna-dl-go/downloader"
	"github.com/Nquxii/anna-dl-go/scraper"
	tea "github.com/charmbracelet/bubbletea"
)

// Command constructors
type searchMsg struct {
	books []scraper.Book
	err   error
}

type downloadLinksMsg struct {
	links []scraper.DownloadLink
	err   error
}

type downloadProgressMsg struct {
	progress float64
	message  string
}

type downloadCompleteMsg struct {
	path string
	err  error
}

// performSearch performs a search and returns the results
func (m Model) performSearch() tea.Cmd {
	return func() tea.Msg {
		s := scraper.New()
		books, err := s.Search(m.query, 20)
		return searchMsg{books: books, err: err}
	}
}

// fetchDownloadLinks fetches download links for the selected book
func (m Model) fetchDownloadLinks() tea.Cmd {
	return func() tea.Msg {
		if m.selectedBookIndex >= len(m.books) {
			return downloadLinksMsg{err: fmt.Errorf("no book selected")}
		}

		s := scraper.New()
		links, err := s.GetBookDetails(m.books[m.selectedBookIndex].URL)
		return downloadLinksMsg{links: links, err: err}
	}
}

// performDownload performs the download
func (m Model) performDownload() tea.Cmd {
	return func() tea.Msg {
		if m.selectedLinkIndex >= len(m.downloadLinks) {
			return downloadCompleteMsg{err: fmt.Errorf("no download link selected")}
		}
		if m.selectedBookIndex >= len(m.books) {
			return downloadCompleteMsg{err: fmt.Errorf("no book selected")}
		}

		book := m.books[m.selectedBookIndex]
		link := m.downloadLinks[m.selectedLinkIndex]

		// Sanitize filename components
		title := strings.ReplaceAll(book.Title[:min(len(book.Title), 50)], "/", "_")
		author := strings.ReplaceAll(book.Author, "/", "_")
		format := strings.ToLower(book.Format)
		
		filename := fmt.Sprintf("%s - %s.%s", title, author, format)

		d := downloader.New(m.downloadPath)

		// Create progress channel
		progressChan := make(chan struct {
			current int64
			total   int64
		}, 10)

		// Run download in goroutine
		errChan := make(chan error, 1)
		pathChan := make(chan string, 1)

		go func() {
			path, err := d.Download(link.URL, filename, func(current, total int64) {
				progressChan <- struct {
					current int64
					total   int64
				}{current, total}
			})
			if err != nil {
				errChan <- err
			} else {
				pathChan <- path
			}
			close(progressChan)
		}()

		// Monitor progress
		progressDone := make(chan struct{})
		go func() {
			for progress := range progressChan {
				progressMsg := downloadProgressMsg{
					progress: float64(progress.current) / float64(progress.total),
					message:  downloader.GetProgressBar(progress.current, progress.total, 30),
				}
				ea.Send(progressMsg)
			}
			close(progressDone)
		}()

		// Wait for completion
		select {
		case err := <-errChan:
			<-progressDone // Ensure progress goroutine finishes
			return downloadCompleteMsg{err: err}
		case path := <-pathChan:
			<-progressDone // Ensure progress goroutine finishes
			return downloadCompleteMsg{path: path}
		case <-time.After(30 * time.Minute): // Timeout
			<-progressDone
			return downloadCompleteMsg{err: fmt.Errorf("download timeout")}
		}
	}
}

// RunTUI runs the interactive terminal UI
func RunTUI(initialQuery, downloadPath string, numResults int) error {
	p := tea.NewProgram(NewModel(downloadPath), tea.WithAltScreen())
	
	// If there's an initial query, set it and trigger search
	model := NewModel(downloadPath)
	model.query = initialQuery
	
	if initialQuery != "" {
		// Start with search in progress
		model.mode = ModeDownloading
		model.downloadMessage = "Searching..."
	}
	
	p = tea.NewProgram(model, tea.WithAltScreen())
	
	// Run the program
	if _, err := p.Run(); err != nil {
		return fmt.Errorf("error running program: %w", err)
	}
	
	return nil
}

// RunNonInteractive runs in non-interactive mode
func RunNonInteractive(query, downloadPath string, numResults int) error {
	fmt.Printf("🔍 Searching for: %s\n\n", query)

	// Search
	s := scraper.New()
	books, err := s.Search(query, numResults)
	if err != nil {
		return fmt.Errorf("search failed: %w", err)
	}

	if len(books) == 0 {
		fmt.Println("❌ No results found")
		return nil
	}

	// Display results
	fmt.Printf("📚 Found %d results:\n\n", len(books))

	for i, book := range books {
		fmt.Printf("  %d. %s\n", i+1, book.Title)
		fmt.Printf("     Author: %s\n", book.Author)
		fmt.Printf("     Year: %s | Language: %s | Format: %s | Size: %s\n",
			book.Year, book.Language, book.Format, book.Size)
		fmt.Println()
	}

	// Auto-select first book (for non-interactive mode)
	fmt.Printf("Auto-selecting first book: %s\n", books[0].Title)

	// Fetch download links
	fmt.Printf("🔗 Fetching download links...\n")
	links, err := s.GetBookDetails(books[0].URL)
	if err != nil {
		return fmt.Errorf("failed to fetch links: %w", err)
	}

	if len(links) == 0 {
		fmt.Println("❌ No download links found")
		return nil
	}

	fmt.Printf("📥 Found %d download links\n\n", len(links))

	// Try to auto-select LibGen link
	var selectedLink *scraper.DownloadLink
	for i, link := range links {
		fmt.Printf("  %d. %s (Source: %s)\n", i+1, link.Text, link.Source)
		if selectedLink == nil && strings.Contains(strings.ToLower(link.Text), "libgen") {
			selectedLink = &links[i]
		}
	}

	if selectedLink == nil {
		selectedLink = &links[0]
	}

	fmt.Printf("\n⬇️  Downloading from: %s\n", selectedLink.Text)

	// Generate filename
	filename := fmt.Sprintf("%s - %s.%s",
		strings.ReplaceAll(books[0].Title[:min(len(books[0].Title), 50)], "/", "_"),
		strings.ReplaceAll(books[0].Author, "/", "_"),
		strings.ToLower(books[0].Format))

	// Download
	d := downloader.New(downloadPath)
	
	fmt.Printf("Progress: ")
	path, err := d.Download(selectedLink.URL, filename, func(current, total int64) {
		fmt.Printf("\rProgress: %s", downloader.GetProgressBar(current, total, 30))
	})
	fmt.Println()

	if err != nil {
		return fmt.Errorf("download failed: %w", err)
	}

	fmt.Printf("\n✅ Download complete: %s\n", path)

	return nil
}

func min(a, b int) int {
	if a < b {
		return a
	}
	return b
}
