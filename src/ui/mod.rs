pub mod chain;
pub mod dashboard;

use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
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
    let help_spans = vec![
        Span::styled(" [1]", Style::default().fg(GREEN)),
        Span::styled(" Dashboard  ", Style::default().fg(DIM)),
        Span::styled("[2]", Style::default().fg(GREEN)),
        Span::styled(" Grid  ", Style::default().fg(DIM)),
        Span::styled("[j/k]", Style::default().fg(GREEN)),
        Span::styled(" Nav  ", Style::default().fg(DIM)),
        Span::styled("[J/K]", Style::default().fg(GREEN)),
        Span::styled(" Jump  ", Style::default().fg(DIM)),
        Span::styled("[m]", Style::default().fg(GREEN)),
        Span::styled(" Mine  ", Style::default().fg(DIM)),
        Span::styled("[+/-]", Style::default().fg(GREEN)),
        Span::styled(" Threads  ", Style::default().fg(DIM)),
        Span::styled("[q]", Style::default().fg(GREEN)),
        Span::styled(" Quit", Style::default().fg(DIM)),
    ];
    frame.render_widget(Paragraph::new(Line::from(help_spans)), outer[2]);

    match app.current_view {
        1 => dashboard::render(frame, app, outer[0], outer[1]),
        2 => chain::render(frame, app, outer[0], outer[1]),
        _ => {}
    }
}
