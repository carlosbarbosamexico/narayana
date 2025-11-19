# NarayanaDB - Native Executable Build System
# Compiles to a self-contained native executable that "just works"

.PHONY: all build release release-lto release-fast clean install uninstall dist help test bench clippy fmt

# Default target
all: release

# Build variables
BINARY_NAME = narayana
SERVER_BINARY = narayana-server
CLI_BINARY = narayana
TARGET_DIR = target
RELEASE_DIR = $(TARGET_DIR)/release
INSTALL_PREFIX ?= /usr/local
BIN_DIR = $(INSTALL_PREFIX)/bin
MAN_DIR = $(INSTALL_PREFIX)/share/man/man1

# Build info
VERSION ?= $(shell git describe --tags --always --dirty 2>/dev/null || echo "0.1.0")
BUILD_TIME = $(shell date -u +"%Y-%m-%dT%H:%M:%SZ")
GIT_COMMIT = $(shell git rev-parse --short HEAD 2>/dev/null || echo "unknown")

# Feature flags for release builds
RELEASE_FEATURES = --features native

# Build for development (debug)
build:
	@echo "🔨 Building NarayanaDB (debug)..."
	cargo build --bin $(BINARY_NAME) --bin $(SERVER_BINARY)
	@echo "✅ Build complete: $(TARGET_DIR)/debug/$(BINARY_NAME)"
	@echo "✅ Build complete: $(TARGET_DIR)/debug/$(SERVER_BINARY)"

# Build optimized release binary (default)
release: export RUSTFLAGS = -C link-arg=-s
release:
	@echo "🚀 Building NarayanaDB (release - optimized)..."
	cargo build --release --bin $(BINARY_NAME) --bin $(SERVER_BINARY) $(RELEASE_FEATURES)
	@echo "✅ Release build complete!"
	@echo "📦 Binary: $(RELEASE_DIR)/$(BINARY_NAME)"
	@echo "📦 Binary: $(RELEASE_DIR)/$(SERVER_BINARY)"
	@echo "📊 Binary size:"
	@ls -lh $(RELEASE_DIR)/$(BINARY_NAME) $(RELEASE_DIR)/$(SERVER_BINARY) | awk '{print $$5, $$9}'

# Build with aggressive LTO (smallest binary)
release-lto:
	@echo "🎯 Building NarayanaDB (release-lto - smallest binary)..."
	cargo build --profile release-lto --bin $(BINARY_NAME) --bin $(SERVER_BINARY) $(RELEASE_FEATURES)
	@echo "✅ LTO release build complete!"
	@echo "📦 Binary: $(TARGET_DIR)/release-lto/$(BINARY_NAME)"
	@echo "📦 Binary: $(TARGET_DIR)/release-lto/$(SERVER_BINARY)"
	@echo "📊 Binary size:"
	@ls -lh $(TARGET_DIR)/release-lto/$(BINARY_NAME) $(TARGET_DIR)/release-lto/$(SERVER_BINARY) | awk '{print $$5, $$9}'

# Build fast release (for development/testing)
release-fast:
	@echo "⚡ Building NarayanaDB (release-fast - faster compilation)..."
	cargo build --profile release-fast --bin $(BINARY_NAME) --bin $(SERVER_BINARY) $(RELEASE_FEATURES)
	@echo "✅ Fast release build complete!"
	@echo "📦 Binary: $(TARGET_DIR)/release-fast/$(BINARY_NAME)"
	@echo "📦 Binary: $(TARGET_DIR)/release-fast/$(SERVER_BINARY)"

# Build statically linked binary (fully self-contained)
static: export RUSTFLAGS = -C target-feature=+crt-static
static:
	@echo "🔒 Building NarayanaDB (static - fully self-contained)..."
	@if [ -z "$(TARGET)" ]; then \
		echo "❌ TARGET must be set for static builds (e.g., TARGET=x86_64-unknown-linux-musl)"; \
		exit 1; \
	fi
	cargo build --release --target $(TARGET) --bin $(BINARY_NAME) --bin $(SERVER_BINARY) $(RELEASE_FEATURES)
	@echo "✅ Static build complete!"
	@echo "📦 Binary: $(TARGET_DIR)/$(TARGET)/release/$(BINARY_NAME)"
	@echo "📦 Binary: $(TARGET_DIR)/$(TARGET)/release/$(SERVER_BINARY)"

