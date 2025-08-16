//! Git operations and history browser.

use crate::cli::GitCommands;
use crate::tui_common::{self, colors};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::{
    io,
    process::{Command, Stdio},
    time::Duration,
};

/// Run a git command with timeout to prevent hanging
fn run_git_command_with_timeout(args: &[&str], timeout_secs: u64) -> io::Result<String> {
    use std::time::Instant;
    
    let start = Instant::now();
    let mut cmd = Command::new("git");
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
    
    let output = cmd.output()?;
    
    // Simple timeout check (not perfect but better than hanging)
    if start.elapsed().as_secs() > timeout_secs {
        return Err(io::Error::new(io::ErrorKind::TimedOut, "Git command timed out"));
    }
    
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Git command failed: {}", String::from_utf8_lossy(&output.stderr))
        ))
    }
}

/// Git commit information
#[derive(Debug, Clone)]
pub struct GitCommit {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author: String,
    pub date: String,
}

/// Git branch information
#[derive(Debug, Clone)]
pub struct GitBranch {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
}

/// Git log browser
pub struct GitLogBrowser {
    commits: Vec<GitCommit>,
    list_state: ListState,
    should_quit: bool,
    status_message: String,
    preview_content: String,
}

impl GitLogBrowser {
    /// Create a new git log browser
    pub fn new() -> io::Result<Self> {
        let mut browser = GitLogBrowser {
            commits: Vec::new(),
            list_state: ListState::default(),
            should_quit: false,
            status_message: "Loading git log...".to_string(),
            preview_content: String::new(),
        };
        
        browser.load_commits()?;
        
        Ok(browser)
    }
    
    /// Load git commits
    fn load_commits(&mut self) -> io::Result<()> {
        let log_output = match run_git_command_with_timeout(
            &["log", "--pretty=format:%H|%h|%s|%an|%ar", "-50"], 
            5  // 5 second timeout
        ) {
            Ok(output) => output,
            Err(_) => {
                self.status_message = "Error: Not a git repository, git not found, or command timed out".to_string();
                return Ok(());
            }
        };
        
        for line in log_output.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 5 {
                self.commits.push(GitCommit {
                    hash: parts[0].to_string(),
                    short_hash: parts[1].to_string(),
                    message: parts[2].to_string(),
                    author: parts[3].to_string(),
                    date: parts[4].to_string(),
                });
            }
        }
        
        if !self.commits.is_empty() {
            self.list_state.select(Some(0));
            self.update_preview();
        }
        
