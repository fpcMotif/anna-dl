package ui

import (
	"github.com/Nquxii/anna-dl-go/scraper"
	"github.com/charmbracelet/bubbles/key"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
)

type Mode int

const (
	ModeSearch Mode = iota
	ModeResults
	ModeDownloadSelection
	ModeDownloading
	ModeError
	ModeHelp
)

type Model struct {
	// UI State
	mode Mode
	
	// Search
	query string
	
	// Results
	books             []scraper.Book
	selectedBookIndex int
	resultsScroll     int
	
	// Download links
	downloadLinks     []scraper.DownloadLink
	selectedLinkIndex int
	
	// Download progress
	downloadProgress  float64
	downloadMessage   string
	downloadFilename  string
	
	// Error
	errorMessage string
	
	// Config
	downloadPath string
	
	// Dimensions
	width  int
	height int
	
	// Styling
	styles *Styles
}

func NewModel(downloadPath string) Model {
	return Model{
		mode:              ModeSearch,
		query:             "",
		books:             []scraper.Book{},
		selectedBookIndex: 0,
		resultsScroll:     0,
		downloadLinks:     []scraper.DownloadLink{},
		selectedLinkIndex: 0,
		downloadProgress:  0,
		downloadMessage:   "",
		downloadFilename:  "",
		errorMessage:      "",
		downloadPath:      downloadPath,
		styles:            NewStyles(),
	}
}

func (m Model) Init() tea.Cmd {
	return nil
}

type Styles struct {
	AppName        lipgloss.Style
	Header         lipgloss.Style
	SubHeader      lipgloss.Style
	NormalText     lipgloss.Style
	Highlight      lipgloss.Style
	Selected       lipgloss.Style
	Error          lipgloss.Style
	Success        lipgloss.Style
	Keybinding     lipgloss.Style
	Help           lipgloss.Style
	ProgressBar    lipgloss.Style
	ProgressFill   lipgloss.Style
}

func NewStyles() *Styles {
	return &Styles{
		AppName:      lipgloss.NewStyle().Bold(true).Foreground(lipgloss.AdaptiveColor{Light: "#1e1e2e", Dark: "#cdd6f4"}).Underline(true),
		Header:       lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#89b4fa")),
		SubHeader:    lipgloss.NewStyle().Foreground(lipgloss.Color("#6c7086")),
		NormalText:   lipgloss.NewStyle().Foreground(lipgloss.AdaptiveColor{Light: "#1e1e2e", Dark: "#cdd6f4"}),
		Highlight:    lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#f9e2af")),
		Selected:     lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#a6e3a1")),
		Error:        lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#f38ba8")),
		Success:      lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#a6e3a1")),
		Keybinding:   lipgloss.NewStyle().Foreground(lipgloss.Color("#f5c2e7")),
		Help:         lipgloss.NewStyle().Foreground(lipgloss.Color("#6c7086")),
		ProgressBar:  lipgloss.NewStyle().Background(lipgloss.Color("#313244")).Foreground(lipgloss.Color("#89b4fa")),
		ProgressFill: lipgloss.NewStyle().Background(lipgloss.Color("#89b4fa")).Foreground(lipgloss.Color("#181825")),
	}
}

// Global keybindings
var (
	KeyQuit          = key.NewBinding(key.WithKeys("ctrl+c"), key.WithHelp("ctrl+c", "quit"))
	KeyEnter         = key.NewBinding(key.WithKeys("enter"), key.WithHelp("enter", "confirm"))
	KeyBack          = key.NewBinding(key.WithKeys("esc"), key.WithHelp("esc", "back"))
	KeyHelp          = key.NewBinding(key.WithKeys("f1"), key.WithHelp("f1", "help"))
	KeyUp            = key.NewBinding(key.WithKeys("up", "k"), key.WithHelp("↑/k", "up"))
	KeyDown          = key.NewBinding(key.WithKeys("down", "j"), key.WithHelp("↓/j", "down"))
	KeySearch        = key.NewBinding(key.WithKeys("/"), key.WithHelp("/", "search"))
)