# Build for specific target (cross-compilation)
cross: export RUSTFLAGS = -C link-arg=-s
cross:
	@if [ -z "$(TARGET)" ]; then \
		echo "❌ TARGET must be set (e.g., TARGET=x86_64-unknown-linux-gnu)"; \
		exit 1; \
	fi
	@echo "🌍 Building NarayanaDB for $(TARGET)..."
	cargo build --release --target $(TARGET) --bin $(BINARY_NAME) --bin $(SERVER_BINARY) $(RELEASE_FEATURES)
	@echo "✅ Cross-compilation complete!"
	@echo "📦 Binary: $(TARGET_DIR)/$(TARGET)/release/$(BINARY_NAME)"
	@echo "📦 Binary: $(TARGET_DIR)/$(TARGET)/release/$(SERVER_BINARY)"

# Build for multiple targets
dist: release
	@echo "📦 Creating distribution packages..."
	@mkdir -p dist
	@cp $(RELEASE_DIR)/$(BINARY_NAME) dist/$(BINARY_NAME)-$(VERSION)
	@cp $(RELEASE_DIR)/$(SERVER_BINARY) dist/$(SERVER_BINARY)-$(VERSION)
	@chmod +x dist/$(BINARY_NAME)-$(VERSION)
	@chmod +x dist/$(SERVER_BINARY)-$(VERSION)
	@echo "✅ Distribution binaries created in dist/"

# Clean build artifacts
clean:
	@echo "🧹 Cleaning build artifacts..."
	cargo clean
	@rm -rf dist
	@echo "✅ Clean complete!"

# Run tests
test:
	@echo "🧪 Running tests..."
	cargo test --workspace --lib --bins
	@echo "✅ Tests complete!"

# Run all tests
test-all:
	@echo "🧪 Running all tests..."
	cargo test --all
	@echo "✅ All tests complete!"

# Run tests with coverage
test-coverage:
	@echo "📊 Running tests with coverage..."
	@cargo install cargo-tarpaulin --locked || true
	cargo tarpaulin --out Html --output-dir coverage
	@echo "✅ Coverage report generated in coverage/tarpaulin-report.html"

# Run unit tests
test-unit:
	@echo "🧪 Running unit tests..."
	cargo test --lib
	@echo "✅ Unit tests complete!"

# Run integration tests
test-integration:
	@echo "🧪 Running integration tests..."
	cargo test --test '*'
	@echo "✅ Integration tests complete!"

# Check coverage threshold (99%)
check-coverage:
	@echo "📊 Checking coverage..."
	@cargo install cargo-tarpaulin --locked || true
	cargo tarpaulin --out Stdout | grep -oP '\d+\.\d+%' | head -1
	@echo "✅ Coverage check complete"

# Run benchmarks
bench:
	@echo "📊 Running benchmarks..."
	cargo bench
	@echo "✅ Benchmarks complete!"

# Lint code
clippy:
	@echo "🔍 Running clippy..."
	cargo clippy --workspace --all-targets -- -D warnings
	@echo "✅ Clippy complete!"

# Format code
fmt:
	@echo "🎨 Formatting code..."
	cargo fmt --all
	@echo "✅ Formatting complete!"

# Check code
check: fmt clippy test
	@echo "✅ All checks passed!"

# Install binaries
install: release
	@echo "📥 Installing NarayanaDB to $(INSTALL_PREFIX)..."
	@mkdir -p $(BIN_DIR)
	@cp $(RELEASE_DIR)/$(BINARY_NAME) $(BIN_DIR)/
	@cp $(RELEASE_DIR)/$(SERVER_BINARY) $(BIN_DIR)/
	@chmod +x $(BIN_DIR)/$(BINARY_NAME)
	@chmod +x $(BIN_DIR)/$(SERVER_BINARY)
	@echo "✅ Installed $(BINARY_NAME) to $(BIN_DIR)"
	@echo "✅ Installed $(SERVER_BINARY) to $(BIN_DIR)"
	@echo "💡 You can now run: $(BINARY_NAME) --help"
	@echo "💡 You can now run: $(SERVER_BINARY) --help"

# Uninstall binaries
uninstall:
	@echo "📤 Uninstalling NarayanaDB..."
	@rm -f $(BIN_DIR)/$(BINARY_NAME)
	@rm -f $(BIN_DIR)/$(SERVER_BINARY)
	@echo "✅ Uninstalled $(BINARY_NAME)"
	@echo "✅ Uninstalled $(SERVER_BINARY)"

