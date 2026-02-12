use clap::Parser;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;

mod api;
mod app;
mod cube;
mod types;
mod ui;

struct EmbeddedBinary {
    name: &'static str,
    bytes: &'static [u8],
}

include!(concat!(env!("OUT_DIR"), "/embedded_binaries.rs"));

#[derive(Parser)]
#[command(version, about, after_help = "\
If no directory is given, bntui will use this resolution order:
  1. BLOCKNET_DIR environment variable
  2. Existing cookie location (current dir, then platform default)
  3. Platform default path (created automatically if missing):
       macOS:   ~/Library/Application Support/Blocknet
       Linux:   ~/.blocknet
       Windows: %APPDATA%\\Blocknet

If host=localhost and port=8332 and no cookie exists, bntui will auto-start
the embedded Blocknet daemon with:
  --api 127.0.0.1:8332 --daemon --data <dir>/data --wallet <dir>/wallet.dat")]
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

fn default_blocknet_dir() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        if let Ok(home) = std::env::var("HOME") {
            return Some(PathBuf::from(home).join("Library/Application Support/Blocknet"));
        }
    }

    if cfg!(target_os = "linux") {
        if let Ok(home) = std::env::var("HOME") {
            return Some(PathBuf::from(home).join(".blocknet"));
        }
    }

    if cfg!(target_os = "windows") {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return Some(PathBuf::from(appdata).join("Blocknet"));
        }
    }

    None
}

fn is_local_host(host: &str) -> bool {
    matches!(
        host.to_ascii_lowercase().as_str(),
        "localhost" | "127.0.0.1" | "::1"
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BinaryOs {
    Linux,
    Macos,
    Windows,
    Unknown,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BinaryArch {
    X86_64,
    Aarch64,
    X86,
    Unknown,
}

fn runtime_os() -> BinaryOs {
    #[cfg(target_os = "linux")]
    {
        return BinaryOs::Linux;
    }
    #[cfg(target_os = "macos")]
    {
        return BinaryOs::Macos;
    }
    #[cfg(target_os = "windows")]
    {
        return BinaryOs::Windows;
    }
    #[allow(unreachable_code)]
    BinaryOs::Unknown
}

fn runtime_arch() -> BinaryArch {
    #[cfg(target_arch = "x86_64")]
    {
        return BinaryArch::X86_64;
    }
    #[cfg(target_arch = "aarch64")]
    {
        return BinaryArch::Aarch64;
    }
    #[cfg(target_arch = "x86")]
    {
        return BinaryArch::X86;
    }
    #[allow(unreachable_code)]
    BinaryArch::Unknown
}

fn os_tokens_for(os: BinaryOs) -> &'static [&'static str] {
    match os {
        BinaryOs::Linux => &["linux"],
        BinaryOs::Macos => &["darwin", "macos", "mac", "osx"],
        BinaryOs::Windows => &["windows", "win"],
        BinaryOs::Unknown => &[],
    }
}

fn arch_tokens_for(arch: BinaryArch) -> &'static [&'static str] {
    match arch {
        BinaryArch::X86_64 => &["x86_64", "amd64"],
        BinaryArch::Aarch64 => &["aarch64", "arm64"],
        BinaryArch::X86 => &["x86", "386", "i686"],
        BinaryArch::Unknown => &[],
    }
}

fn parse_pe_arch(bytes: &[u8]) -> Option<BinaryArch> {
    if bytes.len() < 0x40 || &bytes[0..2] != b"MZ" {
        return None;
    }
    let pe_offset = u32::from_le_bytes([bytes[0x3C], bytes[0x3D], bytes[0x3E], bytes[0x3F]]) as usize;
    if bytes.len() < pe_offset + 6 || &bytes[pe_offset..pe_offset + 4] != b"PE\0\0" {
        return None;
    }
    let machine = u16::from_le_bytes([bytes[pe_offset + 4], bytes[pe_offset + 5]]);
    let arch = match machine {
        0x8664 => BinaryArch::X86_64,
        0xAA64 => BinaryArch::Aarch64,
        0x014C => BinaryArch::X86,
        _ => BinaryArch::Unknown,
    };
    Some(arch)
}

fn parse_elf_arch(bytes: &[u8]) -> Option<BinaryArch> {
    if bytes.len() < 20 || &bytes[0..4] != b"\x7FELF" {
        return None;
    }
    let little_endian = bytes.get(5).copied().unwrap_or(1) == 1;
    let machine = if little_endian {
        u16::from_le_bytes([bytes[18], bytes[19]])
    } else {
        u16::from_be_bytes([bytes[18], bytes[19]])
    };
    let arch = match machine {
        0x003E => BinaryArch::X86_64,
        0x00B7 => BinaryArch::Aarch64,
        0x0003 => BinaryArch::X86,
        _ => BinaryArch::Unknown,
    };
    Some(arch)
}

fn parse_macho_arch(bytes: &[u8]) -> Option<BinaryArch> {
    if bytes.len() < 8 {
        return None;
    }
    let magic = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let (is_macho, little_endian) = match magic {
        0xFEEDFACE | 0xFEEDFACF => (true, false),
        0xCEFAEDFE | 0xCFFAEDFE => (true, true),
        _ => (false, false),
    };
    if !is_macho {
        return None;
    }
    let cputype_raw = if little_endian {
        u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]])
    } else {
        u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]])
    };
    let arch = match cputype_raw {
        0x01000007 => BinaryArch::X86_64,
        0x0100000C => BinaryArch::Aarch64,
        0x00000007 => BinaryArch::X86,
        _ => BinaryArch::Unknown,
    };
    Some(arch)
}

