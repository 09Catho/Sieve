use crate::scanner::{Finding, Severity};
use ratatui::{
    // backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use std::fs::File;
use std::io::{self, BufRead, BufReader};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum FilterMode {
    All,
    High,
    Medium,
    Low,
}

pub struct App {
    pub all_findings: Vec<Finding>,
    pub findings: Vec<Finding>,
    pub state: ListState,
    pub strict_mode: bool,
    pub show_help: bool,
    pub _show_quit_confirm: bool,
    pub clipboard_status: Option<String>,
    pub filter_mode: FilterMode,
    pub show_context: bool,
    pub context_lines: Option<Vec<(usize, String)>>,
}

impl App {
    pub fn new(findings: Vec<Finding>, strict: bool) -> App {
        let mut state = ListState::default();
        if !findings.is_empty() {
            state.select(Some(0));
        }
        App {
            all_findings: findings.clone(),
            findings,
            state,
            strict_mode: strict,
            show_help: false,
            _show_quit_confirm: false,
            clipboard_status: None,
            filter_mode: FilterMode::All,
            show_context: false,
            context_lines: None,
        }
    }

    pub fn next(&mut self) {
        if self.findings.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.findings.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.findings.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.findings.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn update_visible_findings(&mut self) {
        self.findings = self
            .all_findings
            .iter()
            .filter(|f| match self.filter_mode {
                FilterMode::All => true,
                FilterMode::High => f.severity == Severity::High,
                FilterMode::Medium => f.severity == Severity::Medium,
                FilterMode::Low => f.severity == Severity::Low,
            })
            .cloned()
            .collect();

        // Reset selection if out of bounds
        if self.findings.is_empty() {
            self.state.select(None);
        } else {
            self.state.select(Some(0));
        }
    }
}

pub fn get_file_context(path: &str, line_num: usize) -> io::Result<Vec<(usize, String)>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Context window: +/- 2 lines
    let start = line_num.saturating_sub(2);
    let end = line_num + 2;

    let mut lines = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        let current_line = i + 1;
        if current_line >= start && current_line <= end {
            if let Ok(l) = line {
                lines.push((current_line, l));
            }
        }
        if current_line > end {
            break;
        }
    }
    Ok(lines)
}

