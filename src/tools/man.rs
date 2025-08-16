//! Man page browser with search and preview.

use crate::tui_common::{self, colors};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::{
    io,
    process::{Command, Stdio},
    time::Duration,
};

#[derive(Debug, Clone)]
pub struct ManPage {
    pub name: String,
    pub section: String,
    pub description: String,
}

pub struct ManPageBrowser {
    man_pages: Vec<ManPage>,
    filtered_pages: Vec<ManPage>,
    list_state: ListState,
    search_query: String,
    should_quit: bool,
    status_message: String,
    preview_content: String,
}

impl ManPageBrowser {
    /// Create a new man page browser
    pub fn new(search: Option<String>) -> io::Result<Self> {
        let mut browser = ManPageBrowser {
            man_pages: Vec::new(),
            filtered_pages: Vec::new(),
            list_state: ListState::default(),
            search_query: search.unwrap_or_default(),
            should_quit: false,
            status_message: "Loading man pages...".to_string(),
            preview_content: String::new(),
        };
        
        browser.load_man_pages()?;
        browser.update_filter();
        
        Ok(browser)
    }
    
    /// Load available man pages
    fn load_man_pages(&mut self) -> io::Result<()> {
        // Try to use apropos to get all man pages
        let output = Command::new("apropos")
            .arg(".")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()?;
        
        if output.status.success() {
            let apropos_output = String::from_utf8_lossy(&output.stdout);
            
            for line in apropos_output.lines() {
                if let Some(man_page) = self.parse_apropos_line(line) {
                    self.man_pages.push(man_page);
                }
            }
        } else {
            // Fallback: try to load from common man page directories
            self.load_from_man_directories()?;
        }
        
        // Sort by name
        self.man_pages.sort_by(|a, b| a.name.cmp(&b.name));
        
        if !self.man_pages.is_empty() {
            self.list_state.select(Some(0));
            self.update_preview();
        }
        
        self.status_message = format!("Loaded {} man pages", self.man_pages.len());
        Ok(())
    }
    
    /// Parse apropos output line
    fn parse_apropos_line(&self, line: &str) -> Option<ManPage> {
        // Format: "command (section) - description"
        if let Some(desc_start) = line.find(" - ") {
            let command_section = &line[..desc_start];
            let description = line[desc_start + 3..].to_string();
            
            // Extract command and section
            if let Some(paren_start) = command_section.find(" (") {
                let command = command_section[..paren_start].trim().to_string();
                if let Some(paren_end) = command_section.find(')') {
                    let section = command_section[paren_start + 2..paren_end].to_string();
                    
                    return Some(ManPage {
                        name: command,
                        section,
                        description,
                    });
                }
            }
        }
        None
    }
    
    /// Fallback: Load from man directories
    fn load_from_man_directories(&mut self) -> io::Result<()> {
        // Common man page commands to include
        let common_commands = [
            ("ls", "1", "list directory contents"),
            ("cd", "1", "change directory"),
            ("cp", "1", "copy files"),
            ("mv", "1", "move files"),
            ("rm", "1", "remove files"),
            ("cat", "1", "concatenate files"),
            ("grep", "1", "search text patterns"),
            ("find", "1", "search for files"),
            ("ps", "1", "show running processes"),
            ("top", "1", "display running processes"),
            ("kill", "1", "terminate processes"),
            ("man", "1", "display manual pages"),
            ("vim", "1", "text editor"),
            ("nano", "1", "text editor"),
            ("git", "1", "version control system"),
            ("ssh", "1", "secure shell"),
            ("wget", "1", "download files"),
            ("curl", "1", "transfer data"),
            ("tar", "1", "archive files"),
            ("chmod", "1", "change file permissions"),
            ("chown", "1", "change file ownership"),
        ];
        
        for (name, section, desc) in common_commands.iter() {
            // Check if man page actually exists
            let check_output = Command::new("man")
                .args(&["-w", name])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            
            if check_output.is_ok() {
                self.man_pages.push(ManPage {
                    name: name.to_string(),
                    section: section.to_string(),
                    description: desc.to_string(),
                });
            }
        }
        
        Ok(())
    }
    
