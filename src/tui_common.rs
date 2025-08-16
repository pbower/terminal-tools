//! Common TUI utilities and shared functionality across all terminal tools.
//!
//! This module provides the foundational building blocks for all TUI-based tools in the
//! terminal-tools collection. It handles terminal setup/teardown, consistent styling,
//! keyboard navigation patterns, and common UI components.
//!
//! ## Core Features
//!
//! - **Terminal Management**: Safe setup and restoration of terminal state
//! - **Color Scheme**: Consistent color palette across all tools
//! - **Navigation**: Vim-style keyboard shortcuts with Ctrl-F/Ctrl-B paging
//! - **Error Handling**: Robust terminal state management with cleanup guarantees
//!
//! ## Usage
//!
//! Most tools will use this module to set up their TUI environment:
//!
//! ```rust
//! use crate::tui_common::{setup_terminal, restore_terminal};
//!
//! fn run_tool() -> std::io::Result<()> {
//!     let mut terminal = setup_terminal()?;
//!     
//!     // ... tool implementation ...
//!     
//!     restore_terminal(&mut terminal)?;
//!     Ok(())
//! }
//! ```
//!
//! ## Navigation Patterns
//!
//! All tools implement consistent keyboard navigation:
//! - `↑/↓` or `j/k` for line-by-line movement
//! - `Ctrl-F/Ctrl-B` for page-by-page movement
//! - `Enter` to select or execute items
//! - `Esc` or `q` to quit
//!
//! The [`handle_page_navigation`] function provides standardized page navigation logic
//! that all tools can use to maintain consistency.
//!
//! ## Color Scheme
//!
//! The [`colors`] module defines a cohesive color palette that ensures visual
//! consistency across all tools while maintaining good readability in various
//! terminal environments.

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::io;

/// Set up terminal for TUI mode with proper state management.
///
/// This function prepares the terminal for TUI applications by:
/// - Enabling raw mode for direct key capture
/// - Switching to alternate screen buffer
/// - Enabling mouse capture for scroll events
/// - Creating a ratatui Terminal instance
///
/// # Returns
///
/// Returns a configured `Terminal` instance ready for TUI rendering.
///
/// # Errors
///
/// Returns an `io::Error` if terminal setup fails, typically due to:
/// - Terminal not supporting required features
/// - Permission issues with terminal control
/// - Already being in raw mode from another application
///
/// # Examples
///
/// ```rust,no_run
/// use crate::tui_common::{setup_terminal, restore_terminal};
///
/// let mut terminal = setup_terminal()?;
/// // ... use terminal for TUI ...
/// restore_terminal(&mut terminal)?;
/// # Ok::<(), std::io::Error>(())
/// ```
pub fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    let mut stdout = std::io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore terminal to normal mode and clean up TUI state.
///
/// This function safely restores the terminal to its original state by:
/// - Disabling raw mode to restore normal input handling
/// - Exiting alternate screen buffer to show original content
/// - Disabling mouse capture
/// - Showing the cursor again
///
/// This function should **always** be called before a TUI application exits,
/// preferably in a `Drop` implementation or cleanup handler to ensure the
/// terminal is restored even if the application panics.
///
/// # Arguments
///
/// * `terminal` - Mutable reference to the Terminal to restore
///
/// # Errors
///
/// Returns an `io::Error` if terminal restoration fails. Even if this function
/// returns an error, the terminal state may have been partially restored.
///
/// # Examples
///
/// ```rust,no_run
/// use crate::tui_common::{setup_terminal, restore_terminal};
///
/// let mut terminal = setup_terminal()?;
/// // ... TUI application logic ...
/// restore_terminal(&mut terminal)?;
/// # Ok::<(), std::io::Error>(())
/// ```
pub fn restore_terminal<B: Backend + std::io::Write>(terminal: &mut Terminal<B>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

/// Consistent color scheme used across all terminal tools.
///
/// This module defines a cohesive color palette that ensures visual consistency
/// across all tools while maintaining good readability in various terminal environments.
/// The colors are chosen to work well with both light and dark terminal themes.
///
/// # Design Principles
///
/// - **Primary**: Cyan for headers, selected items, and important UI elements
/// - **Secondary**: Yellow for highlights, warnings, and secondary actions
/// - **Success**: Green for positive feedback and successful operations
/// - **Danger**: Red for errors, warnings, and destructive actions
/// - **Warning**: Magenta for cautions and intermediate states
/// - **Muted**: Dark gray for disabled items and secondary text
/// - **Background/Text**: Standard black/white for optimal contrast
///
/// # Usage
///
/// ```rust
/// use ratatui::style::{Style, Stylize};
/// use crate::tui_common::colors;
///
/// let header_style = Style::default().fg(colors::PRIMARY);
/// let selected_style = Style::default().bg(colors::PRIMARY).fg(colors::BACKGROUND);
/// ```
pub mod colors {
    use ratatui::style::Color;
    
    pub const PRIMARY: Color = Color::Cyan;
    pub const SECONDARY: Color = Color::Yellow;
    #[allow(dead_code)]
    pub const SUCCESS: Color = Color::Green;
    #[allow(dead_code)]
    pub const DANGER: Color = Color::Red;
    #[allow(dead_code)]
    pub const WARNING: Color = Color::Magenta;
    #[allow(dead_code)]
    pub const MUTED: Color = Color::DarkGray;
    pub const BACKGROUND: Color = Color::Black;
    pub const TEXT: Color = Color::White;
}

/// Common key bindings help text
#[allow(dead_code)]
pub fn common_help_text() -> Vec<&'static str> {
    vec![
        "↑/↓ Navigate",
        "Enter Select", 
        "Ctrl-F/B Page",
        "Esc/q Quit",
    ]
}

