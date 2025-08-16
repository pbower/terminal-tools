//! Process killer tool with interactive selection.

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
    fmt,
    io,
    process::{Command, Stdio},
    time::Duration,
};

#[derive(Debug, Clone)]
pub struct Process {
    pub pid: u32,
    pub name: String,
    pub cpu: f32,
    pub memory: f32,
    pub command: String,
}

impl fmt::Display for Process {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:>8} {:>6.1}% {:>6.1}% {}",
            self.pid, self.cpu, self.memory, self.name
        )
    }
}

pub struct ProcessKiller {
    processes: Vec<Process>,
    filtered_processes: Vec<Process>,
    list_state: ListState,
    search_query: String,
    should_quit: bool,
    status_message: String,
    confirmation_mode: bool,
    selected_process: Option<Process>,
}

impl ProcessKiller {
    /// Create a new process killer instance
    pub fn new(filter: Option<String>) -> io::Result<Self> {
        let mut killer = ProcessKiller {
            processes: Vec::new(),
            filtered_processes: Vec::new(),
            list_state: ListState::default(),
            search_query: filter.unwrap_or_default(),
            should_quit: false,
            status_message: "Loading processes...".to_string(),
            confirmation_mode: false,
            selected_process: None,
        };
        
        killer.load_processes()?;
        killer.update_filter();
        
        Ok(killer)
    }
    
