# Crystal Clear Makefile
# Build targets for both Rust TUI and Swift macOS app

.PHONY: all rust-lib swift-bindings xcframework app clean help

# Default target
all: app

# Build Rust library for native architecture
rust-lib:
	@echo "Building Rust library..."
	cargo build --release

# Build Rust library for all macOS targets and generate Swift bindings
swift-bindings:
	@echo "Building XCFramework and Swift bindings..."
	./scripts/build-xcframework.sh

# Alias for swift-bindings
xcframework: swift-bindings

# Build the Swift app (requires Xcode)
app: swift-bindings
	@echo "Building Swift app..."
	xcodebuild -project crystalclear/crystalclear.xcodeproj \
		-scheme crystalclear \
		-configuration Release \
		-derivedDataPath build/DerivedData \
		build

# Run the Rust TUI
run-tui:
	cargo run --release

# Run tests
test:
	cargo test

# Clean all build artifacts
clean:
	cargo clean
	rm -rf build/
	rm -rf crystalclear/CrystalClearCore.xcframework
	rm -rf target/release-macos

# Help
help:
	@echo "Crystal Clear Build Targets:"
	@echo ""
	@echo "  make              - Build the Swift macOS app (default)"
	@echo "  make rust-lib     - Build just the Rust library"
	@echo "  make swift-bindings - Generate Swift bindings and XCFramework"
	@echo "  make xcframework  - Alias for swift-bindings"
	@echo "  make app          - Build the Swift app"
	@echo "  make run-tui      - Run the Rust TUI"
	@echo "  make test         - Run Rust tests"
	@echo "  make clean        - Clean all build artifacts"
	@echo ""
	@echo "For manual Xcode setup, see SWIFT_INTEGRATION.md"