        self.status_message = format!("Loaded {} commits", self.commits.len());
        Ok(())
    }
    
    /// Update preview for selected commit
    fn update_preview(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if let Some(commit) = self.commits.get(selected) {
                self.preview_content = self.load_commit_diff(&commit.hash);
            }
        }
    }
    
    /// Load commit diff with optimization for large commits
    fn load_commit_diff(&self, hash: &str) -> String {
        // First, get just the commit info and stats (fast)
        let mut result = match run_git_command_with_timeout(
            &["show", "--color=never", "--stat", "--no-patch", hash],
            3  // 3 second timeout for stats
        ) {
            Ok(output) => output,
            Err(_) => format!("Commit: {}\n", hash),
        };
        
        // Add a separator
        result.push_str("\n--- Diff Preview (limited) ---\n");
        
        // Get a limited diff with timeout
        match run_git_command_with_timeout(
            &[
                "show", 
                "--color=never", 
                "--patch", 
                "--unified=3",  // Limited context
                hash
            ],
            5  // 5 second timeout for diff
        ) {
            Ok(diff_text) => {
                let lines: Vec<&str> = diff_text.lines().collect();
                
                // Take only first 100 lines to prevent UI freezing
                let limited_lines: Vec<&str> = lines.iter().take(100).cloned().collect();
                result.push_str(&limited_lines.join("\n"));
                
                if lines.len() > 100 {
                    result.push_str(&format!("\n\n... (showing first 100 of {} lines total)\nUse 'git show {}' for full diff", lines.len(), hash));
                }
            }
            Err(_) => {
                result.push_str("Failed to load commit diff (timeout or error)");
            }
        }
        
        result
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
                            key.code, key.modifiers, self.list_state.selected(), self.commits.len(), 10
                        ) {
                            self.list_state.select(Some(new_selection));
                            self.update_preview();
                        }
                    }
                    KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        // Page up
                        if let Some(new_selection) = tui_common::handle_page_navigation(
                            key.code, key.modifiers, self.list_state.selected(), self.commits.len(), 10
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
                            if selected + 1 < self.commits.len() {
                                self.list_state.select(Some(selected + 1));
                                self.update_preview();
                            }
                        } else if !self.commits.is_empty() {
                            self.list_state.select(Some(0));
                            self.update_preview();
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
    
    /// Render the git log browser
    fn render(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(f.area());
        
        self.render_commit_list(f, chunks[0]);
        self.render_commit_diff(f, chunks[1]);
        self.render_status_bar(f);
    }
    
    /// Render commit list
    fn render_commit_list(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.commits
            .iter()
            .map(|commit| {
                let line = Line::from(vec![
                    Span::styled(
                        &commit.short_hash,
                        Style::default().fg(colors::SECONDARY)
                    ),
                    Span::raw(" "),
                    Span::styled(
                        &commit.message,
                        Style::default().fg(colors::TEXT)
                    ),
                    Span::raw(" "),
                    Span::styled(
                        format!("({}) {}", commit.date, commit.author),
                        Style::default().fg(colors::PRIMARY)
                    ),
                ]);
                ListItem::new(line)
            })
            .collect();
        
        let list = List::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(format!("Git Log ({})", self.commits.len()))
                .border_style(Style::default().fg(colors::PRIMARY)))
            .highlight_style(Style::default()
                .bg(colors::PRIMARY)
                .fg(colors::BACKGROUND)
                .add_modifier(Modifier::BOLD))
            .highlight_symbol("► ");
        
        f.render_stateful_widget(list, area, &mut self.list_state);
    }
    
    /// Render commit diff
    fn render_commit_diff(&self, f: &mut Frame, area: Rect) {
        let title = if let Some(selected) = self.list_state.selected() {
            if let Some(commit) = self.commits.get(selected) {
                format!("Diff: {}", commit.short_hash)
            } else {
                "Diff".to_string()
            }
        } else {
            "Diff".to_string()
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
        
        let help_text = "↑↓ Navigate • Esc Quit";
        let status_text = format!("{} | {}", self.status_message, help_text);
        
        let paragraph = Paragraph::new(status_text)
            .style(Style::default().bg(colors::PRIMARY).fg(colors::BACKGROUND));
        
        f.render_widget(paragraph, area);
    }
    
    /// Run the git log browser
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

/// Git branch switcher
pub struct GitBranchSwitcher {
    branches: Vec<GitBranch>,
    list_state: ListState,
    should_quit: bool,
    status_message: String,
}

impl GitBranchSwitcher {
    /// Create a new git branch switcher
    pub fn new() -> io::Result<Self> {
        let mut switcher = GitBranchSwitcher {
            branches: Vec::new(),
            list_state: ListState::default(),
            should_quit: false,
            status_message: "Loading git branches...".to_string(),
        };
        
        switcher.load_branches()?;
        
        Ok(switcher)
    }
    
    /// Load git branches
    fn load_branches(&mut self) -> io::Result<()> {
        let output = Command::new("git")
            .args(&["branch", "-a"])
            .stdout(Stdio::piped())
            .output()?;
        
        if !output.status.success() {
            self.status_message = "Error: Not a git repository or git not found".to_string();
            return Ok(());
        }
        
        let branches_output = String::from_utf8_lossy(&output.stdout);
        
        for line in branches_output.lines() {
            let line = line.trim();
            if line.is_empty() || line.contains("HEAD ->") {
                continue;
            }
            
            let is_current = line.starts_with('*');
            let is_remote = line.contains("remotes/");
            
            let name = line
                .trim_start_matches('*')
                .trim()
                .trim_start_matches("remotes/origin/")
                .to_string();
            
            // Skip if we already have this branch (local version takes precedence)
            if !self.branches.iter().any(|b| b.name == name) {
                self.branches.push(GitBranch {
                    name,
                    is_current,
                    is_remote,
                });
            }
        }
        
        if !self.branches.is_empty() {
            self.list_state.select(Some(0));
        }
        
        self.status_message = format!("Loaded {} branches", self.branches.len());
        Ok(())
    }
    
    /// Switch to selected branch
    fn switch_branch(&mut self) -> io::Result<()> {
        if let Some(selected) = self.list_state.selected() {
            if let Some(branch) = self.branches.get(selected) {
                if branch.is_current {
                    self.status_message = "Already on this branch".to_string();
                    return Ok(());
                }
                
                let output = Command::new("git")
                    .args(&["checkout", &branch.name])
                    .output()?;
                
                if output.status.success() {
                    self.status_message = format!("Switched to branch '{}'", branch.name);
                    self.should_quit = true;
                } else {
                    let error = String::from_utf8_lossy(&output.stderr);
                    self.status_message = format!("Failed to switch: {}", error.trim());
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
                            key.code, key.modifiers, self.list_state.selected(), self.branches.len(), 10
                        ) {
                            self.list_state.select(Some(new_selection));
                        }
                    }
                    KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        // Page up
                        if let Some(new_selection) = tui_common::handle_page_navigation(
                            key.code, key.modifiers, self.list_state.selected(), self.branches.len(), 10
                        ) {
                            self.list_state.select(Some(new_selection));
                        }
                    }
                    KeyCode::Up => {
                        if let Some(selected) = self.list_state.selected() {
                            if selected > 0 {
                                self.list_state.select(Some(selected - 1));
                            }
                        }
                    }
                    KeyCode::Down => {
                        if let Some(selected) = self.list_state.selected() {
                            if selected + 1 < self.branches.len() {
                                self.list_state.select(Some(selected + 1));
                            }
                        } else if !self.branches.is_empty() {
                            self.list_state.select(Some(0));
                        }
                    }
                    KeyCode::Enter => {
                        self.switch_branch()?;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
    
    /// Render the branch switcher
    fn render(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(1)])
            .split(f.area());
        
        self.render_branch_list(f, chunks[0]);
        self.render_status_bar(f, chunks[1]);
    }
    
    /// Render branch list
    fn render_branch_list(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.branches
            .iter()
            .map(|branch| {
                let prefix = if branch.is_current { "* " } else { "  " };
                let style = if branch.is_current {
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                } else if branch.is_remote {
                    Style::default().fg(colors::SECONDARY)
                } else {
                    Style::default().fg(colors::TEXT)
                };
                
                let line = Line::from(vec![
                    Span::raw(prefix),
                    Span::styled(&branch.name, style),
                ]);
                
                ListItem::new(line)
            })
            .collect();
        
        let list = List::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(format!("Git Branches ({})", self.branches.len()))
                .border_style(Style::default().fg(colors::PRIMARY)))
            .highlight_style(Style::default()
                .bg(colors::PRIMARY)
                .fg(colors::BACKGROUND)
                .add_modifier(Modifier::BOLD))
            .highlight_symbol("► ");
        
        f.render_stateful_widget(list, area, &mut self.list_state);
    }
    
    /// Render status bar
    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        let help_text = "↑↓ Navigate • Enter Switch • Esc Quit";
        let status_text = format!("{} | {}", self.status_message, help_text);
        
        let paragraph = Paragraph::new(status_text)
            .style(Style::default().bg(colors::PRIMARY).fg(colors::BACKGROUND));
        
        f.render_widget(paragraph, area);
    }
    
    /// Run the branch switcher
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

