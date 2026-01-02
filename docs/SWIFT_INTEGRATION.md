# Crystal Clear Swift Integration

This document explains how to set up the Swift macOS app to use the Rust core library via UniFFI.

## Prerequisites

- Xcode 15+
- Rust toolchain
- macOS 13+ target

## Quick Build

```bash
# Build everything (XCFramework + Swift app)
make app

# Or just build the XCFramework and bindings
make swift-bindings
```

## Manual Xcode Setup

If you need to set up the Xcode project manually:

### 1. Generate the XCFramework

```bash
./scripts/build-xcframework.sh
```

This creates:
- `crystalclear/CrystalClearCore.xcframework/` - Static library
- `crystalclear/crystalclear/Generated/crystal_clear_core.swift` - Swift bindings
- `crystalclear/crystalclear/Generated/crystal_clear_coreFFI.h` - C header

### 2. Add XCFramework to Xcode

1. Open `crystalclear/crystalclear.xcodeproj`
2. Select the project in the navigator
3. Select the `crystalclear` target
4. Go to "General" tab
5. Under "Frameworks, Libraries, and Embedded Content":
   - Click "+"
   - Click "Add Other..." → "Add Files..."
   - Navigate to `crystalclear/CrystalClearCore.xcframework`
   - Select "Do Not Embed" (it's a static library)

### 3. Configure Build Settings

In the target's "Build Settings":

1. **Header Search Paths**:
   Add: `$(PROJECT_DIR)/CrystalClearCore.xcframework/macos-arm64`

2. **Library Search Paths**:
   Add: `$(PROJECT_DIR)/CrystalClearCore.xcframework/macos-arm64`

3. **Other Linker Flags**:
   Add: `-lcrystal_clear_core`

4. **Swift Compiler - Search Paths > Import Paths**:
   Add: `$(PROJECT_DIR)/CrystalClearCore.xcframework/macos-arm64`

### 4. Add Swift Files to Project

Ensure these files are in the target:
- `crystalclear/crystalclear/Generated/crystal_clear_core.swift`
- All files in `crystalclear/crystalclear/Core/`
- All files in `crystalclear/crystalclear/Views/`

### 5. Create Bridging Header (if needed)

Create `crystalclear-Bridging-Header.h`:

```objc
#import "crystal_clear_coreFFI.h"
```

Set in Build Settings:
- "Objective-C Bridging Header": `$(PROJECT_DIR)/crystalclear/crystalclear-Bridging-Header.h`

## Project Structure

```
crystalclear/
├── crystalclear.xcodeproj
├── CrystalClearCore.xcframework/    # Generated XCFramework
│   ├── Info.plist
│   └── macos-arm64/
│       ├── libcrystal_clear_core.a
│       ├── crystal_clear_coreFFI.h
│       └── module.modulemap
└── crystalclear/
    ├── crystalclearApp.swift        # App entry point
    ├── ContentView.swift            # Main layout
    ├── Generated/
    │   └── crystal_clear_core.swift # UniFFI bindings
    ├── Core/
    │   ├── CrystalClearService.swift
    │   └── ViewModels/
    │       └── MainViewModel.swift
    └── Views/
        ├── Sidebar/
        ├── CleanableItems/
        ├── Apps/
        ├── DiskHealth/
        └── Shared/
```

## Available Swift Functions

The UniFFI bindings expose these functions:

```swift
// Scanning
func scanAllCategories(options: FfiScanOptions) -> [FfiScanResult]
func scanCategory(categoryId: String, options: FfiScanOptions) -> FfiScanResult
func scanApplications() -> [FfiInstalledApp]

// Cleaning
func cleanItems(paths: [String], options: FfiCleanOptions) -> FfiCleanResult
func uninstallApp(appPath: String, includeLeftovers: Bool) -> FfiCleanResult

// Utilities
func getAvailableCleaners() -> [FfiCleanerInfo]
func getRootDiskInfo() -> FfiDiskInfo?
func getAllDiskInfo() -> [FfiDiskInfo]
func isProtectedPath(path: String) -> Bool
func formatSize(bytes: UInt64) -> String
```

## Troubleshooting

### "Library not found" error

Ensure the XCFramework is properly linked:
1. Check Library Search Paths includes the XCFramework path
2. Verify Other Linker Flags includes `-lcrystal_clear_core`

### "Symbol not found" error

Make sure:
1. The bridging header is correctly configured
2. The generated Swift file is added to the target
3. You rebuilt after running `./scripts/build-xcframework.sh`

### FFI module not found

The UniFFI scaffolding must be set up in the Rust crate root (`src/lib.rs`):
```rust
uniffi::setup_scaffolding!();
```

## Rebuilding After Rust Changes

After modifying Rust code:

```bash
# Rebuild everything
make swift-bindings

# Then rebuild in Xcode (⌘B)
```
