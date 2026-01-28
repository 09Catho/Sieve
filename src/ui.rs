use crate::scanner::{Finding, Severity};
use ratatui::{
    // backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

pub struct App {
    pub findings: Vec<Finding>,
    pub state: ListState,
    pub strict_mode: bool,
    pub show_help: bool,
    pub _show_quit_confirm: bool,
    pub clipboard_status: Option<String>,
}

impl App {
    pub fn new(findings: Vec<Finding>, strict: bool) -> App {
        let mut state = ListState::default();
        if !findings.is_empty() {
            state.select(Some(0));
        }
        App {
            findings,
            state,
            strict_mode: strict,
            show_help: false,
            _show_quit_confirm: false,
            clipboard_status: None,
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
    let help_text =
        "q:Quit | g:Baseline (Ignore) | c:Copy | r:Repair | s:Switch Mode | ?:Help | \u{2191}\u{2193}:Nav";

    let status_bar = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" MODE: {} ", mode_str),
            Style::default().bg(Color::Blue).fg(Color::White),
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
