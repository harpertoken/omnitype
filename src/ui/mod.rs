use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use omnitype::analyzer::{AnalysisResult, Analyzer, Diagnostic};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs},
    Frame, Terminal,
};
use std::{
    fs, io,
    path::{Path, PathBuf},
    time::Duration,
};

pub struct App {
    pub should_quit: bool,
    pub selected_tab: usize,
    pub tabs: Vec<&'static str>,
    pub logs: Vec<String>,
    current_dir: PathBuf,
    files: Vec<PathBuf>,
    file_list_state: ListState,
    file_content: Option<String>,
    analysis_result: Option<AnalysisResult>,
    errors: Vec<Diagnostic>,
    errors_state: ListState,
    editor_scroll: u16,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        let files = list_files(&current_dir).unwrap_or_default();

        let mut file_list_state = ListState::default();

        if !files.is_empty() {
            file_list_state.select(Some(0));
        }

        Self {
            should_quit: false,
            selected_tab: 0,
            tabs: vec!["Files", "Types", "Errors", "Logs", "Editor"],
            logs: vec!["Application started".to_string(), "Loading workspace...".to_string()],
            current_dir,
            files,
            file_list_state,
            file_content: None,
            analysis_result: None,
            errors: Vec::new(),
            errors_state: ListState::default(),
            editor_scroll: 0,
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        // Setup terminal
        enable_raw_mode()?;

        let mut stdout = io::stdout();

        execute!(stdout, EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stdout);

        let mut terminal = Terminal::new(backend)?;

        // Main event loop
        while !self.should_quit {
            self.draw(&mut terminal)?;

            self.handle_events()?;
        }

        // Cleanup terminal
        disable_raw_mode()?;

        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

        terminal.show_cursor()?;

        Ok(())
    }

    fn draw(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
        terminal.draw(|f| {
            let size = f.size();

            // Main layout with 3 vertical chunks: header, content, status
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(3), // Header with tabs
                        Constraint::Min(3),    // Main content
                        Constraint::Length(1), // Status bar
                    ]
                    .as_ref(),
                )
                .split(size);

            // Create tab titles with first letter highlighted
            let tab_titles = self
                .tabs
                .iter()
                .map(|t| {
                    let (first, rest) = t.split_at(1);

                    Line::from(vec![
                        Span::styled(
                            first,
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(rest, Style::default().fg(Color::White)),
                    ])
                })
                .collect::<Vec<_>>();

            // Render tabs
            let tabs = Tabs::new(tab_titles)
                .block(Block::default().borders(Borders::ALL).title("OmniType"))
                .select(self.selected_tab)
                .style(Style::default().fg(Color::White))
                .highlight_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
                .divider("│");

            f.render_widget(tabs, chunks[0]);

            // Draw the appropriate tab content
            match self.selected_tab {
                0 => self.draw_files_tab(f, chunks[1]),
                1 => self.draw_editor_tab(f, chunks[1]),
                2 => self.draw_types_tab(f, chunks[1]),
                3 => self.draw_errors_tab(f, chunks[1]),
                4 => self.draw_logs_tab(f, chunks[1]),
                _ => {},
            }

            // Status bar with context-sensitive help
            let status = match self.selected_tab {
                0 => "↑/↓: Navigate | Enter: Open | R: Refresh | Q: Quit",
                1 => "←/→: Switch tabs | Q: Quit",
                2 => "←/→: Switch tabs | Q: Quit",
                3 => "↑/↓: Select | Enter: Open file | Q: Quit",
                4 => "←/→: Switch tabs | Q: Quit",
                _ => "←/→: Switch tabs | Q: Quit",
            };

            let status_bar = Paragraph::new(Line::from(Span::styled(
                status,
                Style::default().fg(Color::White).bg(Color::DarkGray),
            )));

            f.render_widget(status_bar, chunks[2]);
        })?;