pub fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
        .split(f.size());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunks[0]);

    // --- LEFT PANEL: FINDINGS LIST ---
    let items: Vec<ListItem> = app
        .findings
        .iter()
        .map(|finding| {
            let (icon, color) = match finding.severity {
                Severity::High => ("FAIL", Color::Red),
                Severity::Medium => ("WARN", Color::Yellow),
                Severity::Low => ("INFO", Color::Blue),
            };

            let content = Line::from(vec![
                Span::styled(
                    format!("{: <4}", icon),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(&finding.file_path, Style::default().fg(Color::White)),
                Span::styled(
                    format!(":{}", finding.line_number),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);

            ListItem::new(content)
        })
        .collect();

    let title = format!(" Findings ({}) ", app.findings.len());
    let list_block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::Cyan));

    let list = List::new(items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::DarkGray),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, main_chunks[0], &mut app.state);

    // --- RIGHT PANEL: DETAILS ---
    let detail_block = Block::default()
        .borders(Borders::ALL)
        .title(" Detail ")
        .border_style(Style::default().fg(Color::White));

    if let Some(selected_index) = app.state.selected() {
        if let Some(finding) = app.findings.get(selected_index) {
            let severity_style = match finding.severity {
                Severity::High => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                Severity::Medium => Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
                Severity::Low => Style::default().fg(Color::Blue),
            };

            let mut text = vec![
                Line::from(vec![
                    Span::raw("Rule ID:   "),
                    Span::styled(&finding.rule_id, Style::default().fg(Color::Cyan)),
                ]),
                Line::from(vec![
                    Span::raw("Severity:  "),
                    Span::styled(format!("{:?}", finding.severity), severity_style),
                ]),
                Line::from(vec![
                    Span::raw("Confidence: "),
                    Span::raw(format!("{}%", finding.score)),
                ]),
                Line::from(vec![
                    Span::raw("Location:  "),
                    Span::styled(
                        format!(
                            "{}",
                            std::env::current_dir()
                                .unwrap_or_default()
                                .join(&finding.file_path)
                                .display()
                        ),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(format!(":{}", finding.line_number)),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "Redacted Preview:",
                    Style::default().add_modifier(Modifier::UNDERLINED),
                )),
                Line::from(Span::styled(
                    &finding.redacted_preview,
                    Style::default().fg(Color::Red).bg(Color::Black),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Why:",
                    Style::default().add_modifier(Modifier::UNDERLINED),
                )),
            ];

            // Split reasons by comma if needed, or wrap
            let reason_parts: Vec<&str> = finding.reason.split(", ").collect();
            for r in reason_parts {
                text.push(Line::from(vec![Span::raw("- "), Span::raw(r)]));
            }

            text.push(Line::from(""));
            text.push(Line::from(Span::styled(
                "Remediation:",
                Style::default().add_modifier(Modifier::UNDERLINED),
            )));
            text.push(Line::from("1. Revoke this secret immediately."));
            text.push(Line::from("2. Rotate credentials."));
            text.push(Line::from(
                "3. Use 'g' to add to baseline if this is a false positive.",
            ));

            if let Some(status) = &app.clipboard_status {
                text.push(Line::from(""));
                text.push(Line::from(Span::styled(
                    status,
                    Style::default().fg(Color::Green),
                )));
            }

            let paragraph = Paragraph::new(text)
                .block(detail_block)
                .wrap(Wrap { trim: true });

            f.render_widget(paragraph, main_chunks[1]);
        }
    } else {
        let p = Paragraph::new("No finding selected.").block(detail_block);
        f.render_widget(p, main_chunks[1]);
    }

    // --- BOTTOM BAR ---
    let mode_str = if app.strict_mode { "STRICT" } else { "NORMAL" };
    let filter_str = match app.filter_mode {
        FilterMode::All => "ALL",
        FilterMode::High => "HIGH",
        FilterMode::Medium => "MED",
        FilterMode::Low => "LOW",
    };
    let help_text =
        "q:Quit | 1-4:Filter | g:Ignore | c:Copy | r:Repair | s:Mode | Enter:Ctx | ?:Help";

    let status_bar = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" MODE: {} ", mode_str),
            Style::default().bg(Color::Blue).fg(Color::White),
        ),
        Span::raw(" "),
        Span::styled(
            format!(" FILTER: {} ", filter_str),
            Style::default().bg(Color::Magenta).fg(Color::White),
        ),
        Span::raw(" "),
        Span::raw(help_text),
    ]))
    .alignment(Alignment::Center);

    f.render_widget(status_bar, chunks[1]);

    // --- POPUPS ---
    if app.show_help {
        let area = centered_rect(60, 50, f.size());
        f.render_widget(Clear, area); // clear background
        let help_block = Block::default()
            .title(" Help - Press Esc to Close ")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::DarkGray));
        let help_content = vec![
            Line::from("Sieve - Secret Leak Tripwire"),
            Line::from(""),
            Line::from(Span::styled(
                "Navigation:",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from("  Up/Down Arrow : Select finding"),
            Line::from(""),
            Line::from(Span::styled(
                "Actions:",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from("  g : Generate Baseline Entry (Ignore this finding)"),
            Line::from("  c : Copy details to clipboard"),
            Line::from("  r : Repair finding"),
            Line::from("  s : Switch Mode (Strict/Normal)"),
            Line::from("  q : Quit / Exit"),
            Line::from(""),
            Line::from(Span::styled(
                "Modes:",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from("  Strict: Fails on Medium/High findings"),
            Line::from("  Normal: Fails on High findings only"),
            Line::from(""),
            Line::from(Span::styled(
                "Close this window:",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from("  Press Esc, q, or ?"),
        ];
        let p = Paragraph::new(help_content)
            .block(help_block)
            .wrap(Wrap { trim: true });
        f.render_widget(p, area);
    }

    if app.show_context {
        if let Some(lines) = &app.context_lines {
            let area = centered_rect(80, 60, f.size());
            f.render_widget(Clear, area);

            let context_block = Block::default()
                .title(" Context View - Esc/Enter to Close ")
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Black));

            let mut content = Vec::new();
            for (num, line) in lines {
                let style = if let Some(idx) = app.state.selected() {
                    if let Some(finding) = app.findings.get(idx) {
                        if *num == finding.line_number {
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::Gray)
                        }
                    } else {
                        Style::default().fg(Color::Gray)
                    }
                } else {
                    Style::default().fg(Color::Gray)
                };

                content.push(Line::from(vec![
                    Span::styled(format!("{: >4} | ", num), style),
                    Span::styled(line.replace('\t', "    "), style),
                ]));
            }

            let p = Paragraph::new(content)
                .block(context_block)
                .wrap(Wrap { trim: false }); // preserve indentation

            f.render_widget(p, area);
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
