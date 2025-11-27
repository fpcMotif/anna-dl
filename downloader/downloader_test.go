package downloader

import (
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"testing"
)

func TestSanitizeFilename(t *testing.T) {
	tests := []struct {
		input    string
		expected string
	}{
		{"normal.pdf", "normal.pdf"},
		{"path/to/file.pdf", "path_to_file.pdf"},
		{"invalid:chars?.pdf", "invalid_chars_.pdf"},
		{"< > | *.pdf", "_ _ _ _.pdf"},
	}

	for _, tt := range tests {
		got := sanitizeFilename(tt.input)
		if got != tt.expected {
			t.Errorf("sanitizeFilename(%q) = %q, want %q", tt.input, got, tt.expected)
		}
	}
}

func TestParseContentDisposition(t *testing.T) {
	tests := []struct {
		input    string
		expected string
	}{
		{`attachment; filename="test.pdf"`, "test.pdf"},
		{`attachment; filename=test.pdf`, "test.pdf"},
		{`attachment; filename*=UTF-8''%e2%82%ac.pdf`, "€.pdf"}, // Euro symbol encoded
		{`inline`, ""},
	}

	for _, tt := range tests {
		got := parseContentDisposition(tt.input)
		if got != tt.expected {
			t.Errorf("parseContentDisposition(%q) = %q, want %q", tt.input, got, tt.expected)
		}
	}
}

func TestDownload(t *testing.T) {
	// Mock server
	content := "Hello World PDF Content"
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/pdf")
		w.Header().Set("Content-Length", fmt.Sprintf("%d", len(content)))
		fmt.Fprint(w, content)
	}))
	defer server.Close()

	// Temp dir for downloads
	tempDir := t.TempDir()

	d := New(tempDir)

	filename := "test_download.pdf"
	path, err := d.Download(server.URL, filename, nil)
	if err != nil {
		t.Fatalf("Download failed: %v", err)
	}

	// Verify file existence
	if _, err := os.Stat(path); os.IsNotExist(err) {
		t.Errorf("File not found at %s", path)
	}

	// Verify content
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("Failed to read file: %v", err)
	}
	if string(data) != content {
		t.Errorf("File content mismatch. Got %s, want %s", string(data), content)
	}
}

func TestDetermineFilename(t *testing.T) {
	d := &Downloader{}

	tests := []struct {
		url      string
		cd       string
		expected string
	}{
		{"http://example.com/test.pdf", "", "test.pdf"},
		{"http://example.com/download", `attachment; filename="from_header.epub"`, "from_header.epub"},
		{"http://example.com/download/weird%20name.pdf", "", "weird name.pdf"},
	}

	for _, tt := range tests {
		got := d.determineFilename(tt.url, tt.cd)
		if got != tt.expected {
			t.Errorf("determineFilename(%q, %q) = %q, want %q", tt.url, tt.cd, got, tt.expected)
		}
	}
}
