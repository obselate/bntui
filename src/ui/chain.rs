use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::{app::App};
use crate::types::{format_bnt, format_time_ago};
use super::{GREEN, DIM};

// Each cell: 2-char block + 1 gap = 3 cols, 1 row tall
const BLOCK_W: u16 = 2;
const CELL_W: u16 = 3;

pub fn render(frame: &mut Frame, app: &mut App, title_area: Rect, content_area: Rect) {
    // single green border around the whole view
    let full = Rect {
        x: title_area.x,
        y: title_area.y,
        width: title_area.width.max(content_area.width),
        height: title_area.height + content_area.height,
    };

    let border = Block::default()
        .title(" Grid ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(GREEN));
    let inner = border.inner(full);
    frame.render_widget(border, full);

    if app.block_cubes.is_empty() {
        return;
    }

    let sections = Layout::vertical([
        Constraint::Min(1),    // cube + grid
        Constraint::Length(1), // horizontal rule
        Constraint::Length(1), // next block progress bar
    ])
    .split(inner);

    render_main_area(frame, app, sections[0]);
    render_hrule(frame, sections[1]);
    render_progress_bar(frame, app, sections[2]);
}

fn render_tx_list(frame: &mut Frame, block: &crate::types::BlockResponse, area: Rect) {
    if area.height == 0 {
        return;
    }

    let mut lines = Vec::new();
    let max_txs = area.height as usize;

    for (i, tx) in block.transactions.iter().enumerate() {
        if i >= max_txs {
            break;
        }

        let hash_short = &tx.hash[..tx.hash.len().min(10)];

        if tx.is_coinbase {
            lines.push(Line::from(vec![
                Span::styled(" coinbase ", Style::default().fg(GREEN)),
                Span::styled(
                    format!("{}in {}out", tx.inputs, tx.outputs),
                    Style::default().fg(DIM),
                ),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled(format!(" {}... ", hash_short), Style::default().fg(Color::White)),
                Span::styled(format_bnt(tx.fee), Style::default().fg(DIM)),
                Span::styled(
                    format!(" {}->{}", tx.inputs, tx.outputs),
                    Style::default().fg(DIM),
                ),
            ]));
        }
    }

    if block.transactions.len() > max_txs {
        // more..
        if let Some(last) = lines.last_mut() {
            *last = Line::from(Span::styled(
                format!(" +{} more...", block.transactions.len() - max_txs + 1),
                Style::default().fg(DIM),
            ));
        }
    }

    frame.render_widget(Paragraph::new(lines), area);
}

// ── Left panel: cube + block info + block time bar ──

fn render_main_area(frame: &mut Frame, app: &mut App, area: Rect) {
    let cols = Layout::horizontal([
        Constraint::Percentage(35),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .split(area);

    render_left_panel(frame, app, cols[0]);
    render_separator(frame, cols[1]);
    render_block_grid(frame, app, cols[2]);
}

fn render_left_panel(frame: &mut Frame, app: &mut App, area: Rect) {
    let sections = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(7),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .split(area);

    // spinning cube
    if app.selected < app.block_cubes.len() {
        let cube = &mut app.block_cubes[app.selected];
        cube.color = GREEN;
        cube.frozen = false;
        frame.render_widget(&mut *cube, sections[0]);
    }

    // block info below cube
    render_block_info(frame, app, sections[1]);

    let block = app.chain_blocks.get(app.selected);
    if let Some(block) = block {
    let rule: String = "─".repeat(sections[2].width as usize);
    frame.render_widget(
        Paragraph::new(Span::styled(rule, Style::default().fg(DIM))),
        sections[2],
    );
    render_tx_list(frame,block,sections[3]);
    }
}

fn render_block_info(frame: &mut Frame, app: &App, area: Rect) {
    let Some(block) = app.chain_blocks.get(app.selected) else {
        return;
    };

    let block_time_secs = if app.selected > 0 {
        app.chain_blocks
            .get(app.selected - 1)
            .map(|prev| block.timestamp.saturating_sub(prev.timestamp))
    } else {
        None
    };

    let w = area.width as usize;
    let rule: String = "─".repeat(w.saturating_sub(2));

    // header: block height
    let header = Line::from(vec![
        Span::styled(" Block ", Style::default().fg(DIM)),
        Span::styled(
            format!("#{}", block.height),
            Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
        ),
    ]);

    // separator
    let sep = Line::from(Span::styled(format!(" {}", rule), Style::default().fg(DIM)));

    // row 1: txs + reward
    let row1 = Line::from(vec![
        Span::styled(" Txs ", Style::default().fg(DIM)),
        Span::styled(format!("{:<6}", block.tx_count), Style::default().fg(Color::White)),
        Span::styled("Reward ", Style::default().fg(DIM)),
        Span::styled(format_bnt(block.reward), Style::default().fg(GREEN)),
    ]);

    // row 2: difficulty + mined time ago
    let row2 = Line::from(vec![
        Span::styled(" Diff ", Style::default().fg(DIM)),
        Span::styled(format!("{:<6}", block.difficulty), Style::default().fg(Color::White)),
        Span::styled("Mined ", Style::default().fg(DIM)),
        Span::styled(format_time_ago(block.timestamp), Style::default().fg(Color::White)),
    ]);

    // row 3: block time bar
    let row3 = if let Some(secs) = block_time_secs {
        let ratio = secs as f32 / 300.0;
        let time_color = if ratio < 0.5 {
            Color::Rgb(0, 255, 255)
        } else if ratio < 0.8 {
            GREEN
        } else if ratio < 1.2 {
            Color::Rgb(170, 255, 0)
        } else if ratio < 2.0 {
            Color::Yellow
        } else {
            Color::Rgb(255, 80, 80)
        };

        let label = " Mined in ";
        let bar_w = w.saturating_sub(label.len() + 10);
        let filled = ((ratio.min(3.0) / 3.0) * bar_w as f32) as usize;
        let target_pos = bar_w / 3;
        let bar: String = (0..bar_w)
            .map(|i| {
                if i == target_pos {
                    '│'
                } else if i < filled {
                    '▓'
                } else {
                    '·'
                }
            })
            .collect();

        let time_str = if secs < 60 {
            format!(" {}s", secs)
        } else {
            format!(" {}m {}s", secs / 60, secs % 60)
        };

        Line::from(vec![
            Span::styled(label, Style::default().fg(DIM)),
            Span::styled(bar, Style::default().fg(time_color)),
            Span::styled(time_str, Style::default().fg(Color::White)),
        ])
    } else {
        Line::from(Span::styled(" Genesis block", Style::default().fg(DIM)))
    };

    frame.render_widget(
        Paragraph::new(vec![header, sep, row1, row2, Line::from(""), row3]),
        area,
    );
}

fn render_separator(frame: &mut Frame, area: Rect) {
    let buf = frame.buffer_mut();
    let style = Style::default().fg(GREEN);
    for y in 0..area.height {
        buf[(area.x, area.y + y)].set_char('│').set_style(style);
    }
}

fn render_hrule(frame: &mut Frame, area: Rect) {
    let rule: String = "─".repeat(area.width as usize);
    frame.render_widget(
        Paragraph::new(Span::styled(rule, Style::default().fg(GREEN))),
        area,
    );
}

// ── Block grid: row gutter with heights, single-row color bar cells ──

fn render_block_grid(frame: &mut Frame, app: &mut App, area: Rect) {
    if area.width < 10 || area.height == 0 {
        return;
    }

    let buf = frame.buffer_mut();
    let total_blocks = app.chain_blocks.len();
    if total_blocks == 0 {
        return;
    }

    // dynamic gutter width based on max block height
    let max_height = app.chain_blocks.last().map_or(0, |b| b.height);
    let gutter_digits = format!("{}", max_height).len();
    let gutter_w = (gutter_digits as u16) + 1; // digits + 1 space

    // grid area: after gutter, with 1-char right margin for scrollbar
    let grid_x = area.x + gutter_w;
    let grid_w = area.width.saturating_sub(gutter_w + 1);

    let blocks_per_row = (grid_w / CELL_W) as usize;
    if blocks_per_row == 0 {
        return;
    }
    app.blocks_per_row = blocks_per_row;

    let total_rows = (total_blocks + blocks_per_row - 1) / blocks_per_row;
    let row_stride: u16 = 2; // 1 block row + 1 gap row
    let visible_rows = (area.height as usize + 1) / row_stride as usize;

    // grid pos 0 = newest block (top-left)
    let selected_grid_pos = total_blocks.saturating_sub(1).saturating_sub(app.selected);
    let selected_row = selected_grid_pos / blocks_per_row;

    // auto-scroll to keep selected row visible
    if selected_row < app.grid_scroll_offset {
        app.grid_scroll_offset = selected_row;
    } else if visible_rows > 0 && selected_row >= app.grid_scroll_offset + visible_rows {
        app.grid_scroll_offset = selected_row - visible_rows + 1;
    }

    let max_txs = app
        .chain_blocks
        .iter()
        .map(|b| b.tx_count)
        .max()
        .unwrap_or(1)
        .max(1);

    for vis_row in 0..visible_rows {
        let abs_row = app.grid_scroll_offset + vis_row;
        if abs_row >= total_rows {
            break;
        }

        let py = area.y + (vis_row as u16) * row_stride;

        // ── row gutter: height of the newest block in this row ──
        let first_grid_pos = abs_row * blocks_per_row;
        let first_block_idx = total_blocks - 1 - first_grid_pos;
        let row_height = app.chain_blocks[first_block_idx].height;
        let label = format!("{:>w$}", row_height, w = gutter_digits);

        let label_style = if abs_row == selected_row {
            Style::default().fg(GREEN)
        } else {
            Style::default().fg(DIM)
        };

        for (i, ch) in label.chars().enumerate() {
            buf[(area.x + i as u16, py)].set_char(ch).set_style(label_style);
        }

        // ── block cells ──
        for col in 0..blocks_per_row {
            let grid_pos = abs_row * blocks_per_row + col;
            if grid_pos >= total_blocks {
                break;
            }

            let block_idx = total_blocks - 1 - grid_pos;
            let px = grid_x + (col as u16) * CELL_W;

            if px + BLOCK_W > grid_x + grid_w || py >= area.y + area.height {
                continue;
            }

            if block_idx == app.selected {
                // selected = bright hole, block is shown as spinning cube
                let hole_style = Style::default().fg(GREEN);
                for dx in 0..BLOCK_W {
                    buf[(px + dx, py)].set_char('░').set_style(hole_style);
                }
            } else {
                // color gradient: white (0 tx) → green 170,255,0 (max tx)
                let block = &app.chain_blocks[block_idx];
                let t = block.tx_count as f32 / max_txs as f32;
                let r = (255.0 - 85.0 * t) as u8;
                let g = 255u8;
                let b_val = (255.0 - 255.0 * t) as u8;
                let fill_style = Style::default().fg(Color::Rgb(r, g, b_val));
                for dx in 0..BLOCK_W {
                    buf[(px + dx, py)].set_char('█').set_style(fill_style);
                }
            }
        }
    }

    // scrollbar on the right edge when content overflows
    if total_rows > visible_rows {
        render_scrollbar(
            buf,
            area.x + area.width - 1,
            area.y,
            area.height as usize,
            app.grid_scroll_offset,
            total_rows,
            visible_rows,
        );
    }
}

fn render_scrollbar(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    track_h: usize,
    offset: usize,
    total: usize,
    visible: usize,
) {
    if track_h == 0 || total <= visible {
        return;
    }

    let max_offset = total - visible;
    let thumb_h = ((visible as f32 / total as f32) * track_h as f32).max(1.0) as usize;
    let thumb_start =
        ((offset as f32 / max_offset as f32) * (track_h - thumb_h) as f32) as usize;

    let track_style = Style::default().fg(DIM);
    let thumb_style = Style::default().fg(GREEN);

    for i in 0..track_h {
        let (ch, style) = if i >= thumb_start && i < thumb_start + thumb_h {
            ('▐', thumb_style)
        } else {
            ('│', track_style)
        };
        buf[(x, y + i as u16)].set_char(ch).set_style(style);
    }
}

// ── Progress bar ──

fn render_progress_bar(frame: &mut Frame, app: &App, area: Rect) {
    let w = area.width as usize;
    if w <= 20 {
        return;
    }

    let last_ts = app.chain_blocks.last().map_or(0, |b| b.timestamp);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let elapsed = now.saturating_sub(last_ts) as f32;
    let is_found = app.block_found_display > 0.0;

    let ratio = (elapsed / 300.0).min(2.0);
    let label = " Next block ";
    let time_label = if elapsed < 60.0 {
        format!(" {:.0}s / 5m ", elapsed)
    } else {
        format!(
            " {}m {:.0}s / 5m ",
            elapsed as u64 / 60,
            elapsed % 60.0
        )
    };

    let overhead = label.len() + time_label.len();
    let usable = w.saturating_sub(overhead);
    let filled = ((ratio / 2.0) * usable as f32) as usize;
    let target_pos = usable / 2;

    let bar_color = if is_found {
        Color::Rgb(255, 255, 100)
    } else if ratio < 0.8 {
        Color::Rgb(0, 200, 255)
    } else if ratio < 1.2 {
        GREEN
    } else {
        Color::Yellow
    };

    let bar: String = (0..usable)
        .map(|i| {
            if i == target_pos {
                '▏'
            } else if i < filled {
                '█'
            } else {
                '░'
            }
        })
        .collect();

    let mut spans = vec![
        Span::styled(label, Style::default().fg(DIM)),
        Span::styled(bar, Style::default().fg(bar_color)),
        Span::styled(time_label, Style::default().fg(Color::White)),
    ];

    if is_found {
        spans.push(Span::styled(
            " FOUND ",
            Style::default()
                .fg(Color::Rgb(255, 255, 100))
                .add_modifier(Modifier::BOLD),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}
