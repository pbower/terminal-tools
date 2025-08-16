//! Content search with ripgrep integration.

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
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub file_path: PathBuf,
    pub line_number: u32,
    pub line_content: String,
    #[allow(dead_code)]
    pub matched_text: String,
}

#[allow(dead_code)]
pub struct SearchBrowser {
    results: Vec<SearchResult>,
    list_state: ListState,
    should_quit: bool,
    status_message: String,
    preview_content: String,
    pattern: String,
    search_path: PathBuf,
}

#[allow(dead_code)]
impl SearchBrowser {
    /// Create a new search browser
    pub fn new(
        pattern: String,
        path: PathBuf,
        file_type: Option<String>,
        ignore_case: bool,
    ) -> io::Result<Self> {
        let mut browser = SearchBrowser {
            results: Vec::new(),
            list_state: ListState::default(),
            should_quit: false,
            status_message: format!("Searching for '{}'...", pattern),
            preview_content: String::new(),
            pattern: pattern.clone(),
            search_path: path.clone(),
        };
        
        browser.perform_search(&pattern, &path, file_type, ignore_case)?;
        
        Ok(browser)
    }
    
    /// Perform ripgrep search
    fn perform_search(
        &mut self,
        pattern: &str,
        path: &Path,
        file_type: Option<String>,
        ignore_case: bool,
    ) -> io::Result<()> {
        let mut cmd = Command::new("rg");
        
        // Basic ripgrep arguments
        cmd.args(&[
            "--line-number",  // Show line numbers
            "--with-filename", // Show file names
            "--no-heading",   // Don't group by file
            "--color=never",  // Disable colors for parsing
        ]);
        
        // Add case insensitive flag
        if ignore_case {
            cmd.arg("--ignore-case");
        }
        
        // Add file type filter
        if let Some(ft) = file_type {
            cmd.args(&["--type", &ft]);
        }
        
        // Add pattern and path
        cmd.arg(pattern);
        cmd.arg(path);
        
        let output = cmd.stdout(Stdio::piped()).output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("ripgrep") || stderr.contains("not found") {
                // Fallback to grep if ripgrep is not available
                self.perform_grep_search(pattern, path, ignore_case)?;
                return Ok(());
            } else {
                self.status_message = format!("Search error: {}", stderr.trim());
                return Ok(());
            }
        }
        
        let search_output = String::from_utf8_lossy(&output.stdout);
        
        for line in search_output.lines() {
            if let Some(result) = self.parse_ripgrep_line(line) {
                self.results.push(result);
            }
        }
        
        if !self.results.is_empty() {
            self.list_state.select(Some(0));
            self.update_preview();
        }
        
