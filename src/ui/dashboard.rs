use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph, Sparkline},
};

use crate::app::App;
use crate::types::{format_bnt, format_time_ago};
use super::{GREEN, DIM, PLASMA_CHARS};

pub fn render(frame: &mut Frame, app: &mut App, title_area: Rect, content_area: Rect) {
    // title
    let title = Paragraph::new("Blocknet Dashboard")
        .block(Block::default().title(" Dashboard ").borders(Borders::ALL))
        .style(Style::new().fg(GREEN))
        .alignment(Alignment::Center);
    frame.render_widget(title, title_area);

    // dashboard: panels + recent blocks ticker
    let dashboard = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(content_area);

    // 2x2 grid
    let rows = Layout::vertical([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(dashboard[0]);

    let top_cols = Layout::horizontal([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(rows[0]);

    let bot_cols = Layout::horizontal([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(rows[1]);

    render_chain_panel(frame, app, top_cols[0]);
    render_wallet_panel(frame, app, top_cols[1]);
    render_mempool_panel(frame, app, bot_cols[0]);
    render_mining_panel(frame, app, bot_cols[1]);
    render_recent_ticker(frame, app, dashboard[1]);
}

fn render_chain_panel(frame: &mut Frame, app: &App, area: Rect) {
    let chain_border = Block::default().title(" Chain ").borders(Borders::ALL);
    let chain_inner = chain_border.inner(area);
    frame.render_widget(chain_border.style(Style::new().fg(GREEN)), area);

    let chain_parts = Layout::vertical([
        Constraint::Length(5),
        Constraint::Length(1), // spacer
        Constraint::Length(1), // diff label + lo/avg/hi
        Constraint::Min(1),    // sparkline
    ])
    .split(chain_inner);

    if let Some(ref stats) = app.status {
        let mut lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  Height: ", Style::default().fg(DIM)),
                Span::styled(
                    format!("{}", stats.chain_height),
                    Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("  Peers:  ", Style::default().fg(DIM)),
                Span::styled(format!("{}", stats.peers), Style::default().fg(Color::White)),
            ]),
        ];
        if stats.syncing {
            lines.push(Line::from(vec![
                Span::styled("  Sync:   ", Style::default().fg(DIM)),
                Span::styled(
                    format!(
                        "{}/{} ({})",
                        stats.sync_progress,
                        stats.sync_target,
                        stats.sync_percent.as_deref().unwrap_or("0%")
                    ),
                    Style::default().fg(Color::Yellow),
                ),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("  Sync:   ", Style::default().fg(DIM)),
                Span::styled("synced", Style::default().fg(GREEN)),
            ]));
        }
        frame.render_widget(Paragraph::new(lines), chain_parts[0]);
    } else {
        frame.render_widget(
            Paragraph::new(" Waiting for node...").style(Style::new().fg(DIM)),
            chain_parts[0],
        );
    }

    // difficulty line chart (braille)
    let difficulties: Vec<u64> = app.chain_blocks.iter().map(|b| b.difficulty).collect();
    if !difficulties.is_empty() {
        let chart_w = chain_parts[3].width as usize;
        let slice = &difficulties[difficulties.len().saturating_sub(chart_w)..];
        let lo = slice.iter().copied().min().unwrap_or(0);
        let hi = slice.iter().copied().max().unwrap_or(0);
        let avg = slice.iter().copied().sum::<u64>() / slice.len() as u64;

        let stats_line = Line::from(vec![
            Span::styled("  diff ", Style::default().fg(DIM)),
            Span::styled("lo ", Style::default().fg(DIM)),
            Span::styled(format!("{}", lo), Style::default().fg(Color::White)),
            Span::styled("  avg ", Style::default().fg(DIM)),
            Span::styled(format!("{}", avg), Style::default().fg(Color::White)),
            Span::styled("  hi ", Style::default().fg(DIM)),
            Span::styled(format!("{}", hi), Style::default().fg(Color::White)),
        ]);
        frame.render_widget(Paragraph::new(stats_line), chain_parts[2]);

        // convert to (f64, f64) for Chart
        let data: Vec<(f64, f64)> = slice
            .iter()
            .enumerate()
            .map(|(i, &v)| (i as f64, v as f64))
            .collect();

        // pad bounds so the line isn't crushed flat
        let margin = ((hi - lo) as f64 * 0.1).max(1.0);
        let y_lo = lo as f64 - margin;
        let y_hi = hi as f64 + margin;

        let dataset = Dataset::default()
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(GREEN))
            .data(&data);

        let chart = Chart::new(vec![dataset])
            .x_axis(Axis::default().bounds([0.0, (slice.len() - 1).max(1) as f64]))
            .y_axis(Axis::default().bounds([y_lo, y_hi]));

        frame.render_widget(chart, chain_parts[3]);
    }
}

fn render_wallet_panel(frame: &mut Frame, app: &App, area: Rect) {
    let wallet_border =
        Block::default().title(" Wallet ").borders(Borders::ALL).style(Style::new().fg(GREEN));
    let wallet_inner = wallet_border.inner(area);
    frame.render_widget(wallet_border, area);

    let wallet_parts = Layout::vertical([
        Constraint::Length(5),
        Constraint::Min(1), // constellation
    ])
    .split(wallet_inner);

    if let Some(ref balance) = app.balance {
        let lines = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  Spendable: ", Style::default().fg(DIM)),
                Span::styled(
                    format_bnt(balance.spendable),
                    Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("  Pending:   ", Style::default().fg(DIM)),
                Span::styled(format_bnt(balance.pending), Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::styled("  Total:     ", Style::default().fg(DIM)),
                Span::styled(format_bnt(balance.total), Style::default().fg(Color::White)),
            ]),
        ];
        frame.render_widget(Paragraph::new(lines), wallet_parts[0]);

        render_constellation(frame, balance.outputs_unspent, app.tick_count, wallet_parts[1]);
    } else {
        frame.render_widget(
            Paragraph::new(" Waiting for data...").style(Style::new().fg(DIM)),
            wallet_parts[0],
        );
    }
}

fn render_constellation(frame: &mut Frame, utxo_count: u32, tick: u64, area: Rect) {
    let w = area.width as usize;
    let h = area.height as usize;
    if w == 0 || h == 0 || utxo_count == 0 {
        return;
    }

    let t = tick as f64 * 0.016; // ~seconds

    // star characters by visual weight (ASCII-safe)
    const STARS: [char; 6] = ['.', '·', ':', '+', '*', '#'];

    // independent irrationals for quasi-random 2D scatter
    const PHI: f64 = 1.618033988749895;   // golden ratio
    const SQRT2: f64 = 1.414213562373095;  // sqrt(2), independent from PHI

    let mut lines: Vec<Line> = Vec::with_capacity(h);

    // pre-compute star positions into a grid
    let mut grid: Vec<Vec<Option<(usize, f64)>>> = vec![vec![None; w]; h];

    for i in 0..utxo_count as usize {
        let fi = i as f64;

        // two independent irrational multipliers — no diagonal correlation
        let base_x = (fi * PHI).fract();
        let base_y = (fi * SQRT2).fract();

        // gentle orbit drift
        let drift_x = (t * 0.08 + fi * 0.7).sin() * 0.03;
        let drift_y = (t * 0.06 + fi * 1.1).cos() * 0.04;

        let px = (((base_x + drift_x).fract() + 1.0).fract() * w as f64) as usize % w;
        let py = (((base_y + drift_y).fract() + 1.0).fract() * h as f64) as usize % h;

        // twinkle phase unique to each star
        let phase = fi * 2.399 + t * (0.8 + (fi * 0.3).sin() * 0.4);

        // star magnitude (0-5) based on twinkle
        let twinkle = phase.sin() * 0.5 + 0.5; // 0..1
        let mag = (twinkle * 5.0) as usize;

        // only place if cell is empty (first one wins)
        if grid[py][px].is_none() {
            grid[py][px] = Some((mag, twinkle));
        }
    }

    // render grid to styled lines
    for row in &grid {
        let spans: Vec<Span> = row
            .iter()
            .map(|cell| {
                if let Some((mag, twinkle)) = cell {
                    let ch = STARS[*mag];
                    // color: dim white → bright green based on twinkle
                    let g = (100.0 + twinkle * 155.0) as u8;
                    let r = (twinkle * 120.0) as u8;
                    Span::styled(
                        String::from(ch),
                        Style::default().fg(Color::Rgb(r, g, 0)),
                    )
                } else {
                    Span::raw(" ")
                }
            })
            .collect();
        lines.push(Line::from(spans));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_mempool_panel(frame: &mut Frame, app: &App, area: Rect) {
    let mempool_border =
        Block::default().title(" Mempool ").borders(Borders::ALL).style(Style::new().fg(GREEN));
    let mempool_inner = mempool_border.inner(area);
    frame.render_widget(mempool_border, area);

    if app.mempool_history.is_empty() {
        let mempool_parts = Layout::vertical([Constraint::Min(1)]).split(mempool_inner);

        if let Some(ref mempool) = app.mempool {
            let lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("  Transactions: ", Style::default().fg(DIM)),
                    Span::styled(format!("{}", mempool.count), Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("  Size:         ", Style::default().fg(DIM)),
                    Span::styled(
                        format!("{} bytes", mempool.size_bytes),
                        Style::default().fg(Color::White),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("  Avg fee:      ", Style::default().fg(DIM)),
                    Span::styled(
                        format_bnt(mempool.avg_fee as u64),
                        Style::default().fg(Color::White),
                    ),
                ]),
            ];
            frame.render_widget(Paragraph::new(lines), mempool_parts[0]);
        } else {
            frame.render_widget(
                Paragraph::new(" Waiting for data...").style(Style::new().fg(DIM)),
                mempool_parts[0],
            );
        }
    } else {
        // 3 stacked sparklines: txs, size, fee
        let mempool_parts = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(mempool_inner);

        let mp_w = mempool_parts[1].width as usize;

        // tx count
        let tx_slice =
            &app.mempool_history[app.mempool_history.len().saturating_sub(mp_w)..];
        let tx_cur = tx_slice.last().copied().unwrap_or(0);
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  txs ", Style::default().fg(DIM)),
                Span::styled(
                    format!("{}", tx_cur),
                    Style::default().fg(Color::Rgb(0, 200, 255)),
                ),
            ])),
            mempool_parts[0],
        );
        frame.render_widget(
            Sparkline::default()
                .data(tx_slice)
                .style(Style::default().fg(Color::Rgb(0, 200, 255))),
            mempool_parts[1],
        );

        // size bytes
        let size_slice =
            &app.mempool_size_history[app.mempool_size_history.len().saturating_sub(mp_w)..];
        let size_cur = size_slice.last().copied().unwrap_or(0);
        let size_str = if size_cur >= 1_000_000 {
            format!("{:.1} MB", size_cur as f64 / 1_000_000.0)
        } else if size_cur >= 1_000 {
            format!("{:.1} KB", size_cur as f64 / 1_000.0)
        } else {
            format!("{} B", size_cur)
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  size ", Style::default().fg(DIM)),
                Span::styled(size_str, Style::default().fg(Color::Yellow)),
            ])),
            mempool_parts[2],
        );
        frame.render_widget(
            Sparkline::default()
                .data(size_slice)
                .style(Style::default().fg(Color::Yellow)),
            mempool_parts[3],
        );

        // avg fee
        let fee_slice =
            &app.mempool_fee_history[app.mempool_fee_history.len().saturating_sub(mp_w)..];
        let fee_cur = fee_slice.last().copied().unwrap_or(0);
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  fee ", Style::default().fg(DIM)),
                Span::styled(format_bnt(fee_cur), Style::default().fg(Color::Magenta)),
            ])),
            mempool_parts[4],
        );
        frame.render_widget(
            Sparkline::default()
                .data(fee_slice)
                .style(Style::default().fg(Color::Magenta)),
            mempool_parts[5],
        );
    }
}

