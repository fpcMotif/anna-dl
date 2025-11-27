package ui

import (
	"github.com/charmbracelet/bubbles/key"
	tea "github.com/charmbracelet/bubbletea"
)

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height
		return m, nil

	case tea.KeyMsg:
		return m.handleKeyPress(msg)

	case searchMsg:
		if msg.err != nil {
			m.mode = ModeError
			m.errorMessage = msg.err.Error()
		} else {
			m.books = msg.books
			if len(msg.books) == 0 {
				m.mode = ModeError
				m.errorMessage = "No results found"
			} else {
				m.mode = ModeResults
				m.selectedBookIndex = 0
				m.resultsScroll = 0
			}
		}
		return m, nil

	case downloadLinksMsg:
		if msg.err != nil {
			m.mode = ModeError
			m.errorMessage = msg.err.Error()
		} else {
			m.downloadLinks = msg.links
			if len(msg.links) == 0 {
				m.mode = ModeError
				m.errorMessage = "No download links found"
			} else {
				m.mode = ModeDownloadSelection
				m.selectedLinkIndex = 0
			}
		}
		return m, nil

	case downloadProgressMsg:
		m.downloadProgress = msg.progress
		m.downloadMessage = msg.message
		return m, nil

	case downloadCompleteMsg:
		if msg.err != nil {
			m.mode = ModeError
			m.errorMessage = msg.err.Error()
		} else {
			m.mode = ModeSearch
			m.query = ""
			m.books = []scraper.Book{}
			m.downloadMessage = fmt.Sprintf("✓ Downloaded to: %s", msg.path)
		}
		m.downloadProgress = 0
		return m, nil
	}

	return m, nil
}

func (m Model) handleKeyPress(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	switch m.mode {
	case ModeSearch:
		return m.handleSearchKeys(msg)
	case ModeResults:
		return m.handleResultsKeys(msg)
	case ModeDownloadSelection:
		return m.handleDownloadSelectionKeys(msg)
	case ModeError:
		return m.handleErrorKeys(msg)
	case ModeHelp:
		return m.handleHelpKeys(msg)
	case ModeDownloading:
		return m.handleDownloadingKeys(msg)
	}
	return m, nil
}

func (m Model) handleSearchKeys(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	switch {
	case key.Matches(msg, KeyQuit):
		return m, tea.Quit

	case key.Matches(msg, KeyEnter):
		if m.query != "" {
			m.mode = ModeDownloading
			m.downloadMessage = "Searching..."
			return m, m.performSearch()
		}

	case key.Matches(msg, KeyHelp):
		m.mode = ModeHelp
		return m, nil

	case msg.Type == tea.KeyBackspace:
		if len(m.query) > 0 {
			m.query = m.query[:len(m.query)-1]
		}

	case msg.Type == tea.KeyRunes:
		m.query += string(msg.Runes)
	}

	return m, nil
}

func (m Model) handleResultsKeys(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	switch {
	case key.Matches(msg, KeyQuit):
		return m, tea.Quit

	case key.Matches(msg, KeyBack):
		m.mode = ModeSearch
		m.query = ""
		m.selectedBookIndex = 0
		m.resultsScroll = 0

	case key.Matches(msg, KeyEnter):
		if len(m.books) > 0 {
			m.mode = ModeDownloading
			m.downloadMessage = "Fetching download links..."
			return m, m.fetchDownloadLinks()
		}

	case key.Matches(msg, KeyUp):
		if m.selectedBookIndex > 0 {
			m.selectedBookIndex--
			if m.selectedBookIndex < m.resultsScroll {
				m.resultsScroll = m.selectedBookIndex
			}
		}

	case key.Matches(msg, KeyDown):
		if m.selectedBookIndex < len(m.books)-1 {
			m.selectedBookIndex++
			maxVisible := m.height - 15
			if m.selectedBookIndex > m.resultsScroll+maxVisible {
				m.resultsScroll = m.selectedBookIndex - maxVisible
			}
		}

	case key.Matches(msg, KeyHelp):
		m.mode = ModeHelp
	}

	return m, nil
}

func (m Model) handleDownloadSelectionKeys(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	switch {
	case key.Matches(msg, KeyQuit):
		return m, tea.Quit

	case key.Matches(msg, KeyBack):
		m.mode = ModeResults
		m.selectedLinkIndex = 0

	case key.Matches(msg, KeyEnter):
		if len(m.downloadLinks) > 0 {
			m.mode = ModeDownloading
			m.downloadMessage = "Starting download..."
			return m, m.performDownload()
		}

	case key.Matches(msg, KeyUp):
		if m.selectedLinkIndex > 0 {
			m.selectedLinkIndex--
		}

	case key.Matches(msg, KeyDown):
		if m.selectedLinkIndex < len(m.downloadLinks)-1 {
			m.selectedLinkIndex++
		}

	case key.Matches(msg, KeyHelp):
		m.mode = ModeHelp
	}

	return m, nil
}

func (m Model) handleErrorKeys(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	switch {
	case key.Matches(msg, KeyQuit):
		return m, tea.Quit

	case key.Matches(msg, KeyBack), key.Matches(msg, KeyEnter):
		m.mode = ModeSearch
		m.errorMessage = ""
	}

	return m, nil
}

func (m Model) handleHelpKeys(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	switch {
	case key.Matches(msg, KeyQuit):
		return m, tea.Quit

	case key.Matches(msg, KeyBack), key.Matches(msg, KeyHelp):
		m.mode = ModeSearch
	}

	return m, nil
}

func (m Model) handleDownloadingKeys(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	if key.Matches(msg, KeyQuit) {
		return m, tea.Quit
	}
	return m, nil
}
