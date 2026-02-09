use clap::Parser;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use std::path::{Path, PathBuf};

mod api;
mod app;
mod cube;
mod types;
mod ui;

#[derive(Parser)]
#[command(version, about, after_help = "\
If no directory is given, bntui will look for the Blocknet data directory in:
  1. BLOCKNET_DIR environment variable
  2. Current directory (if ./data/api.cookie exists)
  3. Platform default:
       macOS:   ~/Library/Application Support/Blocknet
       Linux:   ~/.blocknet
       Windows: %APPDATA%\\Blocknet")]
struct Cli {
    /// Path to blocknet directory [auto-detected if omitted]
    blocknet_dir: Option<String>,

    /// API host to connect to
    #[arg(long, default_value = "localhost")]
    host: String,

    /// API port to connect to
    #[arg(long, default_value_t = 8332)]
    port: u16,

    /// Path to API cookie file (default: {blocknet_dir}/data/api.cookie)
    #[arg(long)]
    cookie: Option<String>,
}

/// Check if a directory looks like a blocknet data directory.
fn has_cookie(dir: &Path) -> bool {
    dir.join("data").join("api.cookie").is_file()
}

/// Try to find the blocknet data directory automatically.
fn discover_blocknet_dir() -> Option<PathBuf> {
    // current directory
    let cwd = std::env::current_dir().ok()?;
    if has_cookie(&cwd) {
        return Some(cwd);
    }

    // macOS: ~/Library/Application Support/Blocknet
    if cfg!(target_os = "macos") {
        if let Ok(home) = std::env::var("HOME") {
            let mac_dir = PathBuf::from(home).join("Library/Application Support/Blocknet");
            if has_cookie(&mac_dir) {
                return Some(mac_dir);
            }
        }
    }

    // Linux: ~/.blocknet
    if cfg!(target_os = "linux") {
        if let Ok(home) = std::env::var("HOME") {
            let linux_dir = PathBuf::from(home).join(".blocknet");
            if has_cookie(&linux_dir) {
                return Some(linux_dir);
            }
        }
    }

    // Windows: %APPDATA%\Blocknet
    if cfg!(target_os = "windows") {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let win_dir = PathBuf::from(appdata).join("Blocknet");
            if has_cookie(&win_dir) {
                return Some(win_dir);
            }
        }
    }

    None
}


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

    let cli = Cli::parse();

    // Resolve blocknet directory: explicit arg > env var > auto-detect
    let blocknet_dir = cli
        .blocknet_dir
        .or_else(|| std::env::var("BLOCKNET_DIR").ok())
        .map(PathBuf::from)
        .or_else(|| {
            let dir = discover_blocknet_dir()?;
            eprintln!("auto-detected blocknet directory: {}", dir.display());
            Some(dir)
        })
        .unwrap_or_else(|| {
            eprintln!("error: could not find blocknet data directory");
            eprintln!();
            eprintln!("Looked for data/api.cookie in:");
            eprintln!("  - current directory");
            if cfg!(target_os = "macos") {
                eprintln!("  - ~/Library/Application Support/Blocknet");
            }
            if cfg!(target_os = "linux") {
                eprintln!("  - ~/.blocknet");
            }
            if cfg!(target_os = "windows") {
                eprintln!("  - %APPDATA%\\Blocknet");
            }
            eprintln!();
            eprintln!("Make sure the Blocknet daemon is running (it creates the cookie file),");
            eprintln!("or provide the path explicitly:");
            eprintln!();
            eprintln!("  bntui /path/to/blocknet");
            eprintln!("  export BLOCKNET_DIR=/path/to/blocknet");
            eprintln!();
            eprintln!("Run 'bntui --help' for more info.");
            std::process::exit(1);
        });

    // Canonicalize to absolute path so relative paths don't break
    let blocknet_dir = blocknet_dir.canonicalize().unwrap_or_else(|_| {
        eprintln!("error: directory does not exist: {}", blocknet_dir.display());
        eprintln!();
        eprintln!("Double-check the path and make sure the Blocknet daemon has been run");
        eprintln!("at least once (it creates the data directory on first start).");
        std::process::exit(1);
    });

    let cookie_path = cli
        .cookie
        .map(PathBuf::from)
        .unwrap_or_else(|| blocknet_dir.join("data").join("api.cookie"));

    // Validate cookie file before trying to read it
    if !cookie_path.exists() {
        eprintln!("error: cookie file not found: {}", cookie_path.display());
        eprintln!();
        eprintln!("The Blocknet daemon creates this file on startup.");
        eprintln!("Make sure the daemon is running with the API enabled (--api flag).");
        std::process::exit(1);
    }

    let cookie_path_str = cookie_path.to_string_lossy().into_owned();
    let base_url = format!("http://{}:{}", cli.host, cli.port);
    let api = match api::ApiClient::new(&base_url, &cookie_path_str) {
        Ok(api) => api,
        Err(e) => {
            let err = e.to_string();
            eprintln!("error: {err}");
            eprintln!();
            if err.contains("permission denied") || err.contains("Permission denied") {
                eprintln!("The cookie file exists but isn't readable by your user.");
                eprintln!("Check the file permissions:");
                eprintln!("  {}", cookie_path.display());
            } else {
                eprintln!("Is the Blocknet daemon running? Make sure it's started with --api.");
                eprintln!("Cookie path: {}", cookie_path.display());
            }
            std::process::exit(1);
        }
    };

    // Quick check that the daemon is actually responding before entering the TUI
    if let Err(e) = api.get_status().await {
        eprintln!("error: could not connect to Blocknet daemon at {base_url}");
        eprintln!("  {e}");
        eprintln!();
        eprintln!("The cookie file was found, so the daemon may have been running earlier.");
        eprintln!("Make sure the daemon is running and the API is enabled (--api flag).");
        std::process::exit(1);
    }

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &api).await;
    ratatui::restore();

    result
}
