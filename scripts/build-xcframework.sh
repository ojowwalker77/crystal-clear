#!/bin/bash
# Build XCFramework for Crystal Clear macOS app
# This script builds the Rust library for Intel and Apple Silicon,
# creates a universal binary, and generates Swift bindings.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
SWIFT_PROJECT="$PROJECT_ROOT/crystalclear/crystalclear"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== Building Crystal Clear XCFramework ===${NC}"

# Detect host architecture
HOST_ARCH=$(uname -m)
if [ "$HOST_ARCH" = "arm64" ]; then
    HOST_TARGET="aarch64-apple-darwin"
else
    HOST_TARGET="x86_64-apple-darwin"
fi

echo -e "${YELLOW}Building for $HOST_TARGET...${NC}"
cargo build --release --target "$HOST_TARGET"

# Try to build for other architecture if available (for universal binary)
UNIVERSAL_BUILD=false
if [ "$HOST_ARCH" = "arm64" ]; then
    OTHER_TARGET="x86_64-apple-darwin"
else
    OTHER_TARGET="aarch64-apple-darwin"
fi

if cargo build --release --target "$OTHER_TARGET" 2>/dev/null; then
    UNIVERSAL_BUILD=true
    echo -e "${GREEN}Built for both architectures - creating universal binary${NC}"
else
    echo -e "${YELLOW}Cross-compilation target not available, using native architecture only${NC}"
fi

# Create output directory
OUTPUT_DIR="$PROJECT_ROOT/target/release-macos"
mkdir -p "$OUTPUT_DIR"

if [ "$UNIVERSAL_BUILD" = true ]; then
    # Create universal binary with lipo
    echo -e "${YELLOW}Creating universal binary...${NC}"
    lipo -create \
        "$PROJECT_ROOT/target/x86_64-apple-darwin/release/libcrystal_clear_core.a" \
        "$PROJECT_ROOT/target/aarch64-apple-darwin/release/libcrystal_clear_core.a" \
        -output "$OUTPUT_DIR/libcrystal_clear_core.a"
    ARCH_STRING="arm64_x86_64"
else
    # Copy native binary
    cp "$PROJECT_ROOT/target/$HOST_TARGET/release/libcrystal_clear_core.a" "$OUTPUT_DIR/"
    if [ "$HOST_ARCH" = "arm64" ]; then
        ARCH_STRING="arm64"
    else
        ARCH_STRING="x86_64"
    fi
fi

echo -e "${GREEN}Static library created at: $OUTPUT_DIR/libcrystal_clear_core.a${NC}"

# Generate Swift bindings
echo -e "${YELLOW}Generating Swift bindings...${NC}"
GENERATED_DIR="$SWIFT_PROJECT/Generated"
mkdir -p "$GENERATED_DIR"

# Run uniffi-bindgen to generate Swift code using the dylib
DYLIB_PATH="$PROJECT_ROOT/target/$HOST_TARGET/release/libcrystal_clear_core.dylib"
cargo run --bin uniffi-bindgen generate \
    --library "$DYLIB_PATH" \
    --language swift \
    --out-dir "$GENERATED_DIR"

echo -e "${GREEN}Swift bindings generated at: $GENERATED_DIR${NC}"

# Create XCFramework directory structure
XCFRAMEWORK_DIR="$PROJECT_ROOT/crystalclear/CrystalClearCore.xcframework"
rm -rf "$XCFRAMEWORK_DIR"
FRAMEWORK_ARCH_DIR="$XCFRAMEWORK_DIR/macos-$ARCH_STRING"
mkdir -p "$FRAMEWORK_ARCH_DIR"

# Copy the static library
cp "$OUTPUT_DIR/libcrystal_clear_core.a" "$FRAMEWORK_ARCH_DIR/"

# Create module map - module name must match what UniFFI Swift bindings expect
cat > "$FRAMEWORK_ARCH_DIR/module.modulemap" << 'EOF'
module crystal_clear_coreFFI {
    header "crystal_clear_coreFFI.h"
    export *
}
EOF

# Copy the generated header if it exists
if [ -f "$GENERATED_DIR/crystal_clear_coreFFI.h" ]; then
    cp "$GENERATED_DIR/crystal_clear_coreFFI.h" "$FRAMEWORK_ARCH_DIR/"
fi

# Build architecture array for Info.plist
if [ "$ARCH_STRING" = "arm64_x86_64" ]; then
    ARCH_ARRAY="<string>arm64</string>
                <string>x86_64</string>"
elif [ "$ARCH_STRING" = "arm64" ]; then
    ARCH_ARRAY="<string>arm64</string>"
else
    ARCH_ARRAY="<string>x86_64</string>"
fi

# Create Info.plist for XCFramework
cat > "$XCFRAMEWORK_DIR/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>AvailableLibraries</key>
    <array>
        <dict>
            <key>LibraryIdentifier</key>
            <string>macos-$ARCH_STRING</string>
            <key>LibraryPath</key>
            <string>libcrystal_clear_core.a</string>
            <key>HeadersPath</key>
            <string>.</string>
            <key>SupportedArchitectures</key>
            <array>
                $ARCH_ARRAY
            </array>
            <key>SupportedPlatform</key>
            <string>macos</string>
        </dict>
    </array>
    <key>CFBundlePackageType</key>
    <string>XFWK</string>
    <key>XCFrameworkFormatVersion</key>
    <string>1.0</string>
</dict>
</plist>
EOF

echo -e "${GREEN}XCFramework created at: $XCFRAMEWORK_DIR${NC}"

# Print summary
echo ""
echo -e "${GREEN}=== Build Complete ===${NC}"
echo ""
echo "Generated files:"
echo "  - Universal static library: $UNIVERSAL_DIR/libcrystal_clear_core.a"
echo "  - Swift bindings: $GENERATED_DIR/"
echo "  - XCFramework: $XCFRAMEWORK_DIR/"
echo ""
echo "Next steps:"
echo "  1. Open crystalclear/crystalclear.xcodeproj in Xcode"
echo "  2. Drag CrystalClearCore.xcframework into the project"
echo "  3. Add the Generated/*.swift files to the project"
echo "  4. Build and run!"
