package downloader

import (
	"fmt"
	"io"
	"net/http"
	"net/url"
	"os"
	"path/filepath"
	"strings"
	"time"
)

type Downloader struct {
	client      *http.Client
	downloadDir string
}

func New(downloadDir string) *Downloader {
	return &Downloader{
		client: &http.Client{
			Timeout: 300 * time.Second,
			Transport: &http.Transport{
				MaxIdleConns:       10,
				IdleConnTimeout:    30 * time.Second,
				DisableCompression: false,
			},
		},
		downloadDir: downloadDir,
	}
}

func (d *Downloader) Download(dlURL, filename string, progressFunc func(current, total int64)) (string, error) {
	// Create download directory
	if err := os.MkdirAll(d.downloadDir, 0755); err != nil {
		return "", fmt.Errorf("failed to create download directory: %w", err)
	}

	// Prepare request
	req, err := http.NewRequest("GET", dlURL, nil)
	if err != nil {
		return "", fmt.Errorf("failed to create request: %w", err)
	}
	req.Header.Set("User-Agent", "Mozilla/5.0 (compatible; anna-dl-go/1.0)")

	// Execute request
	resp, err := d.client.Do(req)
	if err != nil {
		return "", fmt.Errorf("failed to start download: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return "", fmt.Errorf("HTTP error: %s", resp.Status)
	}

	// Determine filename
	if filename == "" {
		filename = d.determineFilename(dlURL, resp.Header.Get("Content-Disposition"))
	}

	// Check for valid extension
	ext := strings.ToLower(filepath.Ext(filename))
	if ext == "" || len(ext) > 5 {
		contentType := resp.Header.Get("Content-Type")
		if strings.Contains(contentType, "pdf") {
			filename += ".pdf"
		} else if strings.Contains(contentType, "epub") {
			filename += ".epub"
		} else {
			filename += ".download"
		}
	}

	// Create file
	filepath := filepath.Join(d.downloadDir, filename)
	file, err := os.Create(filepath)
	if err != nil {
		return "", fmt.Errorf("failed to create file: %w", err)
	}
	defer file.Close()

	// Get content length
	contentLength := resp.ContentLength

	// Download with progress
	buffer := make([]byte, 32*1024) // 32KB buffer
	n, err := io.CopyBuffer(file, &progressReader{
		reader:       resp.Body,
		contentLength: contentLength,
		progressFunc: progressFunc,
	}, buffer)
	
	if err != nil {
		os.Remove(filepath) // Clean up on error
		return "", fmt.Errorf("download failed: %w", err)
	}

	if n == 0 {
		os.Remove(filepath)
		return "", fmt.Errorf("downloaded file is empty")
	}

	return filepath, nil
}

func (d *Downloader) determineFilename(dlURL, contentDisposition string) string {
	// Try to extract from Content-Disposition first
	if contentDisposition != "" {
		if name := parseContentDisposition(contentDisposition); name != "" {
			return sanitizeFilename(name)
		}
	}

	// Try to extract from URL
	parsedURL, err := url.Parse(dlURL)
	if err == nil {
		path := strings.TrimPrefix(parsedURL.Path, "/")
		parts := strings.Split(path, "/")
		if len(parts) > 0 {
			filename := parts[len(parts)-1]
			if filename != "" && !strings.Contains(filename, "?") {
				decoded, _ := url.QueryUnescape(filename)
				if len(decoded) < 200 {
					return sanitizeFilename(decoded)
				}
			}
		}
	}

	// Fallback
	return fmt.Sprintf("download_%d.tmp", time.Now().Unix())
}

func parseContentDisposition(header string) string {
	parts := strings.Split(header, ";")
	for _, part := range parts {
		part = strings.TrimSpace(part)
		if strings.HasPrefix(part, "filename=") {
			filename := strings.TrimPrefix(part, "filename=")
			filename = strings.Trim(filename, "\"")
			return filename
		}
		if strings.HasPrefix(part, "filename*=UTF-8''") {
			filename := strings.TrimPrefix(part, "filename*=UTF-8''")
			decoded, _ := url.QueryUnescape(filename)
			return decoded
		}
	}
	return ""
}

func sanitizeFilename(filename string) string {
	// Remove path separators and other problematic characters
	replacer := strings.NewReplacer(
		"/", "_",
		"\\", "_",
		":", "_",
		"*", "_",
		"?", "_",
		"\"", "_",
		"<", "_",
		">", "_",
		"|", "_",
	)
	return replacer.Replace(filename)
}

// progressReader wraps an io.Reader to track download progress
type progressReader struct {
	reader        io.Reader
	contentLength int64
	progressFunc  func(current, total int64)
	current       int64
}

func (pr *progressReader) Read(p []byte) (int, error) {
	n, err := pr.reader.Read(p)
	pr.current += int64(n)
	
	if pr.progressFunc != nil && pr.contentLength > 0 {
		pr.progressFunc(pr.current, pr.contentLength)
	}
	
	return n, err
}

// GetProgressBar returns a simple text progress representation
func GetProgressBar(current, total int64, width int) string {
	if total <= 0 {
		return "[░░░░░░░░░░]"
	}
	
	percent := float64(current) / float64(total)
	filled := int(percent * float64(width))
	
	if filled > width {
		filled = width
	}
	
	bar := strings.Repeat("█", filled) + strings.Repeat("░", width-filled)
	percentage := int(percent * 100)
	
	return fmt.Sprintf("[%s] %d%% (%s/%s)", 
		bar, 
		percentage,
		formatBytes(current),
		formatBytes(total))
}

func formatBytes(bytes int64) string {
	const unit = 1024
	if bytes < unit {
		return fmt.Sprintf("%d B", bytes)
	}
	div, exp := int64(unit), 0
	for n := bytes / unit; n >= unit; n /= unit {
		div *= unit
		exp++
	}
	return fmt.Sprintf("%.1f %cB", float64(bytes)/float64(div), "KMGTPE"[exp])
}
