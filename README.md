<p align="center">
  <img src="assets/banner.svg" alt="BNTUI" width="600"/>
</p>

<p align="center">
  Terminal block explorer for <a href="https://github.com/blocknetprivacy/blocknet">Blocknet</a>, a privacy-focused cryptocurrency.
</p>

Built with [ratatui](https://ratatui.rs). Connects to Blocknet via its REST API and can auto-start an embedded daemon on localhost.

## Views

### Dashboard (`1`)

Chain stats, wallet balance, mempool sparklines with history, and mining controls with a plasma visualizer that reacts to hashrate. Shockwave animation on block discovery.

### Grid (`2`)

Top-down block field showing the last 500 blocks. Each block is color-coded by transaction count (white = empty, green = busy). Selected block is pulled out and rendered as a spinning 3D wireframe cube whose rotation speed reflects how fast it was mined relative to the 5-minute target. Row gutter shows block heights for orientation.

## Keybindings

| Key | Action |
|-----|--------|
| `1` | Dashboard view |
| `2` | Grid view |
| `j` / `k` | Navigate blocks (newer / older) |
| `J` / `K` | Jump one row (newer / older) |
| `m` | Toggle mining |
| `+` / `-` | Adjust mining threads |
| `q` | Quit |

## Requirements

- For local default use: no manual daemon setup required (embedded daemon autostarts).
- For remote/custom setups: reachable Blocknet API + matching cookie file.

## Embedded daemon binaries

`bntui` embeds every file placed under `binaries/` at build time and picks the best match for the current OS/arch at runtime.

## Install

### Homebrew (macOS)

```bash
brew install obselate/tap/bntui
```

### AUR (Arch Linux)

```bash
yay -S bntui
```

### Chocolatey (Windows)

```powershell
choco install bntui
```

### Cargo (any platform)

```bash
cargo install bntui
```

### From release binaries

Download the latest binary for your platform from [Releases](https://github.com/obselate/bntui/releases).

```bash
chmod +x bntui
./bntui
```

### From source

Requires Rust 1.85+.

```bash
git clone https://github.com/obselate/bntui.git
cd bntui
cargo build --release
./target/release/bntui
```

## Usage

`bntui` can run zero-config locally. If no cookie/daemon is found and you use localhost:8332, it auto-starts the embedded daemon with `--api --daemon`:

```bash
# auto-detect (checks cwd, then platform default)
bntui
```

bntui searches for `data/api.cookie` in the following order:

1. **Explicit argument** or `BLOCKNET_DIR` env var
2. **Existing cookie directory** (`./data/api.cookie`, then platform default)
3. **Platform default path** (created if missing):
   - macOS: `~/Library/Application Support/Blocknet`
   - Linux: `~/.blocknet`
   - Windows: `%APPDATA%\Blocknet`

```
$ bntui --help
Terminal block explorer for Blocknet privacy blockchain

Usage: bntui [OPTIONS] [BLOCKNET_DIR]

Arguments:
  [BLOCKNET_DIR]  Path to blocknet directory [auto-detected if omitted]

Options:
      --host <HOST>      API host to connect to [default: localhost]
      --port <PORT>      API port to connect to [default: 8332]
      --cookie <COOKIE>  Path to API cookie file (default: {blocknet_dir}/data/api.cookie)
  -h, --help             Print help
  -V, --version          Print version
```

```bash
# Optional: pass the directory explicitly
bntui /path/to/blocknet

# Or set the environment variable
export BLOCKNET_DIR=/path/to/blocknet
bntui

# Connect to a remote daemon
bntui --host 192.168.1.100 --port 8332 --cookie /path/to/api.cookie

# Disable embedded daemon autostart (debug/manual mode)
BNTUI_SKIP_EMBEDDED_DAEMON=1 bntui
```

### Docker

If running Blocknet in Docker, make sure the data directory is bind-mounted:

```yaml
volumes:
  - ./data:/data
```

Then point bntui at the directory containing the `data/` folder.

## API Endpoints Used

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/status` | GET | Chain height, peers, sync state |
| `/api/mempool` | GET | Mempool stats |
| `/api/wallet/balance` | GET | Wallet balance |
| `/api/mining` | GET | Mining status and hashrate |
| `/api/mining/start` | POST | Start mining |
| `/api/mining/stop` | POST | Stop mining |
| `/api/mining/threads` | POST | Set thread count |
| `/api/block/{height}` | GET | Block data by height |

All endpoints require Bearer token authentication via the cookie file.

## License

MIT