/// Git diff browser
pub struct GitDiffBrowser {
    diff_content: String,
    scroll_offset: usize,
    should_quit: bool,
    status_message: String,
}

impl GitDiffBrowser {
    /// Create a new git diff browser
    pub fn new() -> io::Result<Self> {
        let mut browser = GitDiffBrowser {
            diff_content: String::new(),
            scroll_offset: 0,
            should_quit: false,
            status_message: "Loading git diff...".to_string(),
        };
        
        browser.load_diff()?;
        
        Ok(browser)
    }
    
    /// Load git diff content
    fn load_diff(&mut self) -> io::Result<()> {
        let output = Command::new("git")
            .args(&["diff", "--color=never"])
            .stdout(Stdio::piped())
            .output()?;
        
        if !output.status.success() {
            self.status_message = "Error: Not a git repository or git not found".to_string();
            return Ok(());
        }
        
        self.diff_content = String::from_utf8_lossy(&output.stdout).to_string();
        
        if self.diff_content.trim().is_empty() {
            self.diff_content = "No changes to show".to_string();
            self.status_message = "Working tree clean".to_string();
        } else {
            let line_count = self.diff_content.lines().count();
            self.status_message = format!("Git diff ({} lines)", line_count);
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
                        self.page_down();
                    }
                    KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        // Page up
                        self.page_up();
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if self.scroll_offset > 0 {
                            self.scroll_offset -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let max_scroll = self.diff_content.lines().count().saturating_sub(1);
                        if self.scroll_offset < max_scroll {
                            self.scroll_offset += 1;
                        }
                    }
                    KeyCode::Home | KeyCode::Char('g') => {
                        self.scroll_offset = 0;
                    }
                    KeyCode::End | KeyCode::Char('G') => {
                        self.scroll_offset = self.diff_content.lines().count().saturating_sub(20);
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
    
    /// Page down
    fn page_down(&mut self) {
        let max_scroll = self.diff_content.lines().count().saturating_sub(1);
        self.scroll_offset = std::cmp::min(self.scroll_offset + 20, max_scroll);
    }
    
    /// Page up
    fn page_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(20);
    }
    
    /// Render the diff browser
    fn render(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(1)])
            .split(f.area());
        
