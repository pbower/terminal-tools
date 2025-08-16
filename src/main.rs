//! # Terminal Tools (tt)
//!
//! Power terminal utils with Text User Interfaces (TUI).
//!
//! Built with Rust and ratatui for max performance and cross-platform compatibility.
//!
//! ## Available Tools
//!
//! - **🔍 find** - Fuzzy file finder with live preview and ASCII image support
//! - **⚡ search** - Lightning-fast content search with ripgrep integration
//! - **📊 kill** - Interactive process manager and killer
//! - **🌳 git** - Git operations (log, diff, branch) with TUI interface
//! - **📁 dir** - File/directory explorer with preview pane
//! - **📚 hist** - Command history browser and executor
//! - **🌍 env** - Environment variable viewer and manager
//! - **📖 man** - Interactive manual page browser
//! - **📂 recent** - Recent files browser with MRU tracking
//!
//! ## Key Features
//!
//! - **Vim-style navigation** with Ctrl-F/Ctrl-B paging
//! - **Native image preview** using ASCII art generation
//! - **High performance** with optimized rendering and timeouts
//! - **Robust error handling** with graceful degradation
//! - **Zero external dependencies** for core functionality
//!
//! ## Quick Start
//!
//! ```bash
//! # Installation
//! cargo install terminal_tools
//!
//! # Essential commands
//! tt find                    # Find files with fuzzy search
//! tt search "pattern"        # Search content in files
//! tt kill                    # Manage processes interactively
//! tt git log                 # Browse git commit history
//! tt dir /path              # Explore directories
//! ```
//!
//! ## Navigation
//!
//! All tools support consistent keyboard shortcuts:
//! - `↑/↓` or `j/k` - Navigate up/down
//! - `Ctrl-F/B` - Page forward/backward
//! - `Enter` - Select/execute
//! - `Esc` or `q` - Quit
//! - `Ctrl-C` - Force quit
//!
//! For detailed usage instructions, see the [README](https://github.com/pbower/terminal-tools#readme).

use clap::Parser;
use std::io;

mod cli;
mod tools;
mod tui_common;
mod image_preview;

use cli::*;

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Find { path, extensions, search } => {
            tools::find::run(path, extensions, search)
        }
        Commands::Kill { filter } => {
            tools::kill::run(filter)
        }
        Commands::Git { subcommand } => {
            tools::git::run(subcommand)
        }
        Commands::Hist { limit } => {
            tools::history::run(limit)
        }
        Commands::Dir { path } => {
            tools::explore::run(path)
        }
        Commands::Env { filter: _ } => {
            tools::env::run()
        }
        Commands::Recent { limit } => {
            tools::recent::run(limit)
        }
        Commands::Man { search } => {
            tools::man::run(search)
        }
        Commands::Search { pattern, path, file_type, ignore_case } => {
            tools::search::run(pattern, path, file_type, ignore_case)
        }
    }
}
