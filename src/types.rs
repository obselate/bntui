use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct DaemonStats {
    pub peers: u32,
    pub chain_height: u64,
    pub syncing: bool,
    #[serde(default)]
    pub sync_progress: u64,
    #[serde(default)]
    pub sync_target: u64,
    #[serde(default)]
    pub sync_percent: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MempoolStats {
    pub count: u32,
    pub size_bytes: u64,
    pub avg_fee: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BalanceResponse {
    pub spendable: u64,
    pub pending: u64,
    pub total: u64,
    pub outputs_unspent: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MiningStatus {
    pub running: bool,
    pub threads: u32,
    #[serde(default)]
    pub hashrate: f64,
    #[serde(default)]
    pub hash_count: u64,
    #[serde(default)]
    pub blocks_found: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlockResponse {
    pub height: u64,
    pub timestamp: u64,
    pub difficulty: u64,
    pub tx_count: u32,
    pub reward: u64,
}

pub fn format_time_ago(timestamp: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let diff = now.saturating_sub(timestamp);
    if diff < 60 {
        format!("{}s ago", diff)
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else {
        format!("{}d ago", diff / 86400)
    }
}

pub fn format_bnt(atomic: u64) -> String {
    let whole = atomic / 100_000_000;
    let frac = atomic % 100_000_000;
    if frac == 0 {
        format!("{}.0 BNT", whole)
    } else {
        let frac_str = format!("{:08}", frac);
        let trimmed = frac_str.trim_end_matches('0');
        format!("{}.{} BNT", whole, trimmed)
    }
}
