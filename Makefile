# Makefile for cc_auto_switch - Cross-platform build support

# Variables
CARGO := cargo
UPX := upx
TARGET_DIR := /Users/jingzhao/target-rust/release
BINARY_NAME := cc_auto_switch
RELEASE_BINARY := $(TARGET_DIR)/$(BINARY_NAME)
PACKED_BINARY := $(TARGET_DIR)/$(BINARY_NAME)-packed

# Cross-platform compilation targets
TARGETS := \
	x86_64-unknown-linux-musl \
	x86_64-pc-windows-gnu \
	x86_64-apple-darwin \
	aarch64-apple-darwin

# Output directory for cross-platform builds
DIST_DIR := dist

# Detect OS
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Linux)
    HOST_OS := linux
    HOST_EXT :=
endif
ifeq ($(UNAME_S),Darwin)
    HOST_OS := macos
    HOST_EXT :=
endif
ifeq ($(OS),Windows_NT)
    HOST_OS := windows
    HOST_EXT := .exe
endif

# Binary name with extension
BINARY_NAME_WITH_EXT := $(BINARY_NAME)$(HOST_EXT)

# Default target
.PHONY: all
all: build

# Build in release mode for current platform
.PHONY: build
build:
	$(CARGO) build --release

# Build and pack with UPX for current platform
.PHONY: pack
pack: build
	@echo "Packing binary with UPX..."
	$(UPX) --best --ultra-brute -o $(PACKED_BINARY) $(RELEASE_BINARY)
	@echo "Packed binary: $(PACKED_BINARY)"

# Build and install for current platform
.PHONY: install
install: build
	$(CARGO) install --path .

# Cross-platform build targets
.PHONY: build-all
build-all: $(addprefix build-,$(TARGETS))

.PHONY: build-linux
build-linux: build-x86_64-unknown-linux-musl

.PHONY: build-macos
build-macos: build-x86_64-apple-darwin build-aarch64-apple-darwin

.PHONY: build-windows
build-windows: build-x86_64-pc-windows-gnu

# Build for specific target
.PHONY: build-%
build-%:
	@echo "Building for target $*..."
	$(CARGO) build --release --target $*

# Package all targets
.PHONY: package-all
package-all: build-all
	@echo "Packaging all targets..."
	@mkdir -p $(DIST_DIR)
	@for target in $(TARGETS); do \
		echo "Packaging $$target..."; \
		mkdir -p $(DIST_DIR)/$$target; \
		if echo "$$target" | grep -q "windows"; then \
			ext=".exe"; \
		else \
			ext=""; \
		fi; \
		if [ -f "target/$$target/release/$(BINARY_NAME)$$ext" ]; then \
			cp "target/$$target/release/$(BINARY_NAME)$$ext" "$(DIST_DIR)/$$target/$(BINARY_NAME)$$ext"; \
			cd "$(DIST_DIR)/$$target" && tar -czf "../$(BINARY_NAME)-$$target.tar.gz" "$(BINARY_NAME)$$ext" && cd ../../; \
			echo "  Created $(DIST_DIR)/$(BINARY_NAME)-$$target.tar.gz"; \
		fi; \
	done

# Package specific target
.PHONY: package-%
package-%: build-%
	@echo "Packaging target $*..."
	@mkdir -p $(DIST_DIR)/$*
	@if echo "$*" | grep -q "windows"; then \
		ext=".exe"; \
	else \
		ext=""; \
	fi; \
	if [ -f "target/$*/release/$(BINARY_NAME)$$ext" ]; then \
		cp "target/$*/release/$(BINARY_NAME)$$ext" "$(DIST_DIR)/$*/$(BINARY_NAME)$$ext"; \
		cd "$(DIST_DIR)/$*" && tar -czf "../$(BINARY_NAME)-$*.tar.gz" "$(BINARY_NAME)$$ext" && cd ../../; \
		echo "Created $(DIST_DIR)/$(BINARY_NAME)-$*.tar.gz"; \
	fi

