<p align="center">
  <img src="assets/banner.svg" alt="BNTUI" width="600"/>
</p>

<p align="center">
  Terminal block explorer for <a href="https://github.com/blocknetprivacy/blocknet">Blocknet</a>, a privacy-focused cryptocurrency.
</p>

Built with [ratatui](https://ratatui.rs). Connects to a running Blocknet daemon via its REST API.

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

- A running Blocknet node with the API enabled (port 8332)
- API cookie file at `<blocknet-dir>/data/api.cookie`

## Install

### Homebrew (macOS)

```bash
brew install obselate/tap/bntui
```

### AUR (Arch Linux)

```bash
yay -S bntui
```

### Cargo (any platform)

```bash
cargo install bntui
```

### From release binaries

Download the latest binary for your platform from [Releases](https://github.com/obselate/bntui/releases).

```bash
chmod +x bntui
./bntui /path/to/blocknet
```

### From source

Requires Rust 1.85+.

```bash
git clone https://github.com/obselate/bntui.git
cd bntui
cargo build --release
./target/release/bntui /path/to/blocknet
```

## Usage

```bash
# Pass the blocknet directory as an argument
bntui /path/to/blocknet

# Or set the environment variable
export BLOCKNET_DIR=/path/to/blocknet
bntui
```

The blocknet directory should contain `data/api.cookie` (created by the daemon on startup).

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
