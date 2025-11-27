package scraper

import (
	"fmt"
	"net/http"
	"net/http/httptest"
	"testing"
)

func TestSearch(t *testing.T) {
	// Mock server
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/search" {
			t.Errorf("Expected path /search, got %s", r.URL.Path)
		}

		html := `
		<html>
		<body>
			<div class="h-[125] flex flex-col justify-center">
				<div class="relative top-[-10]">
					<h3 class="text-xl font-bold">
						<a href="/md5/123456" class="js-vim-focus custom-a">Test Book Title</a>
					</h3>
					<div class="text-sm">
						Test Author [en], epub, 1.2MB, 2023
					</div>
				</div>
			</div>
			<div class="h-[125] flex flex-col justify-center">
				<div class="relative top-[-10]">
					<h3 class="text-xl font-bold">
						<a href="/md5/789012" class="js-vim-focus custom-a">Another Book</a>
					</h3>
					<div class="text-sm">
						Another Author [fr], pdf, 2.5MB, 2022
					</div>
				</div>
			</div>
		</body>
		</html>
		`
		w.Header().Set("Content-Type", "text/html")
		fmt.Fprint(w, html)
	}))
	defer server.Close()

	scraper := New()
	scraper.BaseURL = server.URL

	books, err := scraper.Search("test", 10)
	if err != nil {
		t.Fatalf("Search failed: %v", err)
	}

	if len(books) != 2 {
		t.Errorf("Expected 2 books, got %d", len(books))
	}

	book := books[0]
	if book.Title != "Test Book Title" {
		t.Errorf("Expected title 'Test Book Title', got '%s'", book.Title)
	}
	if book.Author != "Test Author" {
		t.Errorf("Expected author 'Test Author', got '%s'", book.Author)
	}
	if book.Year != "2023" {
		t.Errorf("Expected year '2023', got '%s'", book.Year)
	}
	if book.Language != "en" {
		t.Errorf("Expected language 'en', got '%s'", book.Language)
	}
	if book.Format != "EPUB" {
		t.Errorf("Expected format 'EPUB', got '%s'", book.Format)
	}
}

func TestGetBookDetails(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		html := `
		<html>
		<body>
			<div id="external-downloads">
				<a href="http://libgen.rs/book/123456">Libgen.rs</a>
				<a href="http://example.com/download">Direct Download</a>
			</div>
		</body>
		</html>
		`
		w.Header().Set("Content-Type", "text/html")
		fmt.Fprint(w, html)
	}))
	defer server.Close()

	scraper := New()
	// GetBookDetails takes a full URL, so BaseURL doesn't matter for the request itself,
	// but we pass the server URL.

	links, err := scraper.GetBookDetails(server.URL + "/md5/123456")
	if err != nil {
		t.Fatalf("GetBookDetails failed: %v", err)
	}

	if len(links) != 2 {
		t.Errorf("Expected 2 links, got %d", len(links))
	}

	if links[0].Text != "Libgen.rs" {
		t.Errorf("Expected link text 'Libgen.rs', got '%s'", links[0].Text)
	}
	if links[0].Source != "LibGen" {
		t.Errorf("Expected source 'LibGen', got '%s'", links[0].Source)
	}
}

func TestExtractFormat(t *testing.T) {
	s := New()

	tests := []struct {
		input    string
		expected string
	}{
		{"Some book info, EPUB, 1MB", "EPUB"},
		{"Some book info, PDF, 2MB", "PDF"},
		{"Some book info, epub, 1MB", "EPUB"},
	}

	for _, tt := range tests {
		got := s.extractFormat(tt.input)
		if got != tt.expected {
			t.Errorf("extractFormat(%q) = %q, want %q", tt.input, got, tt.expected)
		}
	}
}