        self.status_message = format!("Found {} matches for '{}'", self.results.len(), pattern);
        Ok(())
    }
    
    /// Fallback to grep if ripgrep is not available
    fn perform_grep_search(&mut self, pattern: &str, path: &Path, ignore_case: bool) -> io::Result<()> {
        let mut cmd = Command::new("grep");
        
        cmd.args(&["-rn"]); // Recursive, line numbers
        
        if ignore_case {
            cmd.arg("-i");
        }
        
        cmd.arg(pattern);
        cmd.arg(path);
        
        let output = cmd.stdout(Stdio::piped()).output()?;
        
        if output.status.success() {
            let grep_output = String::from_utf8_lossy(&output.stdout);
            
            for line in grep_output.lines() {
                if let Some(result) = self.parse_grep_line(line) {
                    self.results.push(result);
                }
            }
        }
        
        self.status_message = format!("Found {} matches using grep fallback", self.results.len());
        Ok(())
    }
    
    /// Parse ripgrep output line
    fn parse_ripgrep_line(&self, line: &str) -> Option<SearchResult> {
        // Format: filename:line_number:line_content
        let parts: Vec<&str> = line.splitn(3, ':').collect();
        if parts.len() >= 3 {
            let file_path = PathBuf::from(parts[0]);
            if let Ok(line_number) = parts[1].parse::<u32>() {
                let line_content = parts[2].to_string();
                let matched_text = self.extract_match(&line_content);
                
                return Some(SearchResult {
                    file_path,
                    line_number,
                    line_content,
                    matched_text,
                });
            }
        }
        None
    }
    
    /// Parse grep output line
    fn parse_grep_line(&self, line: &str) -> Option<SearchResult> {
        // Similar format to ripgrep
        self.parse_ripgrep_line(line)
    }
    
    /// Extract the matched portion of text
    fn extract_match(&self, line_content: &str) -> String {
        // Simple case-insensitive match extraction
        let pattern_lower = self.pattern.to_lowercase();
        let content_lower = line_content.to_lowercase();
        
        if let Some(start) = content_lower.find(&pattern_lower) {
            let end = start + self.pattern.len();
            if end <= line_content.len() {
                return line_content[start..end].to_string();
            }
        }
        
        self.pattern.clone()
    }
    
    /// Update preview content for selected result
    fn update_preview(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if let Some(result) = self.results.get(selected) {
                self.preview_content = self.load_file_context(&result.file_path, result.line_number);
            }
        }
    }
    
    /// Load file context around the matched line
    fn load_file_context(&self, file_path: &Path, line_number: u32) -> String {
        match std::fs::read_to_string(file_path) {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().collect();
                let line_idx = (line_number as usize).saturating_sub(1);
                
                // Show context: 5 lines before and after
                let start = line_idx.saturating_sub(5);
                let end = std::cmp::min(line_idx + 6, lines.len());
                
                let mut context_lines = Vec::new();
                for i in start..end {
                    let marker = if i == line_idx { ">>>" } else { "   " };
                    context_lines.push(format!("{} {:4}: {}", marker, i + 1, lines[i]));
                }
                
                context_lines.join("\n")
            }
            Err(_) => format!("Could not read file: {}", file_path.display()),
        }
    }
    
    /// Open file at specific line in editor
    fn open_file(&mut self) -> io::Result<()> {
        if let Some(selected) = self.list_state.selected() {
            if let Some(result) = self.results.get(selected) {
                // Try to open with line number support
                let editors_with_line = [
                    ("nvim", format!("+{}", result.line_number)),
                    ("vim", format!("+{}", result.line_number)),
                    ("code", format!("--goto {}:{}", result.file_path.display(), result.line_number)),
                ];
                
                for (editor, line_arg) in editors_with_line.iter() {
                    let mut cmd = Command::new(editor);
                    if editor == &"code" {
                        cmd.arg(&line_arg);
                    } else {
                        cmd.arg(&line_arg).arg(&result.file_path);
                    }
                    
                    if cmd.status().is_ok() {
                        self.should_quit = true;
                        return Ok(());
                    }
                }
                
                // Fallback to basic file opening
                println!("{}", result.file_path.display());
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
                            if selected + 1 < self.results.len() {
                                self.list_state.select(Some(selected + 1));
                                self.update_preview();
                            }
                        } else if !self.results.is_empty() {
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
    
    /// Render the search browser
    fn render(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(f.area());
        
        self.render_results_list(f, chunks[0]);
        self.render_file_preview(f, chunks[1]);
        self.render_status_bar(f);
    }
    
    /// Render search results list
    fn render_results_list(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.results
            .iter()
            .map(|result| {
                let file_name = result.file_path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                
                let line = Line::from(vec![
                    Span::styled(
                        format!("{}", file_name),
                        Style::default().fg(colors::PRIMARY).add_modifier(Modifier::BOLD)
                    ),
                    Span::styled(
                        format!(":{}", result.line_number),
                        Style::default().fg(colors::SECONDARY)
                    ),
                    Span::raw(" "),
                    Span::styled(
                        result.line_content.trim(),
                        Style::default().fg(colors::TEXT)
                    ),
                ]);
                
                ListItem::new(line)
            })
            .collect();
        
        let title = format!("Search Results for '{}' ({})", self.pattern, self.results.len());
        
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
    
    /// Render file preview
    fn render_file_preview(&self, f: &mut Frame, area: Rect) {
        let title = if let Some(selected) = self.list_state.selected() {
            if let Some(result) = self.results.get(selected) {
                format!("Context: {}", result.file_path.display())
            } else {
                "Context".to_string()
            }
        } else {
            "Context".to_string()
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
    
    /// Run the search browser
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

/// Live search browser with real-time ripgrep integration
pub struct LiveSearchBrowser {
    search_query: String,
    results: Vec<SearchResult>,
    list_state: ListState,
    should_quit: bool,
    status_message: String,
    preview_content: String,
    search_path: PathBuf,
    file_type: Option<String>,
    ignore_case: bool,
    is_searching: bool,
}

impl LiveSearchBrowser {
    /// Create a new live search browser
    pub fn new(
        initial_pattern: Option<String>,
        path: PathBuf,
        file_type: Option<String>,
        ignore_case: bool,
    ) -> io::Result<Self> {
        let mut browser = LiveSearchBrowser {
            search_query: initial_pattern.unwrap_or_default(),
            results: Vec::new(),
            list_state: ListState::default(),
            should_quit: false,
            status_message: "Type to search with ripgrep...".to_string(),
            preview_content: String::new(),
            search_path: path,
            file_type,
            ignore_case,
            is_searching: false,
        };
        
        // If we have an initial pattern, search immediately
        if !browser.search_query.is_empty() {
            browser.perform_live_search()?;
        }
        
        Ok(browser)
    }
    
    /// Perform live search as user types
    fn perform_live_search(&mut self) -> io::Result<()> {
        if self.search_query.len() < 2 {
            self.results.clear();
            self.status_message = "Type at least 2 characters to search...".to_string();
            return Ok(());
        }
        
        self.is_searching = true;
        self.status_message = format!("Searching for '{}'...", self.search_query);
        
        let mut cmd = Command::new("rg");
        
        // Basic ripgrep arguments for fast search
        cmd.args(&[
            "--line-number",
            "--with-filename", 
            "--no-heading",
            "--color=never",
            "--max-count=100", // Limit results for performance
        ]);
        
        if self.ignore_case {
            cmd.arg("--ignore-case");
        }
        
        if let Some(ref ft) = self.file_type {
            cmd.args(&["--type", ft]);
        }
        
        cmd.arg(&self.search_query);
        cmd.arg(&self.search_path);
        
        let output = cmd.stdout(Stdio::piped()).output()?;
        
        self.results.clear();
        
        if output.status.success() {
            let search_output = String::from_utf8_lossy(&output.stdout);
            
            for line in search_output.lines() {
                if let Some(result) = self.parse_ripgrep_line(line) {
                    self.results.push(result);
                }
            }
        }
        
        if !self.results.is_empty() {
            self.list_state.select(Some(0));
            self.update_preview();
        } else {
            self.list_state.select(None);
            self.preview_content.clear();
        }
        
        self.status_message = format!("Found {} matches for '{}'", self.results.len(), self.search_query);
        self.is_searching = false;
        Ok(())
    }
    
    /// Parse ripgrep output line
    fn parse_ripgrep_line(&self, line: &str) -> Option<SearchResult> {
        let parts: Vec<&str> = line.splitn(3, ':').collect();
        if parts.len() >= 3 {
            let file_path = PathBuf::from(parts[0]);
            if let Ok(line_number) = parts[1].parse::<u32>() {
                let line_content = parts[2].to_string();
                let matched_text = self.extract_match(&line_content);
                
                return Some(SearchResult {
                    file_path,
                    line_number,
                    line_content,
                    matched_text,
                });
            }
        }
        None
    }
    
    /// Extract the matched portion of text
    fn extract_match(&self, line_content: &str) -> String {
        let pattern_lower = self.search_query.to_lowercase();
        let content_lower = line_content.to_lowercase();
        
        if let Some(start) = content_lower.find(&pattern_lower) {
            let end = start + self.search_query.len();
            if end <= line_content.len() {
                return line_content[start..end].to_string();
            }
        }
        
        self.search_query.clone()
    }
    
    /// Update preview content
    fn update_preview(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if let Some(result) = self.results.get(selected) {
                self.preview_content = self.load_file_context(&result.file_path, result.line_number);
            }
        }
    }
    
    /// Load file context around matched line
    fn load_file_context(&self, file_path: &Path, line_number: u32) -> String {
        match std::fs::read_to_string(file_path) {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().collect();
                let line_idx = (line_number as usize).saturating_sub(1);
                
                let start = line_idx.saturating_sub(5);
                let end = std::cmp::min(line_idx + 6, lines.len());
                
                let mut context_lines = Vec::new();
                for i in start..end {
                    let marker = if i == line_idx { ">>>" } else { "   " };
                    context_lines.push(format!("{} {:4}: {}", marker, i + 1, lines[i]));
                }
                
                context_lines.join("\n")
            }
            Err(_) => format!("Could not read file: {}", file_path.display()),
        }
    }
    
    /// Open file at specific line
    fn open_file(&mut self) -> io::Result<()> {
        if let Some(selected) = self.list_state.selected() {
            if let Some(result) = self.results.get(selected) {
                let editors_with_line = [
                    ("nvim", format!("+{}", result.line_number)),
                    ("vim", format!("+{}", result.line_number)),
                    ("code", format!("--goto {}:{}", result.file_path.display(), result.line_number)),
                ];
                
                for (editor, line_arg) in editors_with_line.iter() {
                    let mut cmd = Command::new(editor);
                    if editor == &"code" {
                        cmd.arg(&line_arg);
                    } else {
                        cmd.arg(&line_arg).arg(&result.file_path);
                    }
                    
                    if cmd.status().is_ok() {
                        self.should_quit = true;
                        return Ok(());
                    }
                }
                
                println!("{}", result.file_path.display());
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
                            key.code, key.modifiers, self.list_state.selected(), self.results.len(), 10
                        ) {
                            self.list_state.select(Some(new_selection));
                            self.update_preview();
                        }
                    }
                    KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        // Page up
                        if let Some(new_selection) = tui_common::handle_page_navigation(
                            key.code, key.modifiers, self.list_state.selected(), self.results.len(), 10
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
                            if selected + 1 < self.results.len() {
                                self.list_state.select(Some(selected + 1));
                                self.update_preview();
                            }
                        } else if !self.results.is_empty() {
                            self.list_state.select(Some(0));
                            self.update_preview();
                        }
                    }
                    KeyCode::Enter => {
                        self.open_file()?;
                    }
                    KeyCode::Char(c) => {
                        self.search_query.push(c);
                        self.perform_live_search()?;
                    }
                    KeyCode::Backspace => {
                        self.search_query.pop();
                        if self.search_query.is_empty() {
                            self.results.clear();
                            self.list_state.select(None);
                            self.preview_content.clear();
                            self.status_message = "Type to search with ripgrep...".to_string();
                        } else {
                            self.perform_live_search()?;
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
    
    /// Render the live search browser
    fn render(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(5), Constraint::Length(1)])
            .split(f.area());
        
        // Search input
        self.render_search_input(f, chunks[0]);
        
        // Split main area for results and preview
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(chunks[1]);
        
        self.render_results_list(f, main_chunks[0]);
        self.render_file_preview(f, main_chunks[1]);
        
        // Status bar
        self.render_status_bar(f, chunks[2]);
    }
    
    /// Render search input
    fn render_search_input(&self, f: &mut Frame, area: Rect) {
        let search_text = if self.is_searching {
            format!("🔍 Searching: {}", self.search_query)
        } else {
            format!("🔍 Search: {}", self.search_query)
        };
        
        let paragraph = Paragraph::new(search_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Live Search (ripgrep)")
                .border_style(Style::default().fg(colors::PRIMARY)));
        
        f.render_widget(paragraph, area);
    }
    
    /// Render search results
    fn render_results_list(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.results
            .iter()
            .map(|result| {
                let file_name = result.file_path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                
                let line = Line::from(vec![
                    Span::styled(
                        format!("{}", file_name),
                        Style::default().fg(colors::PRIMARY).add_modifier(Modifier::BOLD)
                    ),
                    Span::styled(
                        format!(":{}", result.line_number),
                        Style::default().fg(colors::SECONDARY)
                    ),
                    Span::raw(" "),
                    Span::styled(
                        result.line_content.trim(),
                        Style::default().fg(colors::TEXT)
                    ),
                ]);
                
                ListItem::new(line)
            })
            .collect();
        
        let title = format!("Results ({})", self.results.len());
        
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
    
    /// Render file preview
    fn render_file_preview(&self, f: &mut Frame, area: Rect) {
        let title = if let Some(selected) = self.list_state.selected() {
            if let Some(result) = self.results.get(selected) {
                format!("Context: {}", result.file_path.display())
            } else {
                "Context".to_string()
            }
        } else {
            "Context".to_string()
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
    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        let help_text = "Type to search • ↑↓ Navigate • Ctrl-F/B Page • Enter Open • Esc Quit";
        let status_text = format!("{} | {}", self.status_message, help_text);
        
        let paragraph = Paragraph::new(status_text)
            .style(Style::default().bg(colors::PRIMARY).fg(colors::BACKGROUND));
        
        f.render_widget(paragraph, area);
    }
    
    /// Run the live search browser
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

/// Run the content search tool
pub fn run(
    pattern: Option<String>,
    path: PathBuf,
    file_type: Option<String>,
    ignore_case: bool,
) -> io::Result<()> {
    let mut browser = LiveSearchBrowser::new(pattern, path, file_type, ignore_case)?;
    browser.run()
}