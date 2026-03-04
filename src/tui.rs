use crate::db::DirEntry;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::io::stdout;

pub struct TuiPicker {
    entries: Vec<DirEntry>,
    filtered: Vec<usize>,
    query: String,
    list_state: ListState,
    matcher: SkimMatcherV2,
}

impl TuiPicker {
    pub fn new(entries: Vec<DirEntry>, initial_query: Option<String>) -> Self {
        let query = initial_query.unwrap_or_default();
        let mut picker = Self {
            entries,
            filtered: Vec::new(),
            query,
            list_state: ListState::default(),
            matcher: SkimMatcherV2::default(),
        };
        picker.update_filter();
        if !picker.filtered.is_empty() {
            picker.list_state.select(Some(0));
        }
        picker
    }

    fn update_filter(&mut self) {
        if self.query.is_empty() {
            self.filtered = (0..self.entries.len()).collect();
        } else {
            let mut scored: Vec<(usize, i64)> = self
                .entries
                .iter()
                .enumerate()
                .filter_map(|(i, entry)| {
                    self.matcher
                        .fuzzy_match(&entry.path, &self.query)
                        .map(|score| (i, score))
                })
                .collect();
            scored.sort_by(|a, b| b.1.cmp(&a.1));
            self.filtered = scored.into_iter().map(|(i, _)| i).collect();
        }
        // Reset selection
        if self.filtered.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }

    fn move_selection(&mut self, delta: i32) {
        if self.filtered.is_empty() {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0) as i32;
        let next = (current + delta).clamp(0, self.filtered.len() as i32 - 1) as usize;
        self.list_state.select(Some(next));
    }

    fn selected_path(&self) -> Option<String> {
        self.list_state
            .selected()
            .and_then(|i| self.filtered.get(i))
            .map(|&idx| self.entries[idx].path.clone())
    }

    pub fn run(mut self) -> Result<Option<String>> {
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

        let result = loop {
            terminal.draw(|f| self.render(f))?;

            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Esc => break None,
                    KeyCode::Enter => break self.selected_path(),
                    KeyCode::Up | KeyCode::BackTab => self.move_selection(-1),
                    KeyCode::Down | KeyCode::Tab => self.move_selection(1),
                    KeyCode::Char('k') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        self.move_selection(-1)
                    }
                    KeyCode::Char('j') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        self.move_selection(1)
                    }
                    KeyCode::Backspace => {
                        self.query.pop();
                        self.update_filter();
                    }
                    KeyCode::Char(c) => {
                        self.query.push(c);
                        self.update_filter();
                    }
                    _ => {}
                }
            }
        };

        disable_raw_mode()?;
        stdout().execute(LeaveAlternateScreen)?;
        Ok(result)
    }

    fn render(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // search bar
                Constraint::Min(1),   // list
                Constraint::Length(1), // help bar
            ])
            .split(f.area());

        // Search bar
        let search = Paragraph::new(format!("> {}", self.query))
            .block(Block::default().borders(Borders::ALL).title(" trod "));
        f.render_widget(search, chunks[0]);

        // Directory list
        let items: Vec<ListItem> = self
            .filtered
            .iter()
            .map(|&idx| {
                let entry = &self.entries[idx];
                let path = shorten_path(&entry.path);
                let time = relative_time(entry.last_visited);
                let count = entry.visit_count;
                let width = chunks[1].width as usize;
                let right = format!("{:>10} {:>4}", time, count);
                let left_width = width.saturating_sub(right.len() + 2);
                let line = format!("  {:<left_width$}{}", path, right);
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::NONE))
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );
        f.render_stateful_widget(list, chunks[1], &mut self.list_state);

        // Help bar
        let help = Paragraph::new(" \u{2191}\u{2193} navigate  \u{23ce} select  esc quit")
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(help, chunks[2]);
    }
}

fn shorten_path(path: &str) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Some(rest) = path.strip_prefix(home.to_str().unwrap_or("")) {
            return format!("~{}", rest);
        }
    }
    path.to_string()
}

fn relative_time(dt: chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let diff = now.signed_duration_since(dt);

    if diff.num_seconds() < 60 {
        "just now".to_string()
    } else if diff.num_minutes() < 60 {
        format!("{}m ago", diff.num_minutes())
    } else if diff.num_hours() < 24 {
        format!("{}h ago", diff.num_hours())
    } else {
        format!("{}d ago", diff.num_days())
    }
}
