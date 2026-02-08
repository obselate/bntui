use crossterm::event::{Event, KeyCode, KeyEventKind};

mod api;
mod app;
mod cube;
mod types;
mod ui;


async fn run(
    terminal: &mut ratatui::DefaultTerminal,
    api: &api::ApiClient,
) -> color_eyre::Result<()> {
    let mut app = app::App::new();

    // initial data load
    if let Ok(stats) = api.get_status().await {
        app.status = Some(stats);
    }

    if let Some(ref stats) = app.status {
        let start = stats.chain_height.saturating_sub(499);
        for h in start..=stats.chain_height {
            if let Ok(block) = api.get_block(h).await {
                app.chain_blocks.push(block);
            }
        }
        app.block_cubes = app
            .chain_blocks
            .iter()
            .map(|_| cube::SpinCube::new())
            .collect();
        app.selected = app.chain_blocks.len().saturating_sub(1);
    }

    if let Ok(mempool) = api.get_mempool().await {
        app.mempool = Some(mempool);
    }
    if let Ok(balance) = api.get_balance().await {
        app.balance = Some(balance);
    }
    if let Ok(mining) = api.get_mining().await {
        app.mining = Some(mining);
    }

    let mut should_quit = false;
    loop {
        terminal.draw(|frame| ui::render(frame, &mut app))?;

        // input handling
        while crossterm::event::poll(std::time::Duration::from_millis(0))? {
            let event = crossterm::event::read()?;
            if let Event::Key(key) = event {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => should_quit = true,
                        KeyCode::Char('1') => app.current_view = 1,
                        KeyCode::Char('2') => app.current_view = 2,
                        KeyCode::Char('m') => {
                            if let Some(ref mining) = app.mining {
                                if mining.running {
                                    api.stop_mining().await.ok();
                                } else {
                                    api.start_mining().await.ok();
                                }
                                if let Ok(m) = api.get_mining().await {
                                    app.mining = Some(m);
                                }
                            }
                        }
                        KeyCode::Char('+') | KeyCode::Char('=') => {
                            if let Some(ref mining) = app.mining {
                                api.set_threads(mining.threads + 1).await.ok();
                                if let Ok(m) = api.get_mining().await {
                                    app.mining = Some(m);
                                }
                            }
                        }
                        KeyCode::Char('-') => {
                            if let Some(ref mining) = app.mining {
                                if mining.threads > 1 {
                                    api.set_threads(mining.threads - 1).await.ok();
                                    if let Ok(m) = api.get_mining().await {
                                        app.mining = Some(m);
                                    }
                                }
                            }
                        }
                        KeyCode::Char('j') => {
                            // left/up in grid = newer
                            if app.current_view == 2
                                && !app.block_cubes.is_empty()
                                && app.selected + 1 < app.block_cubes.len()
                            {
                                app.selected += 1;
                            }
                        }
                        KeyCode::Char('k') => {
                            // right/down in grid = older
                            if app.current_view == 2 && app.selected > 0 {
                                app.selected -= 1;
                            }
                        }
                        KeyCode::Char('J') => {
                            // jump up = newer
                            if app.current_view == 2 && !app.block_cubes.is_empty() {
                                let jump = app.blocks_per_row;
                                let max = app.block_cubes.len() - 1;
                                app.selected = (app.selected + jump).min(max);
                            }
                        }
                        KeyCode::Char('K') => {
                            // jump down = older
                            if app.current_view == 2 && app.selected > 0 {
                                let jump = app.blocks_per_row;
                                app.selected = app.selected.saturating_sub(jump);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        if should_quit {
            break;
        }

        std::thread::sleep(std::time::Duration::from_millis(33));
        app.tick_count += 1;

        // update animations (only for visible view)
        if app.current_view == 2 && !app.block_cubes.is_empty() {
            let speed = app.spin_speed();
            app.update_selected_cube(speed);
        }
        if app.current_view == 1 {
            app.update_plasma();
        }
        app.update_block_found();

        // poll status every ~1 second (30 ticks × 33ms)
        if app.tick_count % 30 == 0 {
            if let Ok(stats) = api.get_status().await {
                let new_height = stats.chain_height;
                let have_height = app.chain_blocks.last().map_or(0, |b| b.height);
                app.status = Some(stats);

                if new_height > app.prev_chain_height && app.prev_chain_height > 0 {
                    app.block_found_display = 3.0;
                }
                app.prev_chain_height = new_height;

                // fetch new blocks
                if new_height > have_height && have_height > 0 {
                    let was_at_newest = app.selected + 1 >= app.chain_blocks.len();
                    for h in (have_height + 1)..=new_height {
                        if let Ok(block) = api.get_block(h).await {
                            app.chain_blocks.push(block);
                            app.block_cubes.push(cube::SpinCube::new());
                        }
                    }
                    if was_at_newest && !app.chain_blocks.is_empty() {
                        app.selected = app.chain_blocks.len() - 1;
                    }
                }
            }
        }

        // poll other data every ~3 seconds (90 ticks × 33ms)
        if app.tick_count % 90 == 0 {
            if let Ok(mempool) = api.get_mempool().await {
                app.record_mempool(&mempool);
                app.mempool = Some(mempool);
            }
            if let Ok(balance) = api.get_balance().await {
                app.balance = Some(balance);
            }
            if let Ok(mining) = api.get_mining().await {
                app.mining = Some(mining);
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let blocknet_dir = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("BLOCKNET_DIR").ok())
        .expect("usage: bnt-explorer <blocknet-dir> (or set BLOCKNET_DIR)");

    let cookie_path = format!("{}/data/api.cookie", blocknet_dir);
    let api = api::ApiClient::new("http://localhost:8332", &cookie_path)
        .map_err(|e| color_eyre::eyre::eyre!(e))?;

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &api).await;
    ratatui::restore();

    result
}
