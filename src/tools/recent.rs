//! Recent files browser with MRU tracking.

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
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

#[derive(Debug, Clone)]
pub struct RecentFile {
    pub path: PathBuf,
    pub display_name: String,
}

pub struct RecentFileBrowser {
    files: Vec<RecentFile>,
    list_state: ListState,
    should_quit: bool,
    status_message: String,
    preview_content: String,
    limit: usize,
}

impl RecentFileBrowser {
    /// Create a new recent file browser
    pub fn new(limit: usize) -> io::Result<Self> {
        let mut browser = RecentFileBrowser {
            files: Vec::new(),
            list_state: ListState::default(),
            should_quit: false,
            status_message: "Loading recent files...".to_string(),
            preview_content: String::new(),
            limit,
        };
        
        browser.load_recent_files()?;
        
        Ok(browser)
    }
    
    /// Load recent files from various sources
    fn load_recent_files(&mut self) -> io::Result<()> {
        // Try to load from our MRU file (like the bash version)
        if let Ok(home) = env::var("HOME") {
            let mru_file = PathBuf::from(home).join(".cache/fzf-mru.txt");
            if let Ok(content) = fs::read_to_string(mru_file) {
                for line in content.lines().rev().take(self.limit) {
                    let path = PathBuf::from(line.trim());
                    if path.exists() {
                        self.files.push(RecentFile {
                            display_name: path.file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string(),
                            path,
                        });
                    }
                }
            } else {
                // Fallback: find recently modified files in common directories
                self.load_recently_modified_files()?;
            }
        }
        
        if !self.files.is_empty() {
            self.list_state.select(Some(0));
            self.update_preview();
        }
        
        self.status_message = format!("Found {} recent files", self.files.len());
        Ok(())
    }
    
    /// Load recently modified files as fallback
    fn load_recently_modified_files(&mut self) -> io::Result<()> {
        let dirs = [
            env::current_dir().unwrap_or_default(),
            PathBuf::from(env::var("HOME").unwrap_or_default()),
        ];
        
        for dir in dirs.iter() {
            if dir.exists() {
                // Use find command to get recently modified files
                let output = Command::new("find")
                    .args(&[
                        dir.to_str().unwrap_or("."),
                        "-type", "f",
                        "-not", "-path", "*/.*",
                        "-mtime", "-7",
                        "-printf", "%T@ %p\n"
                    ])
                    .output();
                
                if let Ok(output) = output {
                    if output.status.success() {
                        let mut files_with_time: Vec<(f64, PathBuf)> = Vec::new();
                        let find_output = String::from_utf8_lossy(&output.stdout);
                        
                        for line in find_output.lines() {
                            if let Some(space_pos) = line.find(' ') {
                                if let Ok(time) = line[..space_pos].parse::<f64>() {
                                    let path = PathBuf::from(&line[space_pos + 1..]);
                                    files_with_time.push((time, path));
                                }
                            }
                        }
                        
                        // Sort by modification time (newest first)
                        files_with_time.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
                        
                        for (_, path) in files_with_time.into_iter().take(self.limit) {
                            self.files.push(RecentFile {
                                display_name: path.file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string(),
                                path,
                            });
                        }
                        break; // Only need one directory to succeed
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Update preview content
    fn update_preview(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if let Some(file) = self.files.get(selected) {
                self.preview_content = self.load_file_preview(&file.path);
            }
        }
    }
    
    /// Load file preview
    fn load_file_preview(&self, path: &Path) -> String {
        match fs::read_to_string(path) {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().take(50).collect();
                lines.join("\n")
            }
            Err(_) => {
                if let Ok(metadata) = fs::metadata(path) {
                    format!(
                        "File: {}\nSize: {} bytes\nModified: {:?}\n\n[Binary file or read error]",
                        path.display(),
                        metadata.len(),
                        metadata.modified().ok()
                    )
                } else {
                    "[Could not read file]".to_string()
                }
            }
        }
    }
    
    /// Open selected file
    fn open_file(&mut self) -> io::Result<()> {
        if let Some(selected) = self.list_state.selected() {
            if let Some(file) = self.files.get(selected) {
                let editors = ["nvim", "vim", "nano", "code"];
                
                for editor in editors.iter() {
                    let result = Command::new(editor)
                        .arg(&file.path)
                        .status();
                        
                    if result.is_ok() {
                        self.should_quit = true;
                        return Ok(());
                    }
                }
                
                println!("{}", file.path.display());
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
                            key.code, key.modifiers, self.list_state.selected(), self.files.len(), 10
                        ) {
                            self.list_state.select(Some(new_selection));
                            self.update_preview();
                        }
                    }
                    KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        // Page up
                        if let Some(new_selection) = tui_common::handle_page_navigation(
                            key.code, key.modifiers, self.list_state.selected(), self.files.len(), 10
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
                            if selected + 1 < self.files.len() {
                                self.list_state.select(Some(selected + 1));
                                self.update_preview();
                            }
                        } else if !self.files.is_empty() {
                            self.list_state.select(Some(0));
                            self.update_preview();
                        }
                    }
                    KeyCode::Enter => {
                        self.open_file()?;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
    
    /// Render the recent files browser
    fn render(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(f.area());
        
        self.render_file_list(f, chunks[0]);
        self.render_preview(f, chunks[1]);
        self.render_status_bar(f);
    }
    
    /// Render file list
    fn render_file_list(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.files
            .iter()
            .map(|file| {
                let line = Line::from(format!("{} ({})", 
                    file.display_name,
                    file.path.parent()
                        .unwrap_or_else(|| Path::new("/"))
                        .display()
                ));
                ListItem::new(line)
            })
            .collect();
        
        let list = List::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(format!("Recent Files ({})", self.files.len()))
                .border_style(Style::default().fg(colors::PRIMARY)))
            .highlight_style(Style::default()
                .bg(colors::PRIMARY)
                .fg(colors::BACKGROUND)
                .add_modifier(Modifier::BOLD))
            .highlight_symbol("► ");
        
        f.render_stateful_widget(list, area, &mut self.list_state);
    }
    
    /// Render preview
    fn render_preview(&self, f: &mut Frame, area: Rect) {
        let title = if let Some(selected) = self.list_state.selected() {
            if let Some(file) = self.files.get(selected) {
                format!("Preview: {}", file.display_name)
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
        
        let help_text = "↑↓ Navigate • Enter Open • Esc Quit";
        let status_text = format!("{} | {}", self.status_message, help_text);
        
        let paragraph = Paragraph::new(status_text)
            .style(Style::default().bg(colors::PRIMARY).fg(colors::BACKGROUND));
        
        f.render_widget(paragraph, area);
    }
    
    /// Run the recent files browser
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

/// Run the recent files browser
pub fn run(limit: usize) -> io::Result<()> {
    let mut browser = RecentFileBrowser::new(limit)?;
    browser.run()
}