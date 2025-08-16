//! Command history browser and executor.

use crate::tui_common::{self, colors};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::{
    env,
    fs,
    io,
    path::PathBuf,
    process::Command,
    time::Duration,
};

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub command: String,
    #[allow(dead_code)]
    pub timestamp: Option<String>,
}

pub struct HistoryBrowser {
    entries: Vec<HistoryEntry>,
    list_state: ListState,
    should_quit: bool,
    status_message: String,
    preview_content: String,
    limit: usize,
}

impl HistoryBrowser {
    /// Create a new history browser
    pub fn new(limit: usize) -> io::Result<Self> {
        let mut browser = HistoryBrowser {
            entries: Vec::new(),
            list_state: ListState::default(),
            should_quit: false,
            status_message: "Loading command history...".to_string(),
            preview_content: String::new(),
            limit,
        };
        
        browser.load_history()?;
        
        Ok(browser)
    }
    
    /// Load command history
    fn load_history(&mut self) -> io::Result<()> {
        // Try to load from bash history file
        if let Ok(home) = env::var("HOME") {
            let history_file = PathBuf::from(home).join(".bash_history");
            if let Ok(content) = fs::read_to_string(history_file) {
                let lines: Vec<&str> = content.lines().collect();
                let start = if lines.len() > self.limit {
                    lines.len() - self.limit
                } else {
                    0
                };
                
                for line in lines[start..].iter().rev() {
                    if !line.trim().is_empty() {
                        self.entries.push(HistoryEntry {
                            command: line.to_string(),
                            timestamp: None,
                        });
                    }
                }
            } else {
                // Fallback to history command
                self.load_from_history_command()?;
            }
        } else {
            self.load_from_history_command()?;
        }
        
        // Remove duplicates while preserving order
        let mut seen = std::collections::HashSet::new();
        self.entries.retain(|entry| seen.insert(entry.command.clone()));
        
        if !self.entries.is_empty() {
            self.list_state.select(Some(0));
            self.update_preview();
        }
        
        self.status_message = format!("Loaded {} commands", self.entries.len());
        Ok(())
    }
    