/// Handle standardized page navigation with Ctrl-F/Ctrl-B shortcuts.
///
/// This function implements consistent page-by-page navigation that all tools
/// use to maintain a uniform user experience. It handles the Vim-style Ctrl-F
/// (page forward) and Ctrl-B (page backward) keyboard shortcuts.
///
/// # Arguments
///
/// * `key_code` - The key that was pressed
/// * `modifiers` - Key modifiers (used to detect Ctrl key)
/// * `current_selection` - Current selected item index (if any)
/// * `total_items` - Total number of items in the list
/// * `page_size` - Number of items to move per page
///
/// # Returns
///
/// Returns the new selection index if navigation occurred, otherwise returns
/// the original `current_selection`.
///
/// # Behavior
///
/// - **Ctrl-F**: Move forward by `page_size` items, clamped to the last item
/// - **Ctrl-B**: Move backward by `page_size` items, clamped to the first item
/// - **Other keys**: No change to selection
/// - **Empty lists**: Returns `None` for safety
///
/// # Examples
///
/// ```rust
/// use crossterm::event::{KeyCode, KeyModifiers};
/// use crate::tui_common::handle_page_navigation;
///
/// let current = Some(5);
/// let total = 100;
/// let page_size = 10;
///
/// // Ctrl-F: move to item 15
/// let new_selection = handle_page_navigation(
///     KeyCode::Char('f'),
///     KeyModifiers::CONTROL,
///     current,
///     total,
///     page_size
/// );
/// assert_eq!(new_selection, Some(15));
/// ```
pub fn handle_page_navigation(
    key_code: crossterm::event::KeyCode,
    modifiers: crossterm::event::KeyModifiers,
    current_selection: Option<usize>,
    total_items: usize,
    page_size: usize,
) -> Option<usize> {
    use crossterm::event::{KeyCode, KeyModifiers};
    
    match key_code {
        KeyCode::Char('f') if modifiers.contains(KeyModifiers::CONTROL) => {
            // Page down
            if let Some(selected) = current_selection {
                Some(std::cmp::min(selected + page_size, total_items.saturating_sub(1)))
            } else if total_items > 0 {
                Some(0)
            } else {
                None
            }
        }
        KeyCode::Char('b') if modifiers.contains(KeyModifiers::CONTROL) => {
            // Page up
            if let Some(selected) = current_selection {
                Some(selected.saturating_sub(page_size))
            } else {
                None
            }
        }
        _ => current_selection,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn test_handle_page_navigation_ctrl_f() {
        // Test Ctrl-F (page forward)
        let result = handle_page_navigation(
            KeyCode::Char('f'),
            KeyModifiers::CONTROL,
            Some(5),
            100,
            10,
        );
        assert_eq!(result, Some(15)); // 5 + 10 = 15
    }

    #[test]
    fn test_handle_page_navigation_ctrl_f_near_end() {
        // Test Ctrl-F near the end of list
        let result = handle_page_navigation(
            KeyCode::Char('f'),
            KeyModifiers::CONTROL,
            Some(95),
            100,
            10,
        );
        assert_eq!(result, Some(99)); // Clamped to last item (99)
    }

    #[test]
    fn test_handle_page_navigation_ctrl_b() {
        // Test Ctrl-B (page backward)
        let result = handle_page_navigation(
            KeyCode::Char('b'),
            KeyModifiers::CONTROL,
            Some(15),
            100,
            10,
        );
        assert_eq!(result, Some(5)); // 15 - 10 = 5
    }

    #[test]
    fn test_handle_page_navigation_ctrl_b_near_start() {
        // Test Ctrl-B near the start of list
        let result = handle_page_navigation(
            KeyCode::Char('b'),
            KeyModifiers::CONTROL,
            Some(5),
            100,
            10,
        );
        assert_eq!(result, Some(0)); // Saturating sub to 0
    }

    #[test]
    fn test_handle_page_navigation_empty_list() {
        // Test with empty list
        let result = handle_page_navigation(
            KeyCode::Char('f'),
            KeyModifiers::CONTROL,
            None,
            0,
            10,
        );
        assert_eq!(result, None);
    }

    #[test]
    fn test_handle_page_navigation_no_selection() {
        // Test Ctrl-F with no current selection but non-empty list
        let result = handle_page_navigation(
            KeyCode::Char('f'),
            KeyModifiers::CONTROL,
            None,
            100,
            10,
        );
        assert_eq!(result, Some(0)); // Should start at beginning

        // Test Ctrl-B with no current selection
        let result = handle_page_navigation(
            KeyCode::Char('b'),
            KeyModifiers::CONTROL,
            None,
            100,
            10,
        );
        assert_eq!(result, None); // Should remain None
    }

    #[test]
    fn test_handle_page_navigation_other_keys() {
        // Test that other keys don't change selection
        let result = handle_page_navigation(
            KeyCode::Char('j'),
            KeyModifiers::NONE,
            Some(5),
            100,
            10,
        );
        assert_eq!(result, Some(5)); // No change

        let result = handle_page_navigation(
            KeyCode::Enter,
            KeyModifiers::NONE,
            Some(5),
            100,
            10,
        );
        assert_eq!(result, Some(5)); // No change
    }

    #[test]
    fn test_handle_page_navigation_f_without_ctrl() {
        // Test 'f' without Ctrl modifier
        let result = handle_page_navigation(
            KeyCode::Char('f'),
            KeyModifiers::NONE,
            Some(5),
            100,
            10,
        );
        assert_eq!(result, Some(5)); // No change
    }

    #[test]
    fn test_common_help_text() {
        let help = common_help_text();
        assert!(!help.is_empty());
        assert!(help.iter().any(|&s| s.contains("Navigate")));
        assert!(help.iter().any(|&s| s.contains("Select")));
        assert!(help.iter().any(|&s| s.contains("Page")));
        assert!(help.iter().any(|&s| s.contains("Quit")));
    }
}