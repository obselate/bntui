pub mod chain;
pub mod dashboard;

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Block, Borders, Clear},
};

use crate::app::App;

pub const GREEN: Color = Color::Rgb(170, 255, 0);
pub const DIM: Color = Color::Rgb(140, 140, 140);
pub const PLASMA_CHARS: [char; 10] = [' ', '·', '∙', ':', '░', '▒', '▓', '█', '▓', '░'];

pub fn render(frame: &mut Frame, app: &mut App) {
    let outer = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(frame.area());

    // help bar (always visible)
    let mut help_spans = vec![
        Span::styled(" [1]", Style::default().fg(GREEN)),
        Span::styled(" Dashboard  ", Style::default().fg(DIM)),
        Span::styled("[2]", Style::default().fg(GREEN)),
        Span::styled(" Grid  ", Style::default().fg(DIM)),

    ];

    match app.current_view {
        1 => {
            help_spans.extend([
                Span::styled("[s/r]", Style::default().fg(GREEN)),
                Span::styled(" Send / Receive  ", Style::default().fg(DIM)),
                Span::styled("[m]", Style::default().fg(GREEN)),
                Span::styled(" Mine  ", Style::default().fg(DIM)),
                Span::styled("[+/-]", Style::default().fg(GREEN)),
                Span::styled(" Threads  ", Style::default().fg(DIM)),
            ]);
        }
        2 => {
            help_spans.extend([
                Span::styled("[j/k]", Style::default().fg(GREEN)),
                Span::styled(" Nav  ", Style::default().fg(DIM)),
                Span::styled("[J/K]", Style::default().fg(GREEN)),
                Span::styled(" Jump  ", Style::default().fg(DIM)),
                Span::styled("[v]", Style::default().fg(GREEN)),
                Span::styled(" View in Browser  ", Style::default().fg(DIM)),
            ]);
        }
        _ => {}
    }

    help_spans.extend([
        Span::styled("[q]", Style::default().fg(GREEN)),
        Span::styled(" Quit", Style::default().fg(DIM)),
    ]);

    frame.render_widget(Paragraph::new(Line::from(help_spans)), outer[2]);

    match app.current_view {
        1 => dashboard::render(frame, app, outer[0], outer[1]),
        2 => chain::render(frame, app, outer[0], outer[1]),
        _ => {}
    }

    // send dialog overlay
    if let crate::app::InputMode::SendDialog {
        ref address,
        ref amount,
        focused,
        ref error,
    } = app.input_mode
    {
        let popup_w = 52u16;
        let popup_h = 11u16;
        let x = (frame.area().width.saturating_sub(popup_w)) / 2;
        let y = (frame.area().height.saturating_sub(popup_h)) / 2;
        let area = Rect::new(x, y, popup_w, popup_h);

        frame.render_widget(Clear, area);
        frame.render_widget(
            Block::default()
                .title(" Send BNT ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(GREEN)),
            area,
        );

        let inner = Rect::new(x + 2, y + 1, popup_w - 4, popup_h - 2);
        let fields = Layout::vertical([
            Constraint::Length(1), // address label
            Constraint::Length(1), // address input
            Constraint::Length(1), // spacer
            Constraint::Length(1), // amount label
            Constraint::Length(1), // amount input
            Constraint::Length(1), // spacer
            Constraint::Min(1),   // error or instructions
        ])
        .split(inner);

        let addr_color = if focused == 0 { GREEN } else { DIM };
        let amt_color = if focused == 1 { GREEN } else { DIM };

        frame.render_widget(
            Paragraph::new(Span::styled("Address:", Style::default().fg(addr_color))),
            fields[0],
        );
        let addr_cursor = if focused == 0 { "_" } else { "" };
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!("{}{}", address, addr_cursor),
                Style::default().fg(Color::White),
            )),
            fields[1],
        );

        frame.render_widget(
            Paragraph::new(Span::styled("Amount (BNT):", Style::default().fg(amt_color))),
            fields[3],
        );
        let amt_cursor = if focused == 1 { "_" } else { "" };
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!("{}{}", amount, amt_cursor),
                Style::default().fg(Color::White),
            )),
            fields[4],
        );

        if let Some(err) = error {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    err.as_str(),
                    Style::default().fg(Color::Red),
                )),
                fields[6],
            );
        } else {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    "Tab switch · Enter send · Esc cancel",
                    Style::default().fg(DIM),
                )),
                fields[6],
            );
        }
    }

    // flash message overlay
    if let Some(ref flash) = app.flash_message {
        let hint = if flash.copyable.is_some() {
            "[c] Copy · Esc close"
        } else if flash.persistent {
            "Esc close"
        } else {
            ""
        };
        let content_w = flash.text.len().max(hint.len()) as u16 + 4;
        let h = if hint.is_empty() { 3u16 } else { 4u16 };
        let x = (frame.area().width.saturating_sub(content_w)) / 2;
        let y = frame.area().height / 2;
        let area = Rect::new(x, y, content_w, h);
        frame.render_widget(Clear, area);

        let mut lines = vec![
            Line::from(Span::styled(
                format!(" {} ", flash.text),
                Style::default().fg(Color::White),
            )),
        ];
        if !hint.is_empty() {
            lines.push(Line::from(Span::styled(
                format!(" {} ", hint),
                Style::default().fg(DIM),
            )));
        }
        frame.render_widget(
            Paragraph::new(lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(GREEN)),
                ),
            area,
        );
    }
}