    /// Load from history command as fallback
    fn load_from_history_command(&mut self) -> io::Result<()> {
        let output = Command::new("history")
            .arg(format!("{}", self.limit))
            .output();
        
        if let Ok(output) = output {
            if output.status.success() {
                let history_output = String::from_utf8_lossy(&output.stdout);
                for line in history_output.lines().rev() {
                    if let Some(cmd_start) = line.find(' ') {
                        let command = line[cmd_start..].trim().to_string();
                        if !command.is_empty() {
                            self.entries.push(HistoryEntry {
                                command,
                                timestamp: None,
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }
    
    /// Update preview content
    fn update_preview(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if let Some(entry) = self.entries.get(selected) {
                // Show command details and man page if available
                let parts: Vec<&str> = entry.command.split_whitespace().collect();
                if let Some(command) = parts.first() {
                    self.preview_content = self.get_command_help(command);
                } else {
                    self.preview_content = "No command selected".to_string();
                }
            }
        }
    }
    
    /// Get help for a command
    fn get_command_help(&self, command: &str) -> String {
        // Try to get brief help from man or --help
        if let Ok(output) = Command::new("man")
            .args(&["-f", command])
            .output() {
            if output.status.success() {
                let help = String::from_utf8_lossy(&output.stdout);
                if !help.trim().is_empty() {
                    return format!("Manual page for '{}':\n\n{}", command, help);
                }
            }
        }
        
        // Try --help as fallback
        if let Ok(output) = Command::new(command)
            .arg("--help")
            .output() {
            if output.status.success() {
                let help = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<&str> = help.lines().take(20).collect();
                return format!("Help for '{}':\n\n{}", command, lines.join("\n"));
            }
        }
        
        format!("No help available for command: {}", command)
    }
    
    /// Execute selected command
    fn execute_command(&mut self) -> io::Result<()> {
        if let Some(selected) = self.list_state.selected() {
            if let Some(entry) = self.entries.get(selected) {
                // Print the command and exit - let the shell handle execution
                println!("{}", entry.command);
                self.should_quit = true;
            }
        }
        Ok(())
    }
    
    /// Handle keyboard input
    fn handle_input(&mut self) -> io::Result<()> {
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        self.should_quit = true;
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.should_quit = true;
                    }
                    KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        // Page down
                        if let Some(new_selection) = tui_common::handle_page_navigation(
                            key.code, key.modifiers, self.list_state.selected(), self.entries.len(), 10
                        ) {
                            self.list_state.select(Some(new_selection));
                            self.update_preview();
                        }
                    }
                    KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        // Page up
                        if let Some(new_selection) = tui_common::handle_page_navigation(
                            key.code, key.modifiers, self.list_state.selected(), self.entries.len(), 10
                        ) {
                            self.list_state.select(Some(new_selection));
                            self.update_preview();
                        }
                    }
                    KeyCode::Up => {
                        if let Some(selected) = self.list_state.selected() {
                            if selected > 0 {
                                self.list_state.select(Some(selected - 1));
                                self.update_preview();
                            }
                        }
                    }
                    KeyCode::Down => {
                        if let Some(selected) = self.list_state.selected() {
                            if selected + 1 < self.entries.len() {
                                self.list_state.select(Some(selected + 1));
                                self.update_preview();
                            }
                        } else if !self.entries.is_empty() {
                            self.list_state.select(Some(0));
                            self.update_preview();
                        }
                    }
                    KeyCode::Enter => {
                        self.execute_command()?;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
    
    /// Render the history browser
    fn render(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(f.area());
        
        self.render_history_list(f, chunks[0]);
        self.render_command_help(f, chunks[1]);
        self.render_status_bar(f);
    }
    
    /// Render history list
    fn render_history_list(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let line = Line::from(format!("{:3}: {}", 
                    self.entries.len() - i, 
                    entry.command
                ));
                ListItem::new(line)
            })
            .collect();
        
        let list = List::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(format!("Command History ({})", self.entries.len()))
                .border_style(Style::default().fg(colors::PRIMARY)))
            .highlight_style(Style::default()
                .bg(colors::PRIMARY)
                .fg(colors::BACKGROUND)
                .add_modifier(Modifier::BOLD))
            .highlight_symbol("► ");
        
        f.render_stateful_widget(list, area, &mut self.list_state);
    }
    
    /// Render command help
    fn render_command_help(&self, f: &mut Frame, area: Rect) {
        let title = if let Some(selected) = self.list_state.selected() {
            if let Some(entry) = self.entries.get(selected) {
                let parts: Vec<&str> = entry.command.split_whitespace().collect();
                if let Some(command) = parts.first() {
                    format!("Help: {}", command)
                } else {
                    "Help".to_string()
                }
            } else {
                "Help".to_string()
            }
        } else {
            "Help".to_string()
        };
        
        let paragraph = Paragraph::new(self.preview_content.as_str())
            .block(Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(colors::SECONDARY)))
            .wrap(Wrap { trim: true });
        
        f.render_widget(paragraph, area);
    }
    
    /// Render status bar
    fn render_status_bar(&self, f: &mut Frame) {
        let area = Rect {
            x: 0,
            y: f.area().height - 1,
            width: f.area().width,
            height: 1,
        };
        
        let help_text = "↑↓ Navigate • Enter Execute • Esc Quit";
        let status_text = format!("{} | {}", self.status_message, help_text);
        
        let paragraph = Paragraph::new(status_text)
            .style(Style::default().bg(colors::PRIMARY).fg(colors::BACKGROUND));
        
        f.render_widget(paragraph, area);
    }
    
    /// Run the history browser
    pub fn run(&mut self) -> io::Result<()> {
        let mut terminal = tui_common::setup_terminal()?;
        let result = self.run_app(&mut terminal);
        tui_common::restore_terminal(&mut terminal)?;
        result
    }
    
    /// Main application loop
    fn run_app<B: ratatui::backend::Backend + std::io::Write>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.render(f))?;
            self.handle_input()?;
            if self.should_quit {
                break;
            }
        }
        Ok(())
    }
}

/// Run the command history browser
pub fn run(limit: usize) -> io::Result<()> {
    let mut browser = HistoryBrowser::new(limit)?;
    browser.run()
}