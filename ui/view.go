package ui

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/lipgloss"
)

func (m Model) View() string {
	if m.width == 0 {
		m.width = 80
	}
	if m.height == 0 {
		m.height = 24
	}

	header := m.renderHeader()
	content := ""

	switch m.mode {
	case ModeSearch:
		content = m.renderSearch()
	case ModeResults:
		content = m.renderResults()
	case ModeDownloadSelection:
		content = m.renderDownloadSelection()
	case ModeDownloading:
		content = m.renderDownloading()
	case ModeError:
		content = m.renderError()
	case ModeHelp:
		content = m.renderHelp()
	}

	footer := m.renderFooter()

	return lipgloss.JoinVertical(lipgloss.Left, header, content, footer)
}

func (m Model) renderHeader() string {
	logo := m.styles.AppName.Render("📚 Anna's Archive Downloader")
	
	// Calculate centered position
	width := m.width
	logoWidth := lipgloss.Width(logo)
	padding := (width - logoWidth) / 2
	if padding < 0 {
		padding = 0
	}

	header := strings.Repeat(" ", padding) + logo

	// Add decorative border
	border := strings.Repeat("─", width)
	
	return m.styles.Header.Render(border + "\n" + header + "\n" + border)
}

func (m Model) renderFooter() string {
	var footerContent string
	
	switch m.mode {
	case ModeSearch:
		footerContent = "Type to search • Enter to confirm • Ctrl+C to quit • F1 for help"
	case ModeResults:
		footerContent = "↑/k ↓/j navigate • Enter to select • Esc back • Ctrl+C quit • F1 help"
	case ModeDownloadSelection:
		footerContent = "↑/k ↓/j navigate • Enter to download • Esc back • Ctrl+C quit • F1 help"
	default:
		footerContent = "Ctrl+C to quit • F1 for help"
	}

	return m.styles.Help.Render(footerContent)
}

func (m Model) renderSearch() string {
	// Create a beautiful search box
	searchBoxWidth := min(m.width-4, 60)
	
	// Title
	title := m.styles.Header.Render("Search Mode")
	
	// Query display
	queryDisplay := m.styles.NormalText.Render(fmt.Sprintf("Query: %s", m.query))
	
	// Instructions
	instructions := []string{
		m.styles.Help.Render("Type your search query and press Enter"),
		m.styles.Help.Render("Use Ctrl+C to quit or F1 for help"),
	}

	// Combine all elements
	content := lipgloss.JoinVertical(
		lipgloss.Left,
		title,
		"",
		queryDisplay,
		"",
		instructions[0],
		instructions[1],
	)

	// Add border and center
	box := lipgloss.NewStyle().
		Width(searchBoxWidth).
		Border(lipgloss.RoundedBorder()).
		BorderForeground(lipgloss.Color("89b4fa")).
		Padding(2).
		Render(content)

	// Center in terminal
	return lipgloss.Place(m.width, m.height-6,
		lipgloss.Center, lipgloss.Center,
		box,
	)
}

func (m Model) renderResults() string {
	// Title with query
	title := m.styles.Header.Render(fmt.Sprintf("Results for: %s", m.query))
	
	// Book list
	var bookList []string
	maxVisible := m.height - 15
	
	for i := m.resultsScroll; i < len(m.books) && i < m.resultsScroll+maxVisible; i++ {
		book := m.books[i]
		
		// Format book entry
		number := fmt.Sprintf("%d.", i+1)
		
		var line1, line2, line3 string
		
		if i == m.selectedBookIndex {
			// Selected item
			line1 = m.styles.Selected.Render(fmt.Sprintf("%s %s", number, book.Title))
			line2 = m.styles.NormalText.Render(fmt.Sprintf("   Author: %s", book.Author))
			line3 = m.styles.Help.Render(fmt.Sprintf("   Year: %s | Language: %s | Format: %s | Size: %s",
				book.Year, book.Language, book.Format, book.Size))
		} else {
			// Normal item
			line1 = m.styles.NormalText.Render(fmt.Sprintf("%s %s", number, book.Title))
			line2 = m.styles.Help.Render(fmt.Sprintf("   Author: %s", book.Author))
			line3 = m.styles.Help.Render(fmt.Sprintf("   Year: %s | Language: %s | Format: %s | Size: %s",
				book.Year, book.Language, book.Format, book.Size))
		}
		
		bookList = append(bookList, line1, line2, line3, "")
	}

	// Combine all elements
	content := lipgloss.JoinVertical(
		lipgloss.Left,
		title,
		"",
		strings.Join(bookList, "\n"),
	)

	// Add stats
	stats := m.styles.Help.Render(fmt.Sprintf("Showing %d of %d books | Press Enter to see download options",
		min(len(m.books), m.height-15), len(m.books)))
	
	return lipgloss.NewStyle().
		PaddingLeft(2).
		PaddingRight(2).
		Render(lipgloss.JoinVertical(lipgloss.Left, content, "", stats))
}

