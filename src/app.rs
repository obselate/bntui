use crate::cube;
use crate::types;

pub enum InputMode {
    Normal,
    SendDialog {
        address: String,
        amount: String,
        focused: u8,
        error: Option<String>,
    },
}

pub struct FlashMessage {
    pub text: String,
    pub created: u64,
    pub persistent: bool,
    pub copyable: Option<String>,
}

pub struct App {
    pub current_view: u8,
    pub tick_count: u64,
    pub block_cubes: Vec<cube::SpinCube>,
    pub chain_blocks: Vec<types::BlockResponse>,
    pub selected: usize,
    pub grid_scroll_offset: usize,
    pub blocks_per_row: usize,
    pub status: Option<types::DaemonStats>,
    pub mempool: Option<types::MempoolStats>,
    pub balance: Option<types::BalanceResponse>,
    pub wallet_address: Option<String>,
    pub mining: Option<types::MiningStatus>,
    // plasma visualizer state
    pub plasma_t: f32,
    pub plasma_intensity: f32,
    pub prev_blocks_found: u64,
    pub shockwave_t: f32,
    // next block timer
    pub prev_chain_height: u64,
    pub block_found_display: f32,
    // mempool history for sparklines
    pub mempool_history: Vec<u64>,
    pub mempool_size_history: Vec<u64>,
    pub mempool_fee_history: Vec<u64>,
    pub threads_pending_restart: Option<u64>,
    pub flash_message: Option<FlashMessage>,
    pub input_mode: InputMode,
    pub tx_history: Vec<String>,
}

impl App {
    pub fn new() -> App {
        App {
            current_view: 1,
            tick_count: 0,
            block_cubes: vec![],
            chain_blocks: vec![],
            selected: 0,
            grid_scroll_offset: 0,
            blocks_per_row: 20,
            status: None,
            mempool: None,
            balance: None,
            wallet_address: None,
            mining: None,
            plasma_t: 0.0,
            plasma_intensity: 0.0,
            prev_blocks_found: 0,
            shockwave_t: -1.0,
            prev_chain_height: 0,
            block_found_display: 0.0,
            mempool_history: vec![],
            mempool_size_history: vec![],
            mempool_fee_history: vec![],
            threads_pending_restart: None,
            flash_message: None,
            input_mode: InputMode::Normal,
            tx_history: vec![],
        }
    }

    pub fn update_selected_cube(&mut self, spin_speed: f32) {
        if !self.block_cubes.is_empty() {
            self.block_cubes[self.selected].update(0.033 * spin_speed);
        }
    }

    pub fn selected_block_time(&self) -> f32 {
        if self.selected == 0 {
            return 300.0;
        }
        if let (Some(block), Some(prev)) = (
            self.chain_blocks.get(self.selected),
            self.chain_blocks.get(self.selected - 1),
        ) {
            block.timestamp.saturating_sub(prev.timestamp) as f32
        } else {
            300.0
        }
    }

    pub fn spin_speed(&self) -> f32 {
        let block_time = self.selected_block_time();
        if block_time <= 0.0 {
            return 3.0;
        }
        (300.0 / block_time).clamp(0.3, 3.0)
    }

    pub fn update_plasma(&mut self) {
        let is_mining = self.mining.as_ref().map_or(false, |m| m.running);
        let hashrate = self.mining.as_ref().map_or(0.0, |m| m.hashrate);
        let blocks_found = self.mining.as_ref().map_or(0, |m| m.blocks_found);

        // detect new block found → shockwave
        if blocks_found > self.prev_blocks_found && self.prev_blocks_found > 0 {
            self.shockwave_t = 0.0;
        }
        self.prev_blocks_found = blocks_found;

        // advance shockwave
        if self.shockwave_t >= 0.0 {
            self.shockwave_t += 0.08;
            if self.shockwave_t > 3.0 {
                self.shockwave_t = -1.0;
            }
        }

        // smooth intensity tracking
        let target = if is_mining && hashrate > 0.0 {
            ((hashrate as f32 / 3.0).sqrt()).clamp(0.2, 1.0)
        } else {
            0.0
        };
        self.plasma_intensity += (target - self.plasma_intensity) * 0.10;

        // advance time (speed scales with intensity)
        self.plasma_t += 0.04 + self.plasma_intensity * 0.08;
    }

    pub fn update_block_found(&mut self) {
        if self.block_found_display > 0.0 {
            self.block_found_display -= 0.033;
            if self.block_found_display <= 0.0 {
                self.block_found_display = 0.0;
            }
        }
    }

    pub fn record_mempool(&mut self, mempool: &types::MempoolStats) {
        self.mempool_history.push(mempool.count as u64);
        self.mempool_size_history.push(mempool.size_bytes);
        self.mempool_fee_history.push(mempool.avg_fee as u64);
        for h in [
            &mut self.mempool_history,
            &mut self.mempool_size_history,
            &mut self.mempool_fee_history,
        ] {
            if h.len() > 200 {
                h.drain(..h.len() - 200);
            }
        }
    }

    pub fn set_flash(&mut self, msg: String) {
        self.flash_message = Some(FlashMessage {
            text: msg,
            created: self.tick_count,
            persistent: false,
            copyable: None,
        });
    }

    pub fn set_flash_persistent(&mut self, msg: String, copyable: String) {
        self.flash_message = Some(FlashMessage {
            text: msg,
            created: self.tick_count,
            persistent: true,
            copyable: Some(copyable),
        });
    }

    pub fn log_tx(&mut self, txid: &str, address: &str, amount: u64) {
        self.tx_history.push(txid.to_string());
        if let Ok(home) = std::env::var("HOME") {
            let dir = std::path::PathBuf::from(home).join(".bntui");
            let _ = std::fs::create_dir_all(&dir);
            let log_path = dir.join("tx.log");
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path)
            {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let _ = writeln!(f, "{} {} {} {}", ts, txid, address, amount);
            }
        }
    }

    pub fn update_flash(&mut self) {
        if let Some(ref flash) = self.flash_message {
            if !flash.persistent && self.tick_count - flash.created > 90 {
                self.flash_message = None;
            }
        }
    }
}