fn detect_binary_target(entry: &EmbeddedBinary) -> (BinaryOs, BinaryArch) {
    if let Some(arch) = parse_pe_arch(entry.bytes) {
        return (BinaryOs::Windows, arch);
    }
    if let Some(arch) = parse_elf_arch(entry.bytes) {
        return (BinaryOs::Linux, arch);
    }
    if let Some(arch) = parse_macho_arch(entry.bytes) {
        return (BinaryOs::Macos, arch);
    }
    (BinaryOs::Unknown, BinaryArch::Unknown)
}

fn select_embedded_daemon() -> Option<&'static EmbeddedBinary> {
    fn score_for(entry: &EmbeddedBinary) -> i32 {
        let (detected_os, detected_arch) = detect_binary_target(entry);
        let runtime_os = runtime_os();
        let runtime_arch = runtime_arch();
        let lower = entry.name.to_ascii_lowercase();
        let daemon_hint = lower.contains("blocknet") || lower.contains("daemon");

        let os_name_match = os_tokens_for(runtime_os).iter().any(|t| lower.contains(t));
        let arch_name_match = arch_tokens_for(runtime_arch).iter().any(|t| lower.contains(t));

        let mut score = 0;
        if detected_os == runtime_os {
            score += 100;
        }
        if detected_arch == runtime_arch {
            score += 100;
        }
        if detected_os == BinaryOs::Unknown && os_name_match {
            score += 20;
        }
        if detected_arch == BinaryArch::Unknown && arch_name_match {
            score += 20;
        }
        if daemon_hint {
            score += 5;
        }
        if runtime_os == BinaryOs::Windows && lower.ends_with(".exe") {
            score += 2;
        }
        score
    }

    EMBEDDED_BINARIES
        .iter()
        .max_by_key(|entry| score_for(entry))
        .and_then(|entry| {
            let (detected_os, detected_arch) = detect_binary_target(entry);
            let os_ok = detected_os == runtime_os()
                || (detected_os == BinaryOs::Unknown
                    && os_tokens_for(runtime_os())
                        .iter()
                        .any(|t| entry.name.to_ascii_lowercase().contains(t)));
            let arch_ok = detected_arch == runtime_arch()
                || detected_arch == BinaryArch::Unknown;

            if os_ok && arch_ok {
                Some(entry)
            } else {
                None
            }
        })
}