func (m Model) renderDownloadSelection() string {
	if m.selectedBookIndex >= len(m.books) {
		return m.styles.Error.Render("No book selected")
	}

	book := m.books[m.selectedBookIndex]
	
	// Book info panel
	bookInfo := []string{
		m.styles.Header.Render("Book Information"),
		"",
		m.styles.NormalText.Render(fmt.Sprintf("Title: %s", book.Title)),
		m.styles.NormalText.Render(fmt.Sprintf("Author: %s", book.Author)),
		m.styles.NormalText.Render(fmt.Sprintf("Year: %s", book.Year)),
		m.styles.NormalText.Render(fmt.Sprintf("Language: %s", book.Language)),
		m.styles.NormalText.Render(fmt.Sprintf("Format: %s", book.Format)),
		m.styles.NormalText.Render(fmt.Sprintf("Size: %s", book.Size)),
	}
	
	// Download links
	var linkList []string
	linkList = append(linkList, "", m.styles.Header.Render("Available Download Links"), "")
	
	for i, link := range m.downloadLinks {
		prefix := fmt.Sprintf("%d.", i+1)
		
		var line1, line2 string
		if i == m.selectedLinkIndex {
			line1 = m.styles.Selected.Render(fmt.Sprintf("%s %s", prefix, link.Text))
			line2 = m.styles.Help.Render(fmt.Sprintf("   Source: %s", link.Source))
		} else {
			line1 = m.styles.NormalText.Render(fmt.Sprintf("%s %s", prefix, link.Text))
			line2 = m.styles.Help.Render(fmt.Sprintf("   Source: %s", link.Source))
		}
		
		linkList = append(linkList, line1, line2, "")
	}

	content := strings.Join(append(bookInfo, linkList...), "\n")
	
	return lipgloss.NewStyle().
		PaddingLeft(2).
		PaddingRight(2).
		Render(content)
}

func (m Model) renderDownloading() string {
	// Progress bar
	progressBar := m.styles.ProgressBar.Render(
		strings.Repeat("░", 40),
	)
	
	if m.downloadProgress > 0 {
		filled := int(m.downloadProgress * 40)
		if filled > 40 {
			filled = 40
		}
		progressBar = lipgloss.NewStyle().
			Background(lipgloss.Color("#89b4fa")).
			Render(strings.Repeat("█", filled)) +
			m.styles.ProgressBar.Render(strings.Repeat("░", 40-filled))
	}
	
	// Status panel
	status := []string{
		m.styles.Header.Render("Downloading"),
		"",
		m.styles.Highlight.Render(m.downloadMessage),
		"",
		progressBar,
		"",
		m.styles.Help.Render("Press Ctrl+C to cancel"),
	}
	
	// Center the status panel
	statusBox := lipgloss.NewStyle().
		Width(min(60, m.width-4)).
		Padding(2).
		Border(lipgloss.RoundedBorder()).
		BorderForeground(lipgloss.Color("#f9e2af")).
		Render(strings.Join(status, "\n"))
	
	return lipgloss.Place(m.width, m.height-6,
		lipgloss.Center, lipgloss.Center,
		statusBox,
	)
}

func (m Model) renderError() string {
	errorBox := lipgloss.NewStyle().
		Width(min(60, m.width-4)).
		Padding(2).
		Border(lipgloss.RoundedBorder()).
		BorderForeground(lipgloss.Color("#f38ba8")).
		Render(lipgloss.JoinVertical(
			lipgloss.Center,
			m.styles.Error.Render("ERROR"),
			"",
			m.styles.NormalText.Render(m.errorMessage),
			"",
			m.styles.Help.Render("Press ESC or Enter to return"),
		))
	
	return lipgloss.Place(m.width, m.height-6,
		lipgloss.Center, lipgloss.Center,
		errorBox,
	)
}

func (m Model) renderHelp() string {
	helpContent := []string{
		m.styles.Header.Render("Help - Anna's Archive Downloader"),
		"",
		m.styles.Highlight.Render("Navigation Keys:"),
		m.styles.NormalText.Render("  • k/↑ - Move up"),
		m.styles.NormalText.Render("  • j/↓ - Move down"),
		m.styles.NormalText.Render("  • Enter - Confirm/Select"),
		m.styles.NormalText.Render("  • Esc - Go back/Cancel"),
		m.styles.NormalText.Render("  • F1 - Toggle help"),
		m.styles.NormalText.Render("  • Ctrl+C - Force quit"),
		"",
		m.styles.Highlight.Render("Search Mode:"),
		m.styles.Help.Render("  Type your query and press Enter to search"),
		"",
		m.styles.Highlight.Render("Results Mode:"),
		m.styles.Help.Render("  Navigate with arrow keys or j/k"),
		m.styles.Help.Render("  Press Enter to view download options for selected book"),
		"",
		m.styles.Highlight.Render("Download Selection:"),
		m.styles.Help.Render("  Choose from available download sources"),
		m.styles.Help.Render("  Press Enter to start download"),
		"",
		m.styles.Highlight.Render("Features:"),
		m.styles.NormalText.Render("  • Rich terminal UI with beautiful styling"),
		m.styles.NormalText.Render("  • Real-time download progress"),
		m.styles.NormalText.Render("  • Metadata extraction (author, year, format, etc.)"),
		m.styles.NormalText.Render("  • Multiple download sources"),
		m.styles.NormalText.Render("  • Smart error handling"),
		m.styles.NormalText.Render("  • Configurable download directory"),
	}
	
	helpBox := lipgloss.NewStyle().
		Width(min(70, m.width-4)).
		Padding(1).
		Render(strings.Join(helpContent, "\n"))
	
	return lipgloss.Place(m.width, m.height-6,
		lipgloss.Center, lipgloss.Center,
		helpBox,
	)
}
