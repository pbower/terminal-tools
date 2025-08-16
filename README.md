# Terminal Tools (tt)

Power-dev terminal utils with Text User Interfaces (TUI) built with Rust and ratatui.

[![Crates.io](https://img.shields.io/crates/v/terminal_tools.svg)](https://crates.io/crates/terminal_tools)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## ✨ Features

- **🔍 Smart File Finding** - Fuzzy search with live preview
- **⚡ Lightning Fast Search** - Ripgrep integration with TUI interface
- **🖼️ Native Image Preview** - ASCII art generation in terminal
- **📊 Process Management** - Interactive process killer
- **🌳 Git Integration** - Browse history, diffs, and branches
- **📁 File Explorer** - Navigate directories with preview
- **📚 Command History** - Browse and execute past commands
- **🌍 Environment Browser** - Search and view environment variables
- **📖 Man Page Browser** - Interactive manual page viewer
- **📂 Recent Files** - Quick access to recently used files

All tools have:
- ⌨️ **Vim-style navigation** (Ctrl-F/Ctrl-B for paging)
- 🎨 **Good-looking interfaces** with syntax highlighting
- 🚀 **High performance** with optimised rendering
- 🛡️ **Robust error handling** with graceful degradation

## 🚀 Installation

### From crates.io (Recommended)

```bash
cargo install terminal_tools
```

### From source

```bash
git clone https://github.com/pbower/terminal-tools.git
cd terminal-tools
cargo install --path .
```

### System Requirements

- Rust 1.70+ (for installation)
- Git (for git tools)
- ripgrep (optional, for enhanced search)

## 📖 Usage

All tools are accessed through the `tt` command:

```bash
tt <command> [options]
```

### 🔍 File Finding

Find files with fuzzy search and live preview:

```bash
# Find all files
tt find

# Find files with specific extensions
tt find --extensions "rs,toml,md"

# Find files with initial search term
tt find --search "config"
```

**Features:**
- Fuzzy filename matching
- Live file content preview
- Image preview with ASCII art
- Fast directory traversal (skips .git, node_modules, target)

### ⚡ Content Search

Search within files using ripgrep with TUI interface:

```bash
# Interactive search (enter pattern in TUI)
tt search

# Direct search with pattern
tt search "fn main"

# Search specific file types
tt search "TODO" --file-type rust

# Case insensitive search
tt search "error" --ignore-case
```

**Features:**
- Live search as you type (2+ characters)
- Syntax highlighting in results
- Context lines around matches
- Jump to files at specific line numbers

### 📊 Process Management

Interactive process viewer and killer:

```bash
# Show all processes
tt kill

# Filter processes
tt kill --filter "node"
```

**Features:**
- Real-time process list
- Memory and CPU usage display
- Safe process termination
- Search and filter capabilities

### 🌳 Git Integration

Browse git repositories:

```bash
# Browse commit history
tt git log

# View git diff (browsable)
tt git diff

# Switch branches
tt git branch
```

**Features:**
- Commit history with diffs
- Limited diff preview (first 100 lines) to prevent freezing
- Branch switching interface
- Syntax highlighted diffs
- Command timeouts prevent hanging

### 📁 File Explorer

Navigate directories with preview:

```bash
# Explore current directory
tt explore

# Start from specific directory
tt explore /path/to/directory
```

**Features:**
- Two-panel interface (files + preview)
- Image preview support
- File content preview (first 50 lines)
- Directory statistics
- Quick navigation (arrows, Enter, Esc)

### 📚 Command History

Browse and execute command history:

```bash
# Browse shell history
tt history

# Limit number of entries
tt history --limit 50
```

**Features:**
- Search through command history
- Execute commands directly
- Command help integration
- Timestamp support

### 🌍 Environment Variables

Browse environment variables:

```bash
# Show all environment variables
tt env

# Filter variables
tt env --filter "PATH"
```

**Features:**
- Search and filter variables
- Value preview for long variables
- Alphabetical sorting

### 📂 Recent Files

Quick access to recently modified files:

```bash
# Show recent files
tt recent

# Show more files
tt recent --limit 25
```

**Features:**
- Finds files modified in last 7 days
- Sorted by modification time
- File preview support
- Quick file opening

### 📖 Man Pages

Interactive manual page browser:

```bash
# Browse available man pages
tt man

# Search for specific topic
tt man --search "git"
```

**Features:**
- Searchable man page list
- Live preview of man content
- Quick access to common commands

## ⌨️ Keyboard Shortcuts

All tools support consistent navigation:

| Key | Action |
|-----|--------|
| `↑/↓` or `j/k` | Navigate up/down |
| `Ctrl-F` | Page down |
| `Ctrl-B` | Page up |
| `Enter` | Select/Open |
| `Esc` or `q` | Quit |
| `Ctrl-C` | Force quit |

Tool-specific shortcuts:
- **Search tools**: Type to filter
- **File tools**: `Backspace` to delete search
- **Git tools**: `g/G` for top/bottom

## 🛠️ Configuration

### Shell Integration

For the best experience, you may want to create aliases:

```bash
# Add to ~/.bashrc or ~/.zshrc
alias f='tt find'
alias s='tt search'
alias k='tt kill'
alias g='tt git'
alias e='tt explore'
```

### Performance Tips

1. **Large repositories**: Git tools automatically limit output to prevent freezing
2. **File search**: Use `--extensions` to narrow search scope
3. **Content search**: Use specific patterns to reduce results

## 🖼️ Image Support

Terminal Tools includes native image preview support:

- **Supported formats**: JPG, PNG, GIF, BMP
- **ASCII art generation**: Images converted to text art
- **Zero dependencies**: No external image viewers required
- **Safe processing**: Large images handled gracefully

## 🚀 Performance

- **File finding**: Efficiently skips common build directories
- **Content search**: Powered by ripgrep for maximum speed
- **Git operations**: Timeouts prevent hanging on large repos
- **Memory efficient**: Streams large files instead of loading entirely

## 🐛 Troubleshooting

### Issue Resolution

These are uncommon but just in case.

**Git commands hang**:
- Fixed with timeouts and limited output
- Very large repos may still be slow

**TUI doesn't work**:
- Ensure terminal supports ANSI colors
- Some terminals may have compatibility issues
- Tools gracefully fallback to simple output

**Image preview fails**:
- Only common formats supported
- Large images (>50K pixels) are rejected
- Corrupted images handled gracefully

**Search is slow**:
- Install `ripgrep` for best performance
- Use `--file-type` to limit scope
- Exclude large directories with patterns

### Getting Help

```bash
# General help
tt --help

# Command-specific help
tt find --help
tt search --help
# ... etc
```

## 🤝 Contributing

Contributions are welcome! This project is built with:

- **Language**: Rust 2021 edition
- **TUI Framework**: ratatui + crossterm
- **Search**: ripgrep integration
- **Image Processing**: image + viuer crates

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