    /// Load all running processes
    fn load_processes(&mut self) -> io::Result<()> {
        self.processes.clear();
        
        // Use ps command to get process information
        let output = Command::new("ps")
            .args(&["aux", "--no-headers"])
            .stdout(Stdio::piped())
            .output()?;
        
        if !output.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to execute ps command"
            ));
        }
        
        let ps_output = String::from_utf8_lossy(&output.stdout);
        
        for line in ps_output.lines() {
            if let Some(process) = self.parse_ps_line(line) {
                // Skip kernel threads and very short-lived processes
                if !process.name.starts_with('[') && process.pid > 1 {
                    self.processes.push(process);
                }
            }
        }
        
        // Sort by CPU usage (descending)
        self.processes.sort_by(|a, b| b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal));
        
        self.status_message = format!("Found {} processes", self.processes.len());
        Ok(())
    }
    
    /// Parse a line from ps aux output
    fn parse_ps_line(&self, line: &str) -> Option<Process> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        
        if parts.len() < 11 {
            return None;
        }
        
        let pid: u32 = parts[1].parse().ok()?;
        let cpu: f32 = parts[2].parse().ok()?;
        let memory: f32 = parts[3].parse().ok()?;
        
        // Command is everything from column 11 onwards
        let command = parts[10..].join(" ");
        
        // Extract process name (first part of command, without path)
        let name = command
            .split_whitespace()
            .next()
            .unwrap_or(&command)
            .split('/')
            .last()
            .unwrap_or(&command)
            .to_string();
        
        Some(Process {
            pid,
            name,
            cpu,
            memory,
            command,
        })
    }
    
    /// Update filtered processes based on search query
    fn update_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_processes = self.processes.clone();
        } else {
            let query = self.search_query.to_lowercase();
            self.filtered_processes = self.processes
                .iter()
                .filter(|process| {
                    process.name.to_lowercase().contains(&query) ||
                    process.command.to_lowercase().contains(&query) ||
                    process.pid.to_string().contains(&query)
                })
                .cloned()
                .collect();
        }
        
        // Reset selection
        if !self.filtered_processes.is_empty() {
            self.list_state.select(Some(0));
        } else {
            self.list_state.select(None);
        }
    }
    
    /// Handle keyboard input
    fn handle_input(&mut self) -> io::Result<()> {
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if self.confirmation_mode {
                    self.handle_confirmation_input(key.code)?;
                } else {
                    self.handle_normal_input(key.code, key.modifiers)?;
                }
            }
        }
        Ok(())
    }
    
    /// Handle input in normal mode
    fn handle_normal_input(&mut self, key_code: KeyCode, modifiers: KeyModifiers) -> io::Result<()> {
        match key_code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Char('f') if modifiers.contains(KeyModifiers::CONTROL) => {
                // Page down
                if let Some(new_selection) = tui_common::handle_page_navigation(
                    key_code, modifiers, self.list_state.selected(), self.filtered_processes.len(), 10
                ) {
                    self.list_state.select(Some(new_selection));
                }
            }
            KeyCode::Char('b') if modifiers.contains(KeyModifiers::CONTROL) => {
                // Page up
                if let Some(new_selection) = tui_common::handle_page_navigation(
                    key_code, modifiers, self.list_state.selected(), self.filtered_processes.len(), 10
                ) {
                    self.list_state.select(Some(new_selection));
                }
            }
            KeyCode::Char('r') => {
                self.load_processes()?;
                self.update_filter();
                self.status_message = "Processes refreshed".to_string();
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
                    if selected + 1 < self.filtered_processes.len() {
                        self.list_state.select(Some(selected + 1));
                    }
                } else if !self.filtered_processes.is_empty() {
                    self.list_state.select(Some(0));
                }
            }
            KeyCode::Enter => {
                if let Some(selected) = self.list_state.selected() {
                    if let Some(process) = self.filtered_processes.get(selected) {
                        self.selected_process = Some(process.clone());
                        self.confirmation_mode = true;
                        self.status_message = format!("Kill process {} ({})?", process.name, process.pid);
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
        Ok(())
    }
    
    /// Handle input in confirmation mode
    fn handle_confirmation_input(&mut self, key_code: KeyCode) -> io::Result<()> {
        match key_code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                if let Some(process) = &self.selected_process {
                    self.kill_process(process.pid)?;
                }
                self.confirmation_mode = false;
                self.selected_process = None;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.confirmation_mode = false;
                self.selected_process = None;
                self.status_message = "Kill cancelled".to_string();
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Kill a process by PID
    fn kill_process(&mut self, pid: u32) -> io::Result<()> {
        let result = Command::new("kill")
            .arg(pid.to_string())
            .output();
        
        match result {
            Ok(output) => {
                if output.status.success() {
                    self.status_message = format!("Process {} killed successfully", pid);
                    // Refresh process list
                    self.load_processes()?;
                    self.update_filter();
                } else {
                    let error = String::from_utf8_lossy(&output.stderr);
                    self.status_message = format!("Failed to kill process {}: {}", pid, error.trim());
                }
            }
            Err(e) => {
                self.status_message = format!("Error killing process {}: {}", pid, e);
            }
        }
        
        Ok(())
    }
    
    /// Render the process killer interface
    fn render(&mut self, f: &mut Frame) {
        if self.confirmation_mode {
            self.render_confirmation(f);
        } else {
            self.render_normal(f);
        }
    }
    
    /// Render normal mode
    fn render_normal(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(1)])
            .split(f.area());
        
        // Process list
        self.render_process_list(f, chunks[0]);
        
        // Status bar
        self.render_status_bar(f, chunks[1]);
    }
    
    /// Render confirmation dialog
    fn render_confirmation(&self, f: &mut Frame) {
        let area = f.area();
        
        // Create a centered popup
        let popup_area = Rect {
            x: area.width / 4,
            y: area.height / 3,
            width: area.width / 2,
            height: 7,
        };
        
        if let Some(process) = &self.selected_process {
            let text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("Kill process {} (PID {})?", process.name, process.pid),
                    Style::default().fg(colors::PRIMARY).add_modifier(Modifier::BOLD)
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!("Command: {}", process.command),
                    Style::default().fg(colors::SECONDARY)
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "[Y]es / [N]o",
                    Style::default().fg(colors::TEXT).add_modifier(Modifier::BOLD)
                )),
            ];
            
            let paragraph = Paragraph::new(text)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title("Confirm Kill")
                    .border_style(Style::default().fg(Color::Red)))
                .wrap(Wrap { trim: true });
            
            // Clear background
            f.render_widget(
                Block::default()
                    .style(Style::default().bg(Color::Black)),
                area
            );
            
            f.render_widget(paragraph, popup_area);
        }
    }
    
    /// Render the process list
    fn render_process_list(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.filtered_processes
            .iter()
            .map(|process| {
                let line = Line::from(vec![
                    Span::styled(
                        format!("{:>8}", process.pid),
                        Style::default().fg(colors::SECONDARY)
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format!("{:>6.1}%", process.cpu),
                        if process.cpu > 50.0 {
                            Style::default().fg(Color::Red)
                        } else if process.cpu > 10.0 {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default().fg(colors::TEXT)
                        }
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format!("{:>6.1}%", process.memory),
                        if process.memory > 50.0 {
                            Style::default().fg(Color::Red)
                        } else if process.memory > 10.0 {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default().fg(colors::TEXT)
                        }
                    ),
                    Span::raw("  "),
                    Span::styled(
                        process.name.clone(),
                        Style::default().fg(colors::PRIMARY).add_modifier(Modifier::BOLD)
                    ),
                ]);
                
                ListItem::new(line)
            })
            .collect();
        
        let title = if self.search_query.is_empty() {
            format!("Processes ({}) - Sorted by CPU", self.filtered_processes.len())
        } else {
            format!("Processes ({}) - Filter: '{}'", self.filtered_processes.len(), self.search_query)
        };
        
        let header = ListItem::new(Line::from(vec![
            Span::styled("     PID", Style::default().fg(colors::SECONDARY).add_modifier(Modifier::BOLD)),
            Span::styled("    CPU", Style::default().fg(colors::SECONDARY).add_modifier(Modifier::BOLD)),
            Span::styled("    MEM", Style::default().fg(colors::SECONDARY).add_modifier(Modifier::BOLD)),
            Span::styled("  NAME", Style::default().fg(colors::SECONDARY).add_modifier(Modifier::BOLD)),
        ]));
        
        let mut all_items = vec![header];
        all_items.extend(items);
        
        let list = List::new(all_items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(colors::PRIMARY)))
            .highlight_style(Style::default()
                .bg(colors::PRIMARY)
                .fg(colors::BACKGROUND)
                .add_modifier(Modifier::BOLD))
            .highlight_symbol("► ");
        
        // Adjust list state to account for header
        let mut adjusted_state = self.list_state.clone();
        if let Some(selected) = adjusted_state.selected() {
            adjusted_state.select(Some(selected + 1));
        }
        
        f.render_stateful_widget(list, area, &mut adjusted_state);
    }
    
    /// Render status bar
    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        let help_text = if self.confirmation_mode {
            "Y/Enter Confirm • N/Esc Cancel"
        } else {
            "Type to filter • ↑↓ Navigate • Enter Kill • R Refresh • Esc Quit"
        };
        
        let status_text = if !self.status_message.is_empty() {
            format!("{} | {}", self.status_message, help_text)
        } else {
            help_text.to_string()
        };
        
        let paragraph = Paragraph::new(status_text)
            .style(Style::default().bg(colors::PRIMARY).fg(colors::BACKGROUND));
        
        f.render_widget(paragraph, area);
    }
    
    /// Run the process killer application
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

/// Run the process killer tool
pub fn run(filter: Option<String>) -> io::Result<()> {
    let mut killer = ProcessKiller::new(filter)?;
    killer.run()
}