# Create release packages
.PHONY: release
release: package-all
	@echo "Creating release packages..."
	@echo "Release packages created in $(DIST_DIR)/"
	@ls -la $(DIST_DIR)/*.tar.gz

# Clean build artifacts
.PHONY: clean
clean:
	$(CARGO) clean
	@rm -rf $(DIST_DIR)

# Clean and rebuild for current platform
.PHONY: rebuild
rebuild: clean build

# Clean and rebuild all targets
.PHONY: rebuild-all
rebuild-all: clean build-all

# Check if UPX is installed
.PHONY: check-upx
check-upx:
	@command -v $(UPX) >/dev/null 2>&1 || { echo "UPX is not installed. Please install UPX to use packing features."; exit 1; }

# Show binary sizes for current platform
.PHONY: sizes
sizes: build
	@echo "Binary sizes:"
	@echo "Original: $$(ls -lh $(RELEASE_BINARY) | awk '{print $$5}')"
	@if [ -f $(PACKED_BINARY) ]; then \
		echo "Packed:   $$(ls -lh $(PACKED_BINARY) | awk '{print $$5}')"; \
		echo "Compression ratio: $$(( 100 - $$(stat -c%s $(PACKED_BINARY) * 100 / $$(stat -c%s $(RELEASE_BINARY)) )) )%"; \
	fi

# Show binary sizes for all targets
.PHONY: sizes-all
sizes-all: build-all
	@echo "Binary sizes for all targets:"
	@for target in $(TARGETS); do \
		if echo "$$target" | grep -q "windows"; then \
			ext=".exe"; \
		else \
			ext=""; \
		fi; \
		if [ -f "target/$$target/release/$(BINARY_NAME)$$ext" ]; then \
			echo "  $$target: $$(ls -lh "target/$$target/release/$(BINARY_NAME)$$ext" | awk '{print $$5}')"; \
		fi; \
	done

# Run the binary for current platform
.PHONY: run
run: build
	$(RELEASE_BINARY)

# Run the packed binary for current platform
.PHONY: run-packed
run-packed: pack
	$(PACKED_BINARY)

# Development targets
.PHONY: dev
dev:
	$(CARGO) run

# Test
.PHONY: test
test:
	$(CARGO) test

# Check code
.PHONY: check
check:
	$(CARGO) check

# Format code
.PHONY: fmt
fmt:
	$(CARGO) fmt

# Lint code
.PHONY: lint
lint:
	$(CARGO) clippy

# Update dependencies
.PHONY: update
update:
	$(CARGO) update

# Publish targets
.PHONY: publish
publish:
	@echo "Publishing to GitHub Packages..."
	$(CARGO) publish
	@echo "Published to GitHub Packages successfully!"

.PHONY: publish-dry-run
publish-dry-run:
	@echo "Running dry-run publish to GitHub Packages..."
	$(CARGO) publish --dry-run
	@echo "Dry-run publish completed successfully!"

# Show help
.PHONY: help
help:
	@echo "Available targets:"
	@echo "  Build targets:"
	@echo "    all             - Build in release mode for current platform (default)"
	@echo "    build           - Build in release mode for current platform"
	@echo "    build-all       - Build for all target platforms"
	@echo "    build-linux     - Build for Linux targets (x86_64, aarch64)"
	@echo "    build-macos     - Build for macOS targets (x86_64, aarch64)"
	@echo "    build-windows   - Build for Windows targets (x86_64)"
	@echo "    build-TARGET    - Build for specific target (e.g., build-x86_64-unknown-linux-gnu)"
	@echo ""
	@echo "  Package targets:"
	@echo "    pack            - Build and pack with UPX for current platform"
	@echo "    package-all     - Package all targets as tar.gz files"
	@echo "    package-TARGET  - Package specific target as tar.gz file"
	@echo "    release         - Create complete release packages"
	@echo ""
	@echo "  Install targets:"
	@echo "    install         - Build and install binary for current platform"
	@echo ""
	@echo "  Publish targets:"
	@echo "    publish         - Publish to GitHub Packages"
	@echo "    publish-dry-run - Dry-run publish to GitHub Packages"
	@echo ""
	@echo "  Clean targets:"
	@echo "    clean           - Clean build artifacts"
	@echo "    rebuild         - Clean and rebuild for current platform"
	@echo "    rebuild-all     - Clean and rebuild all targets"
	@echo ""
	@echo "  Utility targets:"
	@echo "    check-upx       - Check if UPX is installed"
	@echo "    sizes           - Show binary sizes for current platform"
	@echo "    sizes-all       - Show binary sizes for all targets"
	@echo "    run             - Run the release binary"
	@echo "    run-packed      - Run the packed binary"
	@echo "    dev             - Run in development mode"
	@echo "    test            - Run tests"
	@echo "    check           - Check code compilation"
	@echo "    fmt             - Format code"
	@echo "    lint            - Lint code"
	@echo "    update          - Update dependencies"
	@echo "    help            - Show this help message"
	@echo ""
	@echo "  Supported targets:"
	@echo "    $(TARGETS)"
	@echo ""
	@echo "  Examples:"
	@echo "    make build-all                    # Build for all platforms"
	@echo "    make build-linux                  # Build for Linux only"
	@echo "    make package-x86_64-unknown-linux-gnu  # Package specific Linux target"
	@echo "    make release                      # Create complete release"
	@echo "    make publish                      # Publish to GitHub Packages"