fn write_embedded_binary(entry: &EmbeddedBinary) -> Result<PathBuf, String> {
    let mut path = std::env::temp_dir().join("bntui-embedded-daemon");
    std::fs::create_dir_all(&path).map_err(|e| format!("can't create temp dir: {e}"))?;
    path.push(entry.name);
    std::fs::write(&path, entry.bytes).map_err(|e| format!("can't write embedded daemon: {e}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path)
            .map_err(|e| format!("can't read daemon file metadata: {e}"))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms)
            .map_err(|e| format!("can't set daemon file permissions: {e}"))?;
    }

    Ok(path)
}

fn try_spawn_embedded_daemon(
    host: &str,
    port: u16,
    blocknet_dir: &Path,
) -> Result<PathBuf, String> {
    if std::env::var("BNTUI_SKIP_EMBEDDED_DAEMON").ok().as_deref() == Some("1") {
        return Err("embedded daemon autostart disabled (BNTUI_SKIP_EMBEDDED_DAEMON=1)".to_string());
    }

    let entry = select_embedded_daemon().ok_or_else(|| {
        "no embedded daemon binary found for this platform in binaries/".to_string()
    })?;
    let daemon_path = write_embedded_binary(entry)?;

    let api_addr = format!("{}:{}", host, port);
    let data_dir = blocknet_dir.join("data");
    let wallet_path = blocknet_dir.join("wallet.dat");

    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("can't create data dir {}: {}", data_dir.display(), e))?;

    let mut cmd = Command::new(&daemon_path);
    cmd.arg("--api")
        .arg(&api_addr)
        .arg("--daemon")
        .arg("--data")
        .arg(&data_dir)
        .arg("--wallet")
        .arg(&wallet_path);
    cmd.spawn()
        .map_err(|e| format!("failed to launch embedded daemon {}: {}", daemon_path.display(), e))?;

    Ok(daemon_path)
}

