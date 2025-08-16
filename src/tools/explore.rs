//! Interactive file/directory explorer with navigation.

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
    env,
    fs,
    io,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_directory: bool,
    pub size: Option<u64>,
    pub is_parent: bool,
}

pub struct FileExplorer {
    current_dir: PathBuf,
    entries: Vec<FileEntry>,
    list_state: ListState,
    should_quit: bool,
    status_message: String,
    preview_content: String,
}

impl FileExplorer {
    /// Create a new file explorer instance
    pub fn new(start_path: PathBuf) -> io::Result<Self> {
        let mut explorer = FileExplorer {
            current_dir: start_path.canonicalize().unwrap_or(start_path),
            entries: Vec::new(),
            list_state: ListState::default(),
            should_quit: false,
            status_message: String::new(),
            preview_content: String::new(),
        };
        
        explorer.load_directory()?;
        
        Ok(explorer)
    }
    
    /// Load current directory contents
    fn load_directory(&mut self) -> io::Result<()> {
        self.entries.clear();
        
        // Add parent directory entry if not at root
        if self.current_dir.parent().is_some() {
            self.entries.push(FileEntry {
                name: "..".to_string(),
                path: self.current_dir.parent().unwrap().to_path_buf(),
                is_directory: true,
                size: None,
                is_parent: true,
            });
        }
        
        // Read directory entries
        let mut entries = Vec::new();
        if let Ok(dir_entries) = fs::read_dir(&self.current_dir) {
            for entry in dir_entries.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                
                // Skip hidden files (starting with .)
                if name.starts_with('.') && name != ".." {
                    continue;
                }
                
                let is_directory = path.is_dir();
                let size = if is_directory {
                    None
                } else {
                    fs::metadata(&path).ok().map(|m| m.len())
                };
                
                entries.push(FileEntry {
                    name,
                    path,
                    is_directory,
                    size,
                    is_parent: false,
                });
            }
        }
        
        // Sort: directories first, then files, both alphabetically
        entries.sort_by(|a, b| {
            match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });
        
        self.entries.extend(entries);
        
        // Reset selection
        if !self.entries.is_empty() {
            self.list_state.select(Some(0));
            self.update_preview();
        } else {
            self.list_state.select(None);
            self.preview_content.clear();
        }
        
        self.status_message = format!("Directory: {} ({} items)", 
            self.current_dir.display(), 
            self.entries.len()
        );
        
        Ok(())
    }
    
    /// Update preview content for selected file
    fn update_preview(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if let Some(entry) = self.entries.get(selected) {
                self.preview_content = self.load_file_preview(&entry.path, entry.is_directory);
            }
        }
    }
    
    /// Load file preview content
    fn load_file_preview(&self, path: &Path, is_directory: bool) -> String {
        if is_directory {
            // For directories, show contents
            if let Ok(dir_entries) = fs::read_dir(path) {
                let mut contents = Vec::new();
                for entry in dir_entries.flatten().take(20) {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let icon = if entry.path().is_dir() { "📁" } else { "📄" };
                    contents.push(format!("{} {}", icon, name));
                }
                if contents.is_empty() {
                    "[Empty directory]".to_string()
                } else {
                    contents.join("\n")
                }
            } else {
                "[Permission denied]".to_string()
            }
        } else {
            // Check if it's an image file first
            if crate::image_preview::is_image_file(path) {
                return crate::image_preview::generate_image_preview(path);
            }
            
            // For files, show content preview
            match fs::read_to_string(path) {
                Ok(content) => {
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
                    KeyCode::Enter | KeyCode::Right => {
                        if let Some(selected) = self.list_state.selected() {
                            if let Some(entry) = self.entries.get(selected) {
                                if entry.is_directory {
                                    // Navigate to directory
                                    self.current_dir = entry.path.clone();
                                    self.load_directory()?;
                                } else {
                                    // Open file
                                    self.open_file(&entry.path)?;
                                    self.should_quit = true;
                                }
                            }
                        }
                    }
                    KeyCode::Left => {
                        // Go up one directory
                        if let Some(parent) = self.current_dir.parent() {
                            self.current_dir = parent.to_path_buf();
                            self.load_directory()?;
                        }
                    }
                    KeyCode::Char('h') => {
                        // Toggle hidden files (currently not implemented)
                        self.status_message = "Hidden files toggle not implemented yet".to_string();
                    }
                    KeyCode::Char('r') => {
                        // Refresh directory
                        self.load_directory()?;
                        self.status_message = "Directory refreshed".to_string();
                    }
                    KeyCode::Home => {
                        // Go to home directory
                        if let Ok(home) = env::var("HOME") {
                            self.current_dir = PathBuf::from(home);
                            self.load_directory()?;
                        }
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
    
    /// Render the file explorer interface
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
        let items: Vec<ListItem> = self.entries
            .iter()
            .map(|entry| {
                let icon = if entry.is_parent {
                    "⬆️ "
                } else if entry.is_directory {
                    "📁 "
                } else {
                    "📄 "
                };
                
                let size_info = if let Some(size) = entry.size {
                    format!(" ({})", format_size(size))
                } else {
                    String::new()
                };
                
                let line = Line::from(vec![
                    Span::raw(icon),
                    Span::styled(
                        &entry.name,
                        if entry.is_directory {
                            Style::default().fg(colors::PRIMARY).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(colors::TEXT)
                        }
                    ),
                    Span::styled(
                        size_info,
                        Style::default().fg(colors::SECONDARY)
                    ),
                ]);
                
                ListItem::new(line)
            })
            .collect();
        
        let title = format!("Files & Directories ({})", self.entries.len());
        
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
            if let Some(entry) = self.entries.get(selected) {
                format!("Preview: {}", entry.name)
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
        
        let help_text = "↑↓ Navigate • Enter/→ Open • ← Back • Home Home • R Refresh • Esc Quit";
        let status_text = if !self.status_message.is_empty() {
            format!("{} | {}", self.status_message, help_text)
        } else {
            help_text.to_string()
        };
        
        let paragraph = Paragraph::new(status_text)
            .style(Style::default().bg(colors::PRIMARY).fg(colors::BACKGROUND));
        
        f.render_widget(paragraph, area);
    }
    
    /// Run the file explorer application
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

/// Format file size in human readable format
fn format_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{:.0}{}", size, UNITS[unit_index])
    } else {
        format!("{:.1}{}", size, UNITS[unit_index])
    }
}

/// Run the file explorer tool
pub fn run(path: PathBuf) -> io::Result<()> {
    let mut explorer = FileExplorer::new(path)?;
    explorer.run()
}