        Ok(())
    }

    fn draw_files_tab(&mut self, f: &mut Frame<'_>, area: Rect) {
        // Split the area into list and status bar
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);

        // Create list items with proper icons and styling
        let items: Vec<ListItem> = self
            .files
            .iter()
            .map(|path| {
                let is_dir = path.is_dir();

                let display_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("<unknown>");

                // Use different icons for directories and files
                let (icon, style) = if is_dir {
                    ("📁 ", Style::default().fg(Color::Blue))
                } else {
                    ("📄 ", Style::default().fg(Color::White))
                };

                // Create styled spans for the list item
                let line =
                    Line::from(vec![Span::styled(icon, style), Span::styled(display_name, style)]);

                ListItem::new(line)
            })
            .collect();

        // Create the list widget
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" {} ", self.current_dir.display()))
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::Rgb(70, 70, 90)),
            )
            .highlight_symbol("> ");

        // Render the list with persistent state
        f.render_stateful_widget(list, chunks[0], &mut self.file_list_state);

        // Status bar with current path and help
        let status = format!("{} items", self.files.len());

        let help = "↑/↓: Navigate | Enter: Open | R: Refresh | Q: Quit";

        let status_bar = Line::from(vec![
            Span::styled(status, Style::default().fg(Color::Yellow)),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled(help, Style::default().fg(Color::Gray)),
        ]);

        let status_bar =
            Paragraph::new(status_bar).style(Style::default().bg(Color::Rgb(30, 30, 40)));

        f.render_widget(status_bar, chunks[1]);
    }

    fn draw_editor_tab(&self, f: &mut Frame<'_>, area: Rect) {
        // Split into line numbers and content areas
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(6), // Line numbers
                Constraint::Min(10),   // Content
            ])
            .split(area);

        // Get file content or default message
        let content = self.file_content.as_deref().unwrap_or("No file selected");

        // Create line numbers
        let line_count = content.lines().count().max(1);

        let line_numbers = (1..=line_count)
            .map(|i| format!("{:4} ", i))
            .collect::<Vec<_>>()
            .join("\n");

        // Draw line numbers
        let line_numbers = Paragraph::new(line_numbers)
            .style(Style::default().fg(Color::DarkGray))
            .block(
                Block::default()
                    .borders(Borders::RIGHT)
                    .style(Style::default().fg(Color::DarkGray)),
            );

        f.render_widget(line_numbers, chunks[0]);

        // Draw content
        let content_block = Block::default()
            .borders(Borders::ALL)
            .title("Editor")
            .style(Style::default().bg(Color::Rgb(20, 20, 25)));

        let paragraph = Paragraph::new(content)
            .block(content_block)
            .wrap(ratatui::widgets::Wrap { trim: true })
            .style(Style::default().fg(Color::White))
            .scroll((self.editor_scroll, 0));

        f.render_widget(paragraph, chunks[1]);
    }

    fn draw_types_tab(&self, f: &mut Frame<'_>, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Type Information");

        let content = if let Some(res) = &self.analysis_result {
            let mut lines = vec![
                Line::from(Span::styled(
                    "Analysis Result",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(format!("Path: {}", res.path)),
                Line::from(format!("Functions: {}", res.function_count)),
                Line::from(format!("Classes: {}", res.class_count)),
                Line::from(""),
                Line::from(Span::styled(
                    "Diagnostics",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
            ];

            if res.diagnostics.is_empty() {
                lines.push(Line::from("(none)"));
            } else {
                for d in &res.diagnostics {
                    lines.push(Line::from(format!(
                        "{}:{}:{}: {} {}",
                        d.path,
                        d.line + 1,
                        d.column + 1,
                        d.severity,
                        d.message
                    )));
                }
            }

            Text::from(lines)
        } else {
            Text::from(
                "No analysis yet. Open a .py file (Enter) or press 'a' on a selection in Files \
                 tab.",
            )
        };

        let paragraph = Paragraph::new(content).block(block);

        f.render_widget(paragraph, area);
    }

    fn draw_errors_tab(&mut self, f: &mut Frame<'_>, area: Rect) {
        let block = Block::default().borders(Borders::ALL).title("Errors");

        let items: Vec<ListItem> = if self.errors.is_empty() {
            vec![ListItem::new("No diagnostics")]
        } else {
            self.errors
                .iter()
                .map(|d| {
                    let text = format!(
                        "{}:{}:{}: {} {}",
                        d.path,
                        d.line + 1,
                        d.column + 1,
                        d.severity,
                        d.message
                    );

                    ListItem::new(text)
                })
                .collect()
        };

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::Rgb(70, 70, 90)),
            )
            .highlight_symbol("> ");

        f.render_stateful_widget(list, area, &mut self.errors_state);
    }

    fn draw_logs_tab(&self, f: &mut Frame<'_>, area: Rect) {
        let items: Vec<ListItem> = self
            .logs
            .iter()
            .map(|log| ListItem::new(log.as_str()))
            .collect();

        let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Logs"));

        f.render_widget(list, area);
    }

    fn handle_events(&mut self) -> io::Result<bool> {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        // Quit application
                        KeyCode::Char('q') => return Ok(true),

                        // Refresh file list
                        KeyCode::Char('r') => match self.refresh_files() {
                            Ok(_) => self.logs.push("File list refreshed".to_string()),
                            Err(e) => self.logs.push(format!("Failed to refresh files: {}", e)),
                        },

                        // Navigation between tabs
                        KeyCode::Right | KeyCode::Char('l') => {
                            self.selected_tab = (self.selected_tab + 1) % self.tabs.len();
                        },
                        KeyCode::Left | KeyCode::Char('h') => {
                            self.selected_tab = if self.selected_tab > 0 {
                                self.selected_tab - 1
                            } else {
                                self.tabs.len() - 1
                            };
                        },

                        // File navigation (Files tab only)
                        KeyCode::Down | KeyCode::Char('j') if self.selected_tab == 0 => {
                            if let Some(selected) = self.file_list_state.selected() {
                                if selected < self.files.len().saturating_sub(1) {
                                    self.file_list_state.select(Some(selected + 1));
                                }
                            } else if !self.files.is_empty() {
                                self.file_list_state.select(Some(0));
                            }
                        },
                        KeyCode::Up | KeyCode::Char('k') if self.selected_tab == 0 => {
                            if let Some(selected) = self.file_list_state.selected() {
                                if selected > 0 {
                                    self.file_list_state.select(Some(selected - 1));
                                }
                            } else if !self.files.is_empty() {
                                self.file_list_state.select(Some(0));
                            }
                        },
                        KeyCode::Home if self.selected_tab == 0 => {
                            if !self.files.is_empty() {
                                self.file_list_state.select(Some(0));
                            }
                        },
                        KeyCode::End if self.selected_tab == 0 => {
                            if !self.files.is_empty() {
                                self.file_list_state.select(Some(self.files.len() - 1));
                            }
                        },
                        KeyCode::PageUp if self.selected_tab == 0 => {
                            if let Some(selected) = self.file_list_state.selected() {
                                let page_size = 10; // Number of items to jump
                                let new_selection = selected.saturating_sub(page_size);

                                self.file_list_state.select(Some(new_selection));
                            }
                        },
                        KeyCode::PageDown if self.selected_tab == 0 => {
                            if let Some(selected) = self.file_list_state.selected() {
                                let page_size = 10; // Number of items to jump
                                let new_selection =
                                    (selected + page_size).min(self.files.len().saturating_sub(1));

                                self.file_list_state.select(Some(new_selection));
                            }
                        },

                        // File operations (Files tab)
                        KeyCode::Enter | KeyCode::Char(' ') if self.selected_tab == 0 => {
                            if let Some(selected) = self.file_list_state.selected() {
                                if let Some(path) = self.files.get(selected) {
                                    let path_buf = path.clone();

                                    if path_buf.is_file() {
                                        match self.open_file(&path_buf) {
                                            Ok(_) => {
                                                self.selected_tab = 4; // Switch to editor tab
                                                self.logs.push(format!(
                                                    "Opened file: {}",
                                                    path_buf.display()
                                                ));

                                                // If it's a Python file, analyze it immediately
                                                if path_buf.extension().and_then(|e| e.to_str())
                                                    == Some("py")
                                                {
                                                    match self.run_analysis(&path_buf) {
                                                        Ok(_) => self
                                                            .logs
                                                            .push("Analysis complete".to_string()),
                                                        Err(e) => self.logs.push(format!(
                                                            "Analysis failed: {}",
                                                            e
                                                        )),
                                                    }
                                                }
                                            },
                                            Err(e) => {
                                                self.logs
                                                    .push(format!("Failed to open file: {}", e));
                                            },
                                        }
                                    } else if path_buf.is_dir() {
                                        self.current_dir = path_buf.clone();

                                        if let Err(e) = self.refresh_files() {
                                            self.logs
                                                .push(format!("Failed to enter directory: {}", e));
                                        } else {
                                            self.file_list_state.select(Some(0));

                                            self.logs.push(format!(
                                                "Entered directory: {}",
                                                path_buf.display()
                                            ));
                                        }
                                    }
                                }
                            }
                        },
                        // Errors tab navigation and open-on-enter
                        KeyCode::Up if self.selected_tab == 3 => {
                            if let Some(i) = self.errors_state.selected() {
                                if i > 0 {
                                    self.errors_state.select(Some(i - 1));
                                }
                            } else if !self.errors.is_empty() {
                                self.errors_state.select(Some(0));
                            }
                        },
                        KeyCode::Down if self.selected_tab == 3 => {
                            if let Some(i) = self.errors_state.selected() {
                                if i + 1 < self.errors.len() {
                                    self.errors_state.select(Some(i + 1));
                                }
                            } else if !self.errors.is_empty() {
                                self.errors_state.select(Some(0));
                            }
                        },
                        KeyCode::Enter if self.selected_tab == 3 => {
                            if let Some(i) = self.errors_state.selected() {
                                if let Some(d) = self.errors.get(i).cloned() {
                                    let p = PathBuf::from(&d.path);

                                    if p.is_file() {
                                        if let Err(e) = self.open_file(&p) {
                                            self.logs.push(format!(
                                                "Failed to open file from error: {}",
                                                e
                                            ));
                                        } else {
                                            // Jump editor to the diagnostic line
                                            self.editor_scroll = d.line as u16;

                                            self.selected_tab = 4; // Editor
                                            self.logs.push(format!(
                                                "Opened from error: {}",
                                                p.display()
                                            ));
                                        }
                                    }
                                }
                            }
                        },
                        // Editor scrolling
                        KeyCode::Up if self.selected_tab == 4 => {
                            self.editor_scroll = self.editor_scroll.saturating_sub(1);
                        },
                        KeyCode::Down if self.selected_tab == 4 => {
                            // Increment scroll; bounds are not strictly enforced without knowing
                            // content height
                            self.editor_scroll = self.editor_scroll.saturating_add(1);
                        },
                        // Analyze currently selected file in Files tab
                        KeyCode::Char('a') => {
                            if let Some(selected) = self.file_list_state.selected() {
                                if let Some(path) = self.files.get(selected) {
                                    let p = path.clone();

                                    if p.is_file()
                                        && p.extension().and_then(|e| e.to_str()) == Some("py")
                                    {
                                        match self.run_analysis(&p) {
                                            Ok(_) => {
                                                self.logs
                                                    .push(format!("Analyzed: {}", p.display()));

                                                self.selected_tab = 1; // Switch
                                                                       // to Types
                                                                       // tab
                                            },
                                            Err(e) => {
                                                self.logs.push(format!("Analysis failed: {}", e))
                                            },
                                        }
                                    } else {
                                        self.logs.push(
                                            "Select a Python (.py) file to analyze".to_string(),
                                        );
                                    }
                                }
                            }
                        },

                        // Parent directory navigation
                        KeyCode::Backspace | KeyCode::Char('\\') => {
                            if let Some(parent) = self.current_dir.parent() {
                                self.current_dir = parent.to_path_buf();

                                if let Err(e) = self.refresh_files() {
                                    self.logs
                                        .push(format!("Failed to go to parent directory: {}", e));
                                } else {
                                    self.file_list_state.select(Some(0));

                                    self.logs.push(format!(
                                        "Moved to parent directory: {}",
                                        self.current_dir.display()
                                    ));
                                }
                            } else {
                                self.logs.push("Already at root directory".to_string());
                            }
                        },

                        _ => {},
                    }
                }
            }
        }

        Ok(false)
    }

    fn refresh_files(&mut self) -> io::Result<()> {
        self.files = list_files(&self.current_dir)?;

        Ok(())
    }

    fn open_file(&mut self, path: &Path) -> io::Result<()> {
        self.file_content = Some(fs::read_to_string(path)?);

        Ok(())
    }

    fn run_analysis(&mut self, path: &Path) -> Result<(), String> {
        match Analyzer::analyze_python_file(path) {
            Ok(res) => {
                self.analysis_result = Some(res);

                self.errors = self
                    .analysis_result
                    .as_ref()
                    .map(|r| r.diagnostics.clone())
                    .unwrap_or_default();

                if !self.errors.is_empty() && self.errors_state.selected().is_none() {
                    self.errors_state.select(Some(0));
                }

                Ok(())
            },
            Err(e) => Err(e.to_string()),
        }
    }
}

fn list_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut entries = fs::read_dir(dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;

            let path = entry.path();

            let file_name = path.file_name()?.to_string_lossy();

            // Skip hidden files and directories
            if file_name.starts_with('.') {
                return None;
            }

            Some((path, entry.file_type().ok()?.is_dir()))
        })
        .collect::<Vec<_>>();

    // Sort directories first, then by name
    entries.sort_by(|(a_path, a_is_dir), (b_path, b_is_dir)| {
        // Directories first
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a_path
                .file_name()
                .unwrap_or_default()
                .cmp(b_path.file_name().unwrap_or_default()),
        }
    });

    Ok(entries.into_iter().map(|(path, _)| path).collect())
}