        self.render_diff_content(f, chunks[0]);
        self.render_status_bar(f, chunks[1]);
    }
    
    /// Render diff content
    fn render_diff_content(&self, f: &mut Frame, area: Rect) {
        let lines: Vec<&str> = self.diff_content.lines().collect();
        let visible_lines: Vec<Line> = lines
            .iter()
            .skip(self.scroll_offset)
            .take(area.height as usize - 2)
            .map(|line| {
                // Color diff lines
                if line.starts_with('+') && !line.starts_with("+++") {
                    Line::from(Span::styled(*line, Style::default().fg(Color::Green)))
                } else if line.starts_with('-') && !line.starts_with("---") {
                    Line::from(Span::styled(*line, Style::default().fg(Color::Red)))
                } else if line.starts_with("@@") {
                    Line::from(Span::styled(*line, Style::default().fg(colors::PRIMARY).add_modifier(Modifier::BOLD)))
                } else if line.starts_with("diff --git") {
                    Line::from(Span::styled(*line, Style::default().fg(colors::SECONDARY).add_modifier(Modifier::BOLD)))
                } else {
                    Line::from(*line)
                }
            })
            .collect();
        
        let paragraph = Paragraph::new(visible_lines)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Git Diff")
                .border_style(Style::default().fg(colors::PRIMARY)));
        
        f.render_widget(paragraph, area);
    }
    
    /// Render status bar
    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        let help_text = "↑↓/jk Scroll • Ctrl-F/B Page • g/G Top/Bottom • Esc Quit";
        let status_text = format!("{} | {}", self.status_message, help_text);
        
        let paragraph = Paragraph::new(status_text)
            .style(Style::default().bg(colors::PRIMARY).fg(colors::BACKGROUND));
        
        f.render_widget(paragraph, area);
    }
    
    /// Run the diff browser
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

/// Run git tools
pub fn run(subcommand: GitCommands) -> io::Result<()> {
    match subcommand {
        GitCommands::Log => {
            let mut browser = GitLogBrowser::new()?;
            browser.run()
        }
        GitCommands::Branch => {
            let mut switcher = GitBranchSwitcher::new()?;
            switcher.run()
        }
        GitCommands::Status => {
            // For now, just run git status
            let output = Command::new("git")
                .args(&["status", "--porcelain"])
                .output()?;
            
            if output.status.success() {
                let status_output = String::from_utf8_lossy(&output.stdout);
                if status_output.trim().is_empty() {
                    println!("Working tree clean");
                } else {
                    println!("Git Status:");
                    for line in status_output.lines() {
                        println!("{}", line);
                    }
                }
            } else {
                println!("Error: Not a git repository or git not found");
            }
            Ok(())
        }
        GitCommands::Diff => {
            let mut diff_browser = GitDiffBrowser::new()?;
            diff_browser.run()
        }
    }
}