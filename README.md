# Crystal Clear

A fast, safe system cleaner. Works on macOS, Linux, and Windows.

## Why

I'm not paying $40/year for CleanMyMac. Neither should you. Especially if you're an engineer.

## Install

```bash
cargo install --path .
```

Or download from [Releases](https://github.com/ojowwalker77/crystal-clear/releases).

## Usage

```bash
crystal
```

Navigate with keyboard, select items with Space, clean with `c`.

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate up/down |
| `h` / `l` | Switch panels |
| `Space` | Select/deselect |
| `a` | Select all |
| `c` | Clean selected |
| `q` | Quit |

## What It Cleans

- **System Junk** - Caches, logs, temporary files
- **Large Files** - Find forgotten big files
- **Duplicates** - Identical files wasting space
- **Old Downloads** - DMGs, ZIPs, installers

## Safety

- Files go to Trash, never permanently deleted
- System directories are protected
- Your documents and data are never touched

## Building

```bash
cargo build --release
```

---

## For Normies (macOS GUI)

Don't like terminals? No problem. There's a native macOS app with a pretty interface.

**[Download Crystal Clear.app](https://github.com/ojowwalker77/crystal-clear/releases)** (macOS 14+)

Just download, open, click Scan, done.

<details>
<summary>Build from source</summary>

```bash
make xcframework
open crystalclear/crystalclear.xcodeproj
```
</details>

## License

MIT
