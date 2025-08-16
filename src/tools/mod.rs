//! Tool implementations for terminal utilities.
//!
//! This module contains the core implementations of all terminal tools available in the `tt` command.
//! Each tool is implemented as a separate module with a TUI interface built using ratatui.
//!
//! ## Available Tools
//!
//! - [`find`] - Fuzzy file finder with live preview
//! - [`search`] - Content search with ripgrep integration  
//! - [`kill`] - Interactive process manager
//! - [`git`] - Git repository browser and operations
//! - [`explore`] - File/directory explorer
//! - [`history`] - Command history browser
//! - [`mod@env`] - Environment variable viewer
//! - [`man`] - Manual page browser
//! - [`recent`] - Recent files tracker
//!
//! ## Design Patterns
//!
//! All tools follow consistent design patterns:
//!
//! - **TUI Structure**: Each tool has a main struct containing state and a `run()` method
//! - **Input Handling**: Consistent keyboard shortcuts across all tools
//! - **Error Handling**: Graceful degradation with informative error messages
//! - **Performance**: Optimized for large datasets with pagination and limiting
//!
//! ## Navigation
//!
//! All tools support the same navigation keys:
//! - `↑/↓` or `j/k` for line-by-line navigation
//! - `Ctrl-F/Ctrl-B` for page-by-page navigation
//! - `Enter` to select/execute items
//! - `Esc` or `q` to quit
//!
//! ## Integration
//!
//! Tools integrate with external commands where beneficial:
//! - `ripgrep` for fast text search (with graceful fallback to `grep`)
//! - `git` commands for repository operations (with timeouts)
//! - `zoxide` for directory frequency tracking (with fallback)
//! - System commands for process management and file operations

pub mod find;
pub mod kill;
pub mod git;
pub mod history;
pub mod explore;
pub mod env;
pub mod recent;
pub mod man;
pub mod search;