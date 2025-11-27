.PHONY: build run test clean install deps

# Binary name
BINARY_NAME=annadl
BINARY_WINDOWS=$(BINARY_NAME).exe
BINARY_LINUX=$(BINARY_NAME)-linux
BINARY_MACOS=$(BINARY_NAME)-macos

# Build flags
LDFLAGS=-ldflags "-w -s"

# Go parameters
GOCMD=go
GOBUILD=$(GOCMD) build
GOCLEAN=$(GOCMD) clean
GOTEST=$(GOCMD) test
GOGET=$(GOCMD) get
GOMOD=$(GOCMD) mod

# Build directories
DIST_DIR=dist
BUILD_DIR=build

all: clean deps test build

deps:
	@echo "Downloading dependencies..."
	$(GOMOD) tidy
	$(GOMOD) download

build:
	@echo "Building $(BINARY_NAME)..."
	@mkdir -p $(BUILD_DIR)
	$(GOBUILD) $(LDFLAGS) -o $(BUILD_DIR)/$(BINARY_NAME) .

build-windows:
	@echo "Building for Windows..."
	@mkdir -p $(BUILD_DIR)
	GOOS=windows GOARCH=amd64 $(GOBUILD) $(LDFLAGS) -o $(BUILD_DIR)/$(BINARY_WINDOWS) .

build-linux:
	@echo "Building for Linux..."
	@mkdir -p $(BUILD_DIR)
	GOOS=linux GOARCH=amd64 $(GOBUILD) $(LDFLAGS) -o $(BUILD_DIR)/$(BINARY_LINUX) .

build-macos:
	@echo "Building for macOS..."
	@mkdir -p $(BUILD_DIR)
	GOOS=darwin GOARCH=amd64 $(GOBUILD) $(LDFLAGS) -o $(BUILD_DIR)/$(BINARY_MACOS) .

build-all: build-windows build-linux build-macos

run:
	$(GOCMD) run main.go

test:
	@echo "Running tests..."
	$(GOTEST) -v ./...

clean:
	@echo "Cleaning..."
	$(GOCLEAN)
	@rm -rf $(BUILD_DIR)
	@rm -rf $(DIST_DIR)

install: build
	@echo "Installing $(BINARY_NAME)..."
	@cp $(BUILD_DIR)/$(BINARY_NAME) /usr/local/bin/$(BINARY_NAME) 2>/dev/null || echo "Please run with sudo or add $(BUILD_DIR) to your PATH"

uninstall:
	@echo "Uninstalling $(BINARY_NAME)..."
	@rm -f /usr/local/bin/$(BINARY_NAME)

# Development commands
fmt:
	@echo "Formatting code..."
	@go fmt ./...

lint:
	@echo "Running linter..."
	@golangci-lint run || echo "Please install golangci-lint: https://golangci-lint.run/usage/install/"

deps-upgrade:
	@echo "Upgrading dependencies..."
	$(GOMOD) tidy -go=1.21

# Release commands
release: clean deps test build-all
	@echo "Creating release packages..."
	@mkdir -p $(DIST_DIR)
	@cd $(BUILD_DIR) && zip ../$(DIST_DIR)/$(BINARY_NAME)-windows-amd64.zip $(BINARY_WINDOWS)
	@cd $(BUILD_DIR) && tar -czf ../$(DIST_DIR)/$(BINARY_NAME)-linux-amd64.tar.gz $(BINARY_LINUX)
	@cd $(BUILD_DIR) && tar -czf ../$(DIST_DIR)/$(BINARY_NAME)-macos-amd64.tar.gz $(BINARY_MACOS)
	@echo "Release packages created in $(DIST_DIR)/"

# Help
help:
	@echo "Available commands:"
	@echo "  make build        - Build the binary for current platform"
	@echo "  make build-all    - Build for Windows, Linux, and macOS"
	@echo "  make run          - Run the application"
	@echo "  make test         - Run tests"
	@echo "  make clean        - Clean build artifacts"
	@echo "  make deps         - Download dependencies"
	@echo "  make install      - Install binary to /usr/local/bin"
	@echo "  make release      - Create release packages"
	@echo "  make help         - Show this help"