    /// Update filtered man pages based on search query
    fn update_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_pages = self.man_pages.clone();
        } else {
            let query = self.search_query.to_lowercase();
            self.filtered_pages = self.man_pages
                .iter()
                .filter(|page| {
                    page.name.to_lowercase().contains(&query) ||
                    page.description.to_lowercase().contains(&query)
                })
                .cloned()
                .collect();
        }
        
        // Reset selection
        if !self.filtered_pages.is_empty() {
            self.list_state.select(Some(0));
            self.update_preview();
        } else {
            self.list_state.select(None);
            self.preview_content.clear();
        }
    }
    
    /// Update preview content for selected man page
    fn update_preview(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if let Some(page) = self.filtered_pages.get(selected) {
                self.preview_content = self.load_man_page_preview(&page.name, &page.section);
            }
        }
    }
    
    /// Load man page preview content
    fn load_man_page_preview(&self, name: &str, section: &str) -> String {
        // Try to get man page content
        let output = Command::new("man")
            .args(&[section, name])
            .env("MANPAGER", "cat")  // Disable paging
            .env("MANWIDTH", "80")   // Set width
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output();
        
        match output {
            Ok(output) if output.status.success() => {
                let content = String::from_utf8_lossy(&output.stdout);
                // Take first 50 lines for preview
                let lines: Vec<&str> = content.lines().take(50).collect();
                lines.join("\n")
            }
            _ => {
                // Fallback: try whatis command for description
                let whatis_output = Command::new("whatis")
                    .arg(name)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .output();
                
                match whatis_output {
                    Ok(output) if output.status.success() => {
                        let description = String::from_utf8_lossy(&output.stdout);
                        format!("Manual page for: {}\n\n{}\n\nUse 'man {}' to view the full manual page.", name, description.trim(), name)
                    }
                    _ => {
                        format!("Manual page for: {}\nSection: {}\n\nNo preview available.\nUse 'man {}' to view the manual page.", name, section, name)
                    }
                }
            }
        }
    }
    
    /// Open selected man page in full viewer
    fn open_man_page(&mut self) -> io::Result<()> {
        if let Some(selected) = self.list_state.selected() {
            if let Some(page) = self.filtered_pages.get(selected) {
                // Open man page in default pager
                let status = Command::new("man")
                    .args(&[&page.section, &page.name])
                    .status();
                
                if status.is_ok() {
                    self.should_quit = true;
                } else {
                    self.status_message = format!("Failed to open man page for {}", page.name);
                }
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
                            key.code, key.modifiers, self.list_state.selected(), self.filtered_pages.len(), 10
                        ) {
                            self.list_state.select(Some(new_selection));
                            self.update_preview();
                        }
                    }
                    KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        // Page up
                        if let Some(new_selection) = tui_common::handle_page_navigation(
                            key.code, key.modifiers, self.list_state.selected(), self.filtered_pages.len(), 10
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
                            if selected + 1 < self.filtered_pages.len() {
                                self.list_state.select(Some(selected + 1));
                                self.update_preview();
                            }
                        } else if !self.filtered_pages.is_empty() {
                            self.list_state.select(Some(0));
                            self.update_preview();
                        }
                    }
                    KeyCode::Enter => {
                        self.open_man_page()?;
                    }
                    KeyCode::Char(c) => {
                        self.search_query.push(c);
                        self.update_filter();
                    }
                    KeyCode::Backspace => {
                        self.search_query.pop();
                        self.update_filter();
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
    
    /// Render the man page browser
    fn render(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(f.area());
        
        self.render_man_page_list(f, chunks[0]);
        self.render_man_page_preview(f, chunks[1]);
        self.render_status_bar(f);
    }
    
    /// Render man page list
    fn render_man_page_list(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.filtered_pages
            .iter()
            .map(|page| {
                let line = Line::from(vec![
                    Span::styled(
                        format!("{}({})", page.name, page.section),
                        Style::default().fg(colors::PRIMARY).add_modifier(Modifier::BOLD)
                    ),
                    Span::raw(" - "),
                    Span::styled(
                        page.description.chars().take(60).collect::<String>(),
                        Style::default().fg(colors::TEXT)
                    ),
                ]);
                ListItem::new(line)
            })
            .collect();
        
        let title = if self.search_query.is_empty() {
            format!("Manual Pages ({})", self.filtered_pages.len())
        } else {
            format!("Manual Pages ({}) - Filter: '{}'", self.filtered_pages.len(), self.search_query)
        };
        
        let list = List::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(colors::PRIMARY)))
            .highlight_style(Style::default()
                .bg(colors::PRIMARY)
                .fg(colors::BACKGROUND)
                .add_modifier(Modifier::BOLD))
            .highlight_symbol("► ");
        
        f.render_stateful_widget(list, area, &mut self.list_state);
    }
    
    /// Render man page preview
    fn render_man_page_preview(&self, f: &mut Frame, area: Rect) {
        let title = if let Some(selected) = self.list_state.selected() {
            if let Some(page) = self.filtered_pages.get(selected) {
                format!("Preview: {}({})", page.name, page.section)
            } else {
                "Preview".to_string()
            }
        } else {
            "Preview".to_string()
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
        
        let help_text = "Type to filter • ↑↓ Navigate • Enter Open • Esc Quit";
        let status_text = format!("{} | {}", self.status_message, help_text);
        
        let paragraph = Paragraph::new(status_text)
            .style(Style::default().bg(colors::PRIMARY).fg(colors::BACKGROUND));
        
        f.render_widget(paragraph, area);
    }
    
    /// Run the man page browser
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

/// Run the man page browser
pub fn run(search: Option<String>) -> io::Result<()> {
    let mut browser = ManPageBrowser::new(search)?;
    browser.run()
}