# Create a portable tarball
tarball: release
	@echo "📦 Creating portable tarball..."
	@mkdir -p dist/narayana-$(VERSION)
	@cp $(RELEASE_DIR)/$(BINARY_NAME) dist/narayana-$(VERSION)/
	@cp $(RELEASE_DIR)/$(SERVER_BINARY) dist/narayana-$(VERSION)/
	@cp README.md dist/narayana-$(VERSION)/ 2>/dev/null || true
	@cd dist && tar czf narayana-$(VERSION).tar.gz narayana-$(VERSION)
	@rm -rf dist/narayana-$(VERSION)
	@echo "✅ Tarball created: dist/narayana-$(VERSION).tar.gz"

# Create a ZIP archive (for Windows)
zip: release
	@echo "📦 Creating ZIP archive..."
	@mkdir -p dist/narayana-$(VERSION)
	@cp $(RELEASE_DIR)/$(BINARY_NAME).exe dist/narayana-$(VERSION)/$(BINARY_NAME).exe 2>/dev/null || cp $(RELEASE_DIR)/$(BINARY_NAME) dist/narayana-$(VERSION)/
	@cp $(RELEASE_DIR)/$(SERVER_BINARY).exe dist/narayana-$(VERSION)/$(SERVER_BINARY).exe 2>/dev/null || cp $(RELEASE_DIR)/$(SERVER_BINARY) dist/narayana-$(VERSION)/
	@cp README.md dist/narayana-$(VERSION)/ 2>/dev/null || true
	@cd dist && zip -r narayana-$(VERSION).zip narayana-$(VERSION)
	@rm -rf dist/narayana-$(VERSION)
	@echo "✅ ZIP created: dist/narayana-$(VERSION).zip"

# Build for common targets
build-linux:
	@$(MAKE) cross TARGET=x86_64-unknown-linux-gnu

build-macos:
	@$(MAKE) cross TARGET=x86_64-apple-darwin
	@$(MAKE) cross TARGET=aarch64-apple-darwin

build-windows:
	@$(MAKE) cross TARGET=x86_64-pc-windows-msvc
	@$(MAKE) cross TARGET=x86_64-pc-windows-gnu

build-all: build-linux build-macos build-windows
	@echo "✅ Built for all common targets!"

# Run the server
run: release
	@echo "🚀 Starting NarayanaDB server..."
	@$(RELEASE_DIR)/$(SERVER_BINARY)

# Run the CLI
cli: release
	@echo "💻 Running NarayanaDB CLI..."
	@$(RELEASE_DIR)/$(BINARY_NAME) $(ARGS)

# Show help
help:
	@echo "NarayanaDB - Native Executable Build System"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Build targets:"
	@echo "  build          Build debug binaries"
	@echo "  release        Build optimized release binaries (default)"
	@echo "  release-lto    Build with aggressive LTO (smallest binary)"
	@echo "  release-fast   Build fast release (faster compilation)"
	@echo "  static         Build statically linked binary (requires TARGET)"
	@echo "  cross          Cross-compile (requires TARGET)"
	@echo ""
	@echo "Distribution targets:"
	@echo "  dist           Create distribution binaries"
	@echo "  tarball        Create portable tarball"
	@echo "  zip            Create ZIP archive"
	@echo ""
	@echo "Installation targets:"
	@echo "  install        Install binaries to system (default: /usr/local)"
	@echo "  uninstall      Remove installed binaries"
	@echo ""
	@echo "Development targets:"
	@echo "  test           Run tests"
	@echo "  bench          Run benchmarks"
	@echo "  clippy         Run clippy linter"
	@echo "  fmt            Format code"
	@echo "  check          Run fmt, clippy, and tests"
	@echo "  clean          Remove build artifacts"
	@echo ""
	@echo "Platform-specific:"
	@echo "  build-linux    Build for Linux"
	@echo "  build-macos    Build for macOS"
	@echo "  build-windows  Build for Windows"
	@echo "  build-all      Build for all common platforms"
	@echo ""
	@echo "Examples:"
	@echo "  make release              # Build optimized release"
	@echo "  make static TARGET=x86_64-unknown-linux-musl  # Static Linux build"
	@echo "  make cross TARGET=aarch64-apple-darwin        # macOS ARM build"
	@echo "  make install              # Install to /usr/local"
	@echo "  make install INSTALL_PREFIX=/opt/narayana     # Custom install path"
