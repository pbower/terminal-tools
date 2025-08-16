//! Command line interface definitions and argument parsing for terminal tools.
//!
//! This module defines the complete CLI structure using `clap` for the `tt` (terminal-tools)
//! command-line application. It provides a consistent interface for all available tools
//! with comprehensive help text and validation.
//!
//! ## Command Structure
//!
//! The CLI follows a subcommand pattern where each tool is a separate subcommand:
//!
//! ```bash
//! tt <SUBCOMMAND> [OPTIONS] [ARGS]
//! ```
//!
//! ## Available Commands
//!
//! - **find** - Fuzzy file finder with live preview and filtering
//! - **search** - Content search using ripgrep with regex support
//! - **kill** - Interactive process manager for killing processes
//! - **git** - Git operations (log, branch, status, diff) with TUI
//! - **explore** - File/directory explorer with image preview
//! - **history** - Command history browser and executor
//! - **env** - Environment variable viewer and manager
//! - **man** - Manual page browser with search
//! - **recent** - Recent files tracker with MRU ordering
//!
//! ## Usage Examples
//!
//! ```bash
//! # File operations
//! tt find /path/to/search --extensions "rs,toml" --search "main"
//! tt dir /home/user/projects
//!
//! # Content search
//! tt search "pattern" --path /src --file-type rust --ignore-case
//! tt search  # Start live search mode
//!
//! # Process management  
//! tt kill --filter "python"
//!
//! # Git operations
//! tt git log
//! tt git branch
//! tt git diff
//!
//! # System utilities
//! tt hist --limit 50
//! tt env --filter "PATH"
//! tt man --search "grep"
//! tt recent --limit 20
//! ```
//!
//! ## Design Principles
//!
//! - **Consistent Patterns**: Similar options across commands (--path, --limit, etc.)
//! - **Sensible Defaults**: Reasonable default values that work for most use cases
//! - **Optional Arguments**: Most arguments are optional to enable interactive workflows
//! - **Help Integration**: Comprehensive help text and examples for all commands

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Main CLI structure for the terminal-tools application.
///
/// This is the root command that dispatches to individual tool subcommands.
/// All tools are accessed through this unified interface using the `tt` binary.
#[derive(Parser)]
#[command(name = "terminal-tools")]
#[command(about = "A collection of powerful terminal utilities with beautiful TUI interfaces")]
#[command(version = "0.1.0")]
#[command(disable_help_subcommand = true)]
pub struct Cli {
    /// The specific tool/command to run
    #[command(subcommand)]
    pub command: Commands,
}

/// All available terminal tools as CLI subcommands.
///
/// Each variant represents a different tool with its own set of arguments
/// and functionality. The tools cover file operations, content search,
/// process management, git operations, and system utilities.
#[derive(Subcommand)]
pub enum Commands {
    /// Fuzzy file finder with live preview
    Find {
        /// Starting directory to search
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
        
        /// File extensions to filter (comma-separated)
        #[arg(short, long)]
        extensions: Option<String>,
        
        /// Initial search term (optional for live search)
        #[arg(short, long)]
        search: Option<String>,
    },
    
    /// Process manager and killer with selection
    Kill {
        /// Filter processes by name
        #[arg(short, long)]
        filter: Option<String>,
    },
    
    /// Git operations and history browser
    Git {
        #[command(subcommand)]
        subcommand: GitCommands,
    },
    
    /// Command history browser and executor
    Hist {
        /// Number of recent commands to show
        #[arg(short, long, default_value = "100")]
        limit: usize,
    },
    
    /// Interactive file/directory explorer
    Dir {
        /// Starting directory
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
    },
    
    /// Environment variable viewer and manager
    Env {
        /// Filter environment variables by name
        #[arg(short, long)]
        filter: Option<String>,
    },
    
    /// Recent files browser with MRU tracking
    Recent {
        /// Number of recent files to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    
    
    /// Man page browser
    Man {
        /// Search term for man pages
        #[arg(short, long)]
        search: Option<String>,
    },
    
    /// Content search with ripgrep integration
    Search {
        /// Search pattern (regex supported, optional for live search)
        pattern: Option<String>,
        
        /// Directory to search in
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
        
        /// File type filter (e.g., "rust", "js", "py")
        #[arg(short = 't', long)]
        file_type: Option<String>,
        
        /// Case insensitive search
        #[arg(short, long)]
        ignore_case: bool,
    },
}

/// Git-specific subcommands for repository operations.
///
/// These commands provide TUI interfaces for common git operations,
/// making it easier to browse history, switch branches, and review
/// changes without leaving the terminal.
#[derive(Subcommand)]
pub enum GitCommands {
    /// Browse git log with diff preview
    Log,
    
    /// Switch branches interactively
    Branch,
    
    /// View git status with file selection
    Status,
    
    /// Show git diff with file selection
    Diff,
}