fn render_mining_panel(frame: &mut Frame, app: &App, area: Rect) {
    let mining_border =
        Block::default().title(" Mining ").borders(Borders::ALL).style(Style::new().fg(GREEN));
    let mining_inner = mining_border.inner(area);
    frame.render_widget(mining_border, area);

    let mining_parts = Layout::vertical([
        Constraint::Length(7),
        Constraint::Min(1),
    ])
    .split(mining_inner);

    if let Some(ref mining) = app.mining {
        let status_line = if mining.running {
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled("●", Style::default().fg(GREEN)),
                Span::styled(
                    format!(" Mining  ({} threads)", mining.threads),
                    Style::default().fg(GREEN),
                ),
            ])
        } else {
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled("○", Style::default().fg(DIM)),
                Span::styled(" Idle", Style::default().fg(DIM)),
            ])
        };
        let mut lines = vec![Line::from(""), status_line];
        if mining.running {
            lines.push(Line::from(vec![
                Span::styled("  Hashrate:  ", Style::default().fg(DIM)),
                Span::styled(
                    format!("{:.2} H/s", mining.hashrate),
                    Style::default().fg(Color::White),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Hashes:    ", Style::default().fg(DIM)),
                Span::styled(format!("{}", mining.hash_count), Style::default().fg(Color::White)),
            ]));
        }
        lines.push(Line::from(vec![
            Span::styled("  Found:     ", Style::default().fg(DIM)),
            Span::styled(
                format!("{} blocks", mining.blocks_found),
                Style::default().fg(if mining.blocks_found > 0 { GREEN } else { Color::White }),
            ),
        ]));
        frame.render_widget(Paragraph::new(lines), mining_parts[0]);
    } else {
        frame.render_widget(
            Paragraph::new(" Waiting for data...").style(Style::new().fg(DIM)),
            mining_parts[0],
        );
    }

    // plasma interference field
    render_plasma(frame, app, mining_parts[1]);
}

fn render_plasma(frame: &mut Frame, app: &App, area: Rect) {
    let w = area.width as usize;
    let h = area.height as usize;
    if w == 0 || h == 0 {
        return;
    }

    let t = app.plasma_t;
    let intensity = app.plasma_intensity;
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;

    let mut plasma_lines: Vec<Line> = Vec::new();
    for row in 0..h {
        let mut spans: Vec<Span> = Vec::new();
        let y = row as f32;
        for col in 0..w {
            let x = col as f32;

            // 4 overlapping wave functions
            let v1 = (x * 0.15 + t * 1.3).sin();
            let v2 = (y * 0.2 + t * 0.9).cos();
            let dx = x - cx;
            let dy = (y - cy) * 2.0;
            let dist = (dx * dx + dy * dy).sqrt();
            let v3 = (dist * 0.12 - t * 1.6).sin();
            let v4 = ((x * 0.07 + y * 0.13 + t * 0.7).sin()
                + (x * 0.13 - y * 0.09 + t * 1.1).cos())
                * 0.5;

            let mut v = (v1 + v2 + v3 + v4) / 4.0;
            v = v * 0.5 + 0.5;

            // shockwave
            if app.shockwave_t >= 0.0 {
                let ring_radius = app.shockwave_t * (w as f32 * 0.5);
                let ring_dist = (dist - ring_radius).abs();
                let ring_width = 2.0 + app.shockwave_t * 3.0;
                if ring_dist < ring_width {
                    let ring_v = 1.0 - ring_dist / ring_width;
                    let fade = (1.0 - app.shockwave_t / 3.0).max(0.0);
                    v = (v + ring_v * fade * 1.5).min(1.0);
                }
            }

            v *= intensity;

            let ci = (v * 9.0).min(9.0).max(0.0) as usize;
            let ch = PLASMA_CHARS[ci];

            let hue = v * 0.8 + (dist * 0.01 + t * 0.3).sin() * 0.2;
            let r = (hue * 170.0).min(170.0).max(0.0) as u8;
            let g = (v * 255.0).min(255.0) as u8;
            let b = ((1.0 - hue) * 40.0).max(0.0) as u8;

            // shockwave flash
            let (r, g, b) = if app.shockwave_t >= 0.0 {
                let ring_radius = app.shockwave_t * (w as f32 * 0.5);
                let ring_dist = (dist - ring_radius).abs();
                let ring_width = 2.0 + app.shockwave_t * 3.0;
                if ring_dist < ring_width {
                    let flash =
                        (1.0 - ring_dist / ring_width) * (1.0 - app.shockwave_t / 3.0).max(0.0);
                    (
                        (r as f32 + (255.0 - r as f32) * flash) as u8,
                        g,
                        (b as f32 + (200.0 - b as f32) * flash) as u8,
                    )
                } else {
                    (r, g, b)
                }
            } else {
                (r, g, b)
            };

            spans.push(Span::styled(
                String::from(ch),
                Style::default().fg(Color::Rgb(r, g, b)),
            ));
        }
        plasma_lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(plasma_lines), area);
}

fn render_recent_ticker(frame: &mut Frame, app: &App, area: Rect) {
    let recent_text: String = app
        .chain_blocks
        .iter()
        .rev()
        .take(8)
        .map(|b| {
            format!(
                "#{} {}tx {}",
                b.height,
                b.tx_count,
                format_time_ago(b.timestamp)
            )
        })
        .collect::<Vec<_>>()
        .join("  \u{2502}  ");

    let recent = Paragraph::new(Line::from(format!(" {}", recent_text)))
        .block(Block::default().title(" Recent Blocks ").borders(Borders::ALL))
        .style(Style::new().fg(DIM));
    frame.render_widget(recent, area);
}
