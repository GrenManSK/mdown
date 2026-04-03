# mdown

> A manga downloader for [MangaDex](https://mangadex.org/)

[![License: GPL-3.0](https://img.shields.io/badge/License-GPL%203.0-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)

**mdown** lets you download manga chapters from MangaDex in multiple formats. It supports batch downloading, a web reader, a desktop GUI, and a LAN server for sharing your library.

---

## Table of Contents

- [Quick Start](#quick-start)
- [Installation](#installation)
- [Usage](#usage)
  - [Basic Examples](#basic-examples)
  - [Command Reference](#command-reference)
  - [Subcommands](#subcommands)
- [Features](#features)
- [Building from Source](#building-from-source)
- [Credits & Legal](#credits--legal)
- [Troubleshooting](#troubleshooting)

---

## Quick Start

**First time?** Follow these steps:

1. [Install Rust](https://www.rust-lang.org/tools/install) (if building from source)
2. Build: `cargo build -r`
3. Run: `mdown --url https://mangadex.org/title/UUID`

That's it! Your manga will download as `.cbz` files.

---

## Installation

### Option 1: Build from Source (Recommended)

```bash
# 1. Install Rust from https://www.rust-lang.org/tools/install

# 2. Clone and build
git clone https://github.com/GrenManSK/mdown.git
cd mdown
cargo build -r

# 3. Run
./target/release/mdown --help
```

### Option 2: Use Cargo Run

```bash
# Build and run in one step
cargo run -r -- --url https://mangadex.org/title/UUID

# Pass arguments after --
cargo run -r -- --url UUID --lang en --saver
```

> **Note:** First-time configuration uses yt-dlp for downloading some resources.

---

## Usage

> **Note:** Some flags accept optional arguments. In `--help` output:
> - `<ARG>` — argument is **required**
> - `[<ARG>]` — argument is **optional**, uses default value if omitted

### Basic Examples

```bash
# Download a manga (paste the MangaDex URL)
mdown --url https://mangadex.org/title/a1c7c817-4e59-4a91-8e73-6a6d7b8e9f0a

# Download only English chapters
mdown --url UUID --lang en

# Download a specific chapter
mdown --url UUID --chapter 5

# Download with lower quality (faster)
mdown --url UUID --saver

# Search by title
mdown --search "One Piece"

# Use the web interface (opens browser at localhost:8080)
mdown --web

# Use the desktop GUI
mdown --gui
```

### Command Reference

#### Main Options

| Flag | Description | Example |
|------|-------------|---------|
| `--url <UUID>` | MangaDex URL or UUID (required) | `--url https://mangadex.org/title/UUID` |
| `--lang <CODE>` | Language code (`*` for all) | `--lang en` |
| `--title <NAME>` | Custom manga title | `--title "My Manga"` |
| `--folder <PATH>` | Output folder (`**name**` = manga name) | `--folder "My Downloads"` |
| `--volume <NUM>` | Download specific volume | `--volume 1` |
| `--chapter <NUM>` | Download specific chapter | `--chapter 5` |
| `--saver` | Use data-saver images (smaller files) | |
| `--force` | Re-download even if file exists | |
| `--offset <NUM>` | Skip first N chapters | `--offset 10` |
| `--quiet` | Suppress terminal output | |
| `--max-consecutive <N>` | Parallel image downloads (max 50, default 40) | `--max-consecutive 20` |
| `--stat` | Generate download statistics file | |
| `--log` | Enable logging to `log.json` | |
| `--search <TITLE>` | Search manga by title | `--search "Naruto"` |

#### Interface Modes

| Flag | Description |
|------|-------------|
| `--web` | Web interface on port 8080 |
| `--gui` | Desktop GUI (requires `gui` feature) |
| `--server` | LAN server for sharing downloads |
| `--music` | Background music during downloads (requires `music` feature) |

#### Utility Flags

| Flag | Description |
|------|-------------|
| `--cwd <PATH>` | Change working directory |
| `--encode <URL>` | Encode URL for processing |
| `--unsorted` | Don't sort chapters |
| `--database-offset <N>` | Offset for database queries |
| `--tutorial` | Run interactive tutorial |
| `--skip-tutorial` | Skip first-run tutorial |

### Subcommands

#### Application Management

```bash
mdown app --force-setup    # Re-run initial setup
mdown app --force-delete   # Remove lock file (if crashed)
mdown app --delete         # Delete manga database
mdown app --reset          # Factory reset
mdown app --backup         # Manual backup
mdown app --update         # Update mdown to latest version
```

> **Note:** The lowest time unit in backup filenames is a day. Forced backups will overwrite existing backups with the same day.

#### Database Operations

```bash
mdown database --check          # Check for manga updates
mdown database --update         # Download available updates
mdown database --show           # List downloaded manga
mdown database --show-all       # List all chapters
mdown database --show-log       # View download logs
mdown database --show-settings  # View saved settings
mdown database --backup-choose  # Choose backup to restore
```

#### Settings

```bash
mdown settings --folder <NAME>   # Set default download folder (omitting removes it)
mdown settings --stat <0|1>      # Auto-enable statistics (omitting removes it)
mdown settings --backup <0|1>    # Enable/disable auto-backup (omitting removes it)
mdown settings --music <NUM>     # Default music track (omitting removes it)
mdown settings --clear           # Clear all settings
```

---

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| CLI | ✓ | Command-line interface |
| Web | ✓ | Browser-based reader on localhost:8080 |
| Server | ✓ | LAN server for sharing your library |
| GUI | ✗ | Desktop app with egui |
| Music | ✗ | Background music during downloads |

### Download Behavior

- Will download cover image and description even if no new chapters were downloaded
- If no eligible chapters are found (e.g., manga only has Japanese chapters but you requested English), the original file will be deleted
- Manga titles are automatically shortened to 70 characters maximum
- All temporary files are stored in `.cache/` folder, which is deleted if empty after completion

### GUI Notes

- Setting a default music track in GUI will not auto-start playback due to Mutex limitations

### Feature Flags

```bash
# Build with specific features
cargo build -r -F gui           # GUI only
cargo build -r -F music         # Music only
cargo build -r -F full          # All features

# Combine features
cargo build -r -F gui -F music
```

**Music Feature**: Download the music pack from [Releases](https://github.com/GrenManSK/mdown/releases/tag/resources) and extract to `resources/music/`.

---

## Building from Source

### Requirements

- [Rust](https://www.rust-lang.org/tools/install) 1.75 or later
- Git

### Steps

```bash
git clone https://github.com/GrenManSK/mdown.git
cd mdown

# Standard build
cargo build -r

# With all features
cargo build -r -F full

# The executable is at: target/release/mdown
```

---

## Credits & Legal

### MangaDex

This application uses the [MangaDex API](https://api.mangadex.org/) to fetch manga data. All manga content is hosted by MangaDex.

- Website: [mangadex.org](https://mangadex.org/)
- API Docs: [api.mangadex.org/docs](https://api.mangadex.org/docs/)

### Scanlation Groups

Manga chapters are translated and provided by **scanlation groups**. We credit and respect their work:

- Credits for each chapter are stored in the downloaded files
- **If you are a scanlation group and want your content removed**: Open an [issue](https://github.com/GrenManSK/mdown/issues) or contact us directly — we will honor all legitimate content removal requests
- We encourage supporting official releases when available

### Terms of Use

- This software is provided **as-is** under the GPL-3.0 license
- **No ads or paid services**: This application will never run ads, require payment, or monetize downloaded content
- You are responsible for complying with MangaDex's [Terms of Service](https://mangadex.org/terms)
- Respect copyright laws in your jurisdiction
- Support official manga releases when possible

---

## Troubleshooting

### Common Issues

**"Lock file is present" error**
```bash
mdown app --force-delete
```
This removes a stuck lock file. Only use if you're sure no other instance is running.

**Missing pages in downloads**
- Lower `--max-consecutive` (try 20 or lower)
- Check your internet connection

**Slow downloads**
- Use `--saver` for smaller images
- Reduce `--max-consecutive` if on slow internet

**First-time setup**
The app runs a tutorial on first launch. Reset with:
```bash
mdown app --reset
```

### File Locations

| File | Purpose |
|------|---------|
| `dat.json` | Downloaded manga metadata |
| `resources.db` | Application database |
| `.cache/` | Temporary download files |
| `log.json` | Download logs (if `--log` enabled) |

### Language Codes

Use [ISO 639-1](https://en.wikipedia.org/wiki/List_of_ISO_639_language_codes) codes:
- `en` — English
- `es` — Spanish
- `ja` — Japanese
- `*` — All languages

See [MangaDex API docs](https://api.mangadex.org/docs/3-enumerations/#language-codes--localization) for exceptions.

---

## License

This project is licensed under the GNU General Public License v3.0 — see [LICENSE](LICENSE) for details.