async fn wait_for_daemon(base_url: &str, cookie_path: &Path, timeout_secs: u64) -> Result<api::ApiClient, String> {
    let start = std::time::Instant::now();
    while start.elapsed().as_secs() < timeout_secs {
        if cookie_path.is_file() {
            if let Ok(client) = api::ApiClient::new(base_url, &cookie_path.to_string_lossy()) {
                if client.get_status().await.is_ok() {
                    return Ok(client);
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }

    Err(format!(
        "daemon did not become ready within {}s (cookie: {})",
        timeout_secs,
        cookie_path.display()
    ))
}

fn can_bind_local_port(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

fn choose_available_local_port(preferred: u16) -> Result<u16, String> {
    if can_bind_local_port(preferred) {
        return Ok(preferred);
    }
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .map_err(|e| format!("can't allocate local API port: {}", e))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("can't read allocated API port: {}", e))?
        .port();
    Ok(port)
}

fn discover_cookie_candidates(primary: &Path, blocknet_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = vec![primary.to_path_buf()];

    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("data").join("api.cookie"));
    }

    candidates.push(blocknet_dir.join("data").join("api.cookie"));

    if cfg!(target_os = "macos") {
        if let Ok(home) = std::env::var("HOME") {
            candidates.push(
                PathBuf::from(&home)
                    .join("Library/Application Support/com.blocknet.wallet/data/api.cookie"),
            );
            candidates.push(
                PathBuf::from(home).join("Library/Application Support/Blocknet/data/api.cookie"),
            );
        }
    }

    candidates.dedup();
    candidates
}

async fn try_connect_local_with_cookie(
    host: &str,
    port: u16,
    cookie_path: &Path,
) -> Option<api::ApiClient> {
    if !cookie_path.is_file() {
        return None;
    }
    let base_url = format!("http://{}:{}", host, port);
    let client = api::ApiClient::new(&base_url, &cookie_path.to_string_lossy()).ok()?;
    if client.get_status().await.is_ok() {
        Some(client)
    } else {
        None
    }
}


fn copy_to_clipboard(text: &str) -> Result<(), String> {
    // Try system clipboard commands first (arboard lies about success on Wayland
    // then drops the content when the Clipboard object is freed)
    use std::io::Write;
    use std::process::{Command, Stdio};
    let tools: &[(&str, &[&str])] = &[
        ("wl-copy", &[]),
        ("xclip", &["-selection", "clipboard"]),
    ];
    for (cmd, args) in tools {
        if let Ok(mut child) = Command::new(cmd)
            .args(*args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            if let Some(ref mut stdin) = child.stdin {
                let _ = stdin.write_all(text.as_bytes());
            }
            drop(child.stdin.take());
            if child.wait().map(|s| s.success()).unwrap_or(false) {
                return Ok(());
            }
        }
    }
    // Last resort: arboard (works on macOS/Windows, unreliable on Wayland)
    if let Ok(mut cb) = arboard::Clipboard::new() {
        if cb.set_text(text).is_ok() {
            return Ok(());
        }
    }
    Err("Install wl-clipboard or xclip".to_string())
}

fn open_in_browser(url: &str) {
    use std::process::{Command, Stdio};
    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("xdg-open")
            .arg(url)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open")
            .arg(url)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("cmd")
            .args(["/C", "start", "", url])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
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
        let start = stats.chain_height.saturating_sub(999);
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
    if let Ok(addr) = api.get_address().await {
        app.wallet_address = Some(addr.address);
    }

    let mut should_quit = false;
    loop {
        terminal.draw(|frame| ui::render(frame, &mut app))?;

        // input handling
        while crossterm::event::poll(std::time::Duration::from_millis(0))? {
            let event = crossterm::event::read()?;
            if let Event::Key(key) = event {
                if key.kind == KeyEventKind::Press {
                    match app.input_mode {
                        app::InputMode::Normal => match key.code {
                            KeyCode::Esc => {
                                app.flash_message = None;
                            }
                            KeyCode::Char('c') => {
                                let copyable = app.flash_message.as_ref()
                                    .and_then(|f| f.copyable.clone());
                                if let Some(text) = copyable {
                                    match copy_to_clipboard(&text) {
                                        Ok(_) => {
                                            app.set_flash("Copied!".to_string());
                                        }
                                        Err(e) => {
                                            app.set_flash(format!("Clipboard error: {}", e));
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('q') => should_quit = true,
                            KeyCode::Char('1') => app.current_view = 1,
                            KeyCode::Char('2') => app.current_view = 2,
                            KeyCode::Char('s') => {
                                app.input_mode = app::InputMode::SendDialog {
                                    address: String::new(),
                                    amount: String::new(),
                                    focused: 0,
                                    error: None,
                                };
                            }
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
                                    let new_threads = mining.threads + 1;
                                    let was_running = mining.running;

                                    api.set_threads(new_threads).await.ok();
                                    if let Ok(m) = api.get_mining().await {
                                        app.mining = Some(m);
                                    }
                                    if was_running {
                                        app.threads_pending_restart = Some(app.tick_count);
                                    }
                                }
                            }
                            KeyCode::Char('-') => {
                                if let Some(ref mining) = app.mining {
                                    if mining.threads > 1 {
                                        let new_threads = mining.threads - 1;
                                        let was_running = mining.running;

                                        api.set_threads(new_threads).await.ok();
                                        if let Ok(m) = api.get_mining().await {
                                            app.mining = Some(m);
                                        }
                                        if was_running {
                                            app.threads_pending_restart = Some(app.tick_count);
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('j') => {
                                if app.current_view == 2
                                    && !app.block_cubes.is_empty()
                                    && app.selected + 1 < app.block_cubes.len()
                                {
                                    app.selected += 1;
                                }
                            }
                            KeyCode::Char('k') => {
                                if app.current_view == 2 && app.selected > 0 {
                                    app.selected -= 1;
                                }
                            }
                            KeyCode::Char('J') => {
                                if app.current_view == 2 && !app.block_cubes.is_empty() {
                                    let jump = app.blocks_per_row;
                                    let max = app.block_cubes.len() - 1;
                                    app.selected = (app.selected + jump).min(max);
                                }
                            }
                            KeyCode::Char('K') => {
                                if app.current_view == 2 && app.selected > 0 {
                                    let jump = app.blocks_per_row;
                                    app.selected = app.selected.saturating_sub(jump);
                                }
                            }
                            KeyCode::Char('r') => {
                                if let Some(ref addr) = app.wallet_address {
                                    let addr = addr.clone();
                                    match copy_to_clipboard(&addr) {
                                        Ok(_) => {
                                            app.set_flash(format!("Address copied: {}", addr))
                                        }
                                        Err(e) => {
                                            app.set_flash(format!("Clipboard error: {}", e))
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('v') => {
                                if app.current_view == 2 {
                                    if let Some(block) = app.chain_blocks.get(app.selected) {
                                        let url = format!(
                                            "https://explorer.blocknetcrypto.com/block/{}",
                                            block.height
                                        );
                                        open_in_browser(&url);
                                        app.set_flash("Opening block in browser…".to_string());
                                    }
                                }
                            }
                            _ => {}
                        },
                        app::InputMode::SendDialog {
                            ref mut address,
                            ref mut amount,
                            ref mut focused,
                            ref mut error,
                        } => match key.code {
                            KeyCode::Esc => {
                                app.input_mode = app::InputMode::Normal;
                            }
                            KeyCode::Tab | KeyCode::Down | KeyCode::Up => {
                                *focused = if *focused == 0 { 1 } else { 0 };
                            }
                            KeyCode::BackTab => {
                                *focused = if *focused == 0 { 1 } else { 0 };
                            }
                            KeyCode::Backspace => {
                                let field =
                                    if *focused == 0 { address } else { amount };
                                field.pop();
                                *error = None;
                            }
                            KeyCode::Enter => {
                                let addr = address.clone();
                                let amt_str = amount.clone();

                                if addr.is_empty() {
                                    *error = Some("Address is required".to_string());
                                } else if amt_str.is_empty() {
                                    *error = Some("Amount is required".to_string());
                                } else {
                                    match types::parse_bnt_amount(&amt_str) {
                                        None => {
                                            *error =
                                                Some("Invalid amount format".to_string());
                                        }
                                        Some(0) => {
                                            *error =
                                                Some("Amount must be greater than 0".to_string());
                                        }
                                        Some(atomic) => {
                                            match api.send_to(&addr, atomic).await {
                                                Ok(txid) => {
                                                    app.input_mode =
                                                        app::InputMode::Normal;
                                                    app.log_tx(&txid, &addr, atomic);
                                                    app.set_flash_persistent(
                                                        format!("Sent! tx: {}", txid),
                                                        txid,
                                                    );
                                                }
                                                Err(e) => {
                                                    *error = Some(e);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Char(c) => {
                                let field =
                                    if *focused == 0 { address } else { amount };
                                field.push(c);
                                *error = None;
                            }
                            _ => {}
                        },
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

        app.update_flash();

        if let Some(changed_tick) = app.threads_pending_restart {
            if app.tick_count - changed_tick > 15 {
                app.threads_pending_restart = None;
                api.stop_mining().await.ok();
                api.start_mining().await.ok();
                if let Ok(m) = api.get_mining().await {
                    app.mining = Some(m);
                }
            }
        }

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

    // Resolve blocknet directory: explicit arg > env var > discovered cookie dir > platform default.
    let mut blocknet_dir = cli
        .blocknet_dir
        .clone()
        .or_else(|| std::env::var("BLOCKNET_DIR").ok())
        .map(PathBuf::from)
        .or_else(discover_blocknet_dir)
        .or_else(default_blocknet_dir)
        .unwrap_or_else(|| {
            eprintln!("error: couldn't resolve a Blocknet data directory for this platform");
            std::process::exit(1);
        });

    if !blocknet_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(&blocknet_dir) {
            eprintln!(
                "error: couldn't create Blocknet data directory {}: {}",
                blocknet_dir.display(),
                e
            );
            std::process::exit(1);
        }
    }

    if let Ok(canonical) = blocknet_dir.canonicalize() {
        blocknet_dir = canonical;
    }

    let cookie_path = cli
        .cookie
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(|| blocknet_dir.join("data").join("api.cookie"));
    let mut base_url = format!("http://{}:{}", cli.host, cli.port);
    let mut active_cookie_path = cookie_path.clone();

    // If another local Blocknet daemon is already running, try known cookie locations first.
    let mut api = None;
    if cli.cookie.is_none() && is_local_host(&cli.host) {
        for candidate in discover_cookie_candidates(&cookie_path, &blocknet_dir) {
            if let Some(client) = try_connect_local_with_cookie(&cli.host, cli.port, &candidate).await {
                if candidate != cookie_path {
                    eprintln!("using detected cookie: {}", candidate.display());
                }
                active_cookie_path = candidate;
                api = Some(client);
                break;
            }
        }
    }

    let api = if let Some(api) = api {
        api
    } else {
        let mut launched_embedded = false;
        let mut autostart_port = cli.port;

        if cli.cookie.is_none() && is_local_host(&cli.host) {
            autostart_port = choose_available_local_port(cli.port).unwrap_or(cli.port);
            if autostart_port != cli.port {
                eprintln!(
                    "api port {} is busy; auto-starting embedded daemon on {}",
                    cli.port, autostart_port
                );
            }
        }

        if !active_cookie_path.is_file() && cli.cookie.is_none() && is_local_host(&cli.host) {
            match try_spawn_embedded_daemon(&cli.host, autostart_port, &blocknet_dir) {
                Ok(path) => {
                    launched_embedded = true;
                    base_url = format!("http://{}:{}", cli.host, autostart_port);
                    eprintln!("started embedded blocknet daemon: {}", path.display());
                }
                Err(e) => {
                    eprintln!("warning: couldn't start embedded daemon: {e}");
                }
            }
        }

        if launched_embedded {
            match wait_for_daemon(&base_url, &active_cookie_path, 30).await {
                Ok(api) => api,
                Err(e) => {
                    eprintln!("error: {e}");
                    eprintln!("The embedded daemon was started but never became ready.");
                    std::process::exit(1);
                }
            }
        } else {
            if !active_cookie_path.is_file() {
                eprintln!("error: cookie file not found: {}", active_cookie_path.display());
                eprintln!();
                eprintln!("If this is a local node, bntui can auto-start an embedded daemon when:");
                eprintln!("  - host is localhost");
                eprintln!("  - no custom --cookie is provided");
                eprintln!();
                eprintln!("Otherwise start Blocknet manually with --daemon --api <host:port>.");
                std::process::exit(1);
            }

            let cookie_path_str = active_cookie_path.to_string_lossy().into_owned();
            let api = match api::ApiClient::new(&base_url, &cookie_path_str) {
                Ok(api) => api,
                Err(e) => {
                    let err = e.to_string();
                    eprintln!("error: {err}");
                    eprintln!();
                    if err.contains("permission denied") || err.contains("Permission denied") {
                        eprintln!("The cookie file exists but isn't readable by your user.");
                        eprintln!("Check the file permissions:");
                        eprintln!("  {}", active_cookie_path.display());
                    } else {
                        eprintln!("Is the Blocknet daemon running? Make sure it's started with --api.");
                        eprintln!("Cookie path: {}", active_cookie_path.display());
                    }
                    std::process::exit(1);
                }
            };

            if let Err(e) = api.get_status().await {
                if cli.cookie.is_none() && is_local_host(&cli.host) {
                    match try_spawn_embedded_daemon(&cli.host, autostart_port, &blocknet_dir) {
                        Ok(path) => {
                            base_url = format!("http://{}:{}", cli.host, autostart_port);
                            eprintln!("started embedded blocknet daemon: {}", path.display());
                            match wait_for_daemon(&base_url, &active_cookie_path, 30).await {
                                Ok(api) => api,
                                Err(wait_err) => {
                                    eprintln!("error: {wait_err}");
                                    eprintln!("initial API error: {e}");
                                    std::process::exit(1);
                                }
                            }
                        }
                        Err(spawn_err) => {
                            eprintln!("error: could not connect to Blocknet daemon at {base_url}");
                            eprintln!("  {e}");
                            eprintln!("also failed to auto-start embedded daemon: {spawn_err}");
                            std::process::exit(1);
                        }
                    }
                } else {
                    eprintln!("error: could not connect to Blocknet daemon at {base_url}");
                    eprintln!("  {e}");
                    std::process::exit(1);
                }
            } else {
                api
            }
        }
    };

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &api).await;
    ratatui::restore();

    result
}
