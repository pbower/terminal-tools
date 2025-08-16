//! File finder tool with fuzzy search and preview.

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
    fs,
    io,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
use walkdir::WalkDir;

pub struct FileFinder {
    files: Vec<PathBuf>,
    filtered_files: Vec<PathBuf>,
    list_state: ListState,
    search_query: String,
    preview_content: String,
    should_quit: bool,
    status_message: String,
}

impl FileFinder {
    /// Create a new file finder instance
    pub fn new(start_path: PathBuf, extensions: Option<String>, initial_search: Option<String>) -> io::Result<Self> {
        let mut finder = FileFinder {
            files: Vec::new(),
            filtered_files: Vec::new(),
            list_state: ListState::default(),
            search_query: initial_search.unwrap_or_default(),
            preview_content: String::new(),
            should_quit: false,
            status_message: "Loading files...".to_string(),
        };
        
        finder.load_files(start_path, extensions)?;
        finder.update_filter();
        
        Ok(finder)
    }
    
    /// Load all files from the starting path
    fn load_files(&mut self, start_path: PathBuf, extensions: Option<String>) -> io::Result<()> {
        let ext_filter: Option<Vec<String>> = extensions.map(|exts| {
            exts.split(',').map(|s| s.trim().to_lowercase()).collect()
        });
        
        for entry in WalkDir::new(start_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                let path = entry.path().to_path_buf();
                
                // Filter by extension if specified
                if let Some(ref filters) = ext_filter {
                    if let Some(ext) = path.extension() {
                        let ext_str = ext.to_string_lossy().to_lowercase();
                        if !filters.contains(&ext_str) {
                            continue;
                        }
                    } else {
                        continue; // Skip files without extensions when filtering
                    }
                }
                
                // Skip hidden files and common build directories
                let path_str = path.to_string_lossy();
                if path_str.contains("/.git/") || 
                   path_str.contains("/node_modules/") || 
                   path_str.contains("/target/") ||
                   path_str.contains("/.vscode/") {
                    continue;
                }
                
                self.files.push(path);
            }
        }
        
        self.status_message = format!("Found {} files", self.files.len());
        Ok(())
    }
    
    /// Update filtered files based on search query
    fn update_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_files = self.files.clone();
        } else {
            let query = self.search_query.to_lowercase();
            self.filtered_files = self.files
                .iter()
                .filter(|path| {
                    path.to_string_lossy().to_lowercase().contains(&query)
                })
                .cloned()
                .collect();
        }
        
        // Reset selection
        if !self.filtered_files.is_empty() {
            self.list_state.select(Some(0));
            self.update_preview();
        } else {
            self.list_state.select(None);
            self.preview_content.clear();
        }
    }
    
    /// Update preview content for selected file
    fn update_preview(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if let Some(path) = self.filtered_files.get(selected) {
                self.preview_content = self.load_file_preview(path);
            }
        }
    }
    
    /// Load file preview content
    fn load_file_preview(&self, path: &Path) -> String {
        // Check if it's an image file first
        if crate::image_preview::is_image_file(path) {
            return crate::image_preview::generate_image_preview(path);
        }
        
        // Try to read file content
        match fs::read_to_string(path) {
            Ok(content) => {
                // Limit preview to first 50 lines
                let lines: Vec<&str> = content.lines().take(50).collect();
                lines.join("\n")
            }
            Err(_) => {
                // For binary files or read errors, show file info
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
                            key.code, key.modifiers, self.list_state.selected(), self.filtered_files.len(), 10
                        ) {
                            self.list_state.select(Some(new_selection));
                            self.update_preview();
                        }
                    }
                    KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        // Page up
                        if let Some(new_selection) = tui_common::handle_page_navigation(
                            key.code, key.modifiers, self.list_state.selected(), self.filtered_files.len(), 10
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
                            if selected + 1 < self.filtered_files.len() {
                                self.list_state.select(Some(selected + 1));
                                self.update_preview();
                            }
                        } else if !self.filtered_files.is_empty() {
                            self.list_state.select(Some(0));
                            self.update_preview();
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(selected) = self.list_state.selected() {
                            if let Some(path) = self.filtered_files.get(selected) {
                                self.open_file(path)?;
                                self.should_quit = true;
                            }
                        }
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
    
    /// Open selected file in default editor
    fn open_file(&self, path: &Path) -> io::Result<()> {
        // Try different editors in order of preference
        let editors = ["nvim", "vim", "nano", "code"];
        
        for editor in editors.iter() {
            let result = Command::new(editor)
                .arg(path)
                .status();
                
            if result.is_ok() {
                return Ok(());
            }
        }
        
        // If no editor found, just print the path
        println!("{}", path.display());
        Ok(())
    }
    
    /// Render the file finder interface
    fn render(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(f.area());
        
        // Left panel - file list
        self.render_file_list(f, chunks[0]);
        
        // Right panel - preview
        self.render_preview(f, chunks[1]);
        
        // Status bar
        self.render_status_bar(f);
    }
    
    /// Render the file list panel
    fn render_file_list(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.filtered_files
            .iter()
            .map(|path| {
                let display_path = if let Ok(current_dir) = std::env::current_dir() {
                    path.strip_prefix(&current_dir)
                        .unwrap_or(path)
                        .display()
                        .to_string()
                } else {
                    path.display().to_string()
                };
                
                ListItem::new(Line::from(display_path))
            })
            .collect();
        
        let title = if self.search_query.is_empty() {
            format!("Files ({})", self.filtered_files.len())
        } else {
            format!("Files ({}) - Filter: '{}'", self.filtered_files.len(), self.search_query)
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
    
    /// Render the preview panel
    fn render_preview(&self, f: &mut Frame, area: Rect) {
        let title = if let Some(selected) = self.list_state.selected() {
            if let Some(path) = self.filtered_files.get(selected) {
                format!("Preview: {}", path.file_name().unwrap_or_default().to_string_lossy())
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
        
        let help_text = "Type to filter • ↑↓ Navigate • Ctrl-F/B Page • Enter Open • Esc Quit";
        let status_text = if !self.status_message.is_empty() {
            format!("{} | {}", self.status_message, help_text)
        } else {
            help_text.to_string()
        };
        
        let paragraph = Paragraph::new(status_text)
            .style(Style::default().bg(colors::PRIMARY).fg(colors::BACKGROUND));
        
        f.render_widget(paragraph, area);
    }
    
    /// Run the file finder application
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

/// Run the file finder tool
pub fn run(path: PathBuf, extensions: Option<String>, search: Option<String>) -> io::Result<()> {
    let mut finder = FileFinder::new(path, extensions, search)?;
    finder.run()
}