//! Environment variable browser.

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
    io,
    time::Duration,
};

pub struct EnvBrowser {
    env_vars: Vec<(String, String)>,
    filtered_vars: Vec<(String, String)>,
    list_state: ListState,
    search_query: String,
    should_quit: bool,
    status_message: String,
}

impl EnvBrowser {
    /// Create a new environment browser instance
    pub fn new() -> io::Result<Self> {
        let mut browser = EnvBrowser {
            env_vars: Vec::new(),
            filtered_vars: Vec::new(),
            list_state: ListState::default(),
            search_query: String::new(),
            should_quit: false,
            status_message: "Loading environment variables...".to_string(),
        };
        
        browser.load_env_vars();
        browser.update_filter();
        
        Ok(browser)
    }
    
    /// Load all environment variables
    fn load_env_vars(&mut self) {
        self.env_vars = env::vars().collect();
        self.env_vars.sort_by(|a, b| a.0.cmp(&b.0));
        self.status_message = format!("Found {} environment variables", self.env_vars.len());
    }
    
    /// Update filtered variables based on search query
    fn update_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_vars = self.env_vars.clone();
        } else {
            let query = self.search_query.to_lowercase();
            self.filtered_vars = self.env_vars
                .iter()
                .filter(|(key, value)| {
                    key.to_lowercase().contains(&query) ||
                    value.to_lowercase().contains(&query)
                })
                .cloned()
                .collect();
        }
        
        // Reset selection
        if !self.filtered_vars.is_empty() {
            self.list_state.select(Some(0));
        } else {
            self.list_state.select(None);
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
                            key.code, key.modifiers, self.list_state.selected(), self.filtered_vars.len(), 10
                        ) {
                            self.list_state.select(Some(new_selection));
                        }
                    }
                    KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        // Page up
                        if let Some(new_selection) = tui_common::handle_page_navigation(
                            key.code, key.modifiers, self.list_state.selected(), self.filtered_vars.len(), 10
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
                            if selected + 1 < self.filtered_vars.len() {
                                self.list_state.select(Some(selected + 1));
                            }
                        } else if !self.filtered_vars.is_empty() {
                            self.list_state.select(Some(0));
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
    
    /// Render the environment browser interface
    fn render(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(f.area());
        
        // Left panel - variable list
        self.render_var_list(f, chunks[0]);
        
        // Right panel - value preview
        self.render_value_preview(f, chunks[1]);
        
        // Status bar
        self.render_status_bar(f);
    }
    
    /// Render the variable list panel
    fn render_var_list(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.filtered_vars
            .iter()
            .map(|(key, _)| {
                ListItem::new(Line::from(key.clone()))
            })
            .collect();
        
        let title = if self.search_query.is_empty() {
            format!("Environment Variables ({})", self.filtered_vars.len())
        } else {
            format!("Environment Variables ({}) - Filter: '{}'", self.filtered_vars.len(), self.search_query)
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
    
    /// Render the value preview panel
    fn render_value_preview(&self, f: &mut Frame, area: Rect) {
        let (title, content) = if let Some(selected) = self.list_state.selected() {
            if let Some((key, value)) = self.filtered_vars.get(selected) {
                (format!("Value: {}", key), value.clone())
            } else {
                ("Value".to_string(), String::new())
            }
        } else {
            ("Value".to_string(), String::new())
        };
        
        let paragraph = Paragraph::new(content)
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
        
        let help_text = "Type to filter • ↑↓ Navigate • Esc Quit";
        let status_text = if !self.status_message.is_empty() {
            format!("{} | {}", self.status_message, help_text)
        } else {
            help_text.to_string()
        };
        
        let paragraph = Paragraph::new(status_text)
            .style(Style::default().bg(colors::PRIMARY).fg(colors::BACKGROUND));
        
        f.render_widget(paragraph, area);
    }
    
    /// Run the environment browser application
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

/// Run the environment browser tool
pub fn run() -> io::Result<()> {
    let mut browser = EnvBrowser::new()?;
    browser.run()
}