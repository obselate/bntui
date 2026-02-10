use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct DaemonStats {
    pub peer_id: String,
    pub peers: u32,
    pub chain_height: u64,
    pub best_hash: String,
    pub total_work: u64,
    pub mempool_size: u32,
    pub mempool_bytes: u64,
    pub syncing: bool,
    #[serde(default)]
    pub sync_progress: u64,
    #[serde(default)]
    pub sync_target: u64,
    #[serde(default)]
    pub sync_percent: Option<String>,
    pub identity_age: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct MempoolStats {
    pub count: u32,
    pub size_bytes: u64,
    pub min_fee: u64,
    pub max_fee: u64,
    pub avg_fee: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct BalanceResponse {
    pub spendable: u64,
    pub pending: u64,
    pub total: u64,
    pub outputs_total: u32,
    pub outputs_unspent: u32,
    pub chain_height: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AddressResponse {
    pub address: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct MiningStatus {
    pub running: bool,
    pub threads: u32,
    #[serde(default)]
    pub hashrate: f64,
    #[serde(default)]
    pub hash_count: u64,
    #[serde(default)]
    pub blocks_found: u64,
    #[serde(default)]
    pub started_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlockTransaction {
    pub hash: String,
    pub fee: u64,
    pub inputs: u32,
    pub outputs: u32,
    pub is_coinbase: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct BlockResponse {
    pub height: u64,
    pub hash: String,
    pub timestamp: u64,
    pub difficulty: u64,
    pub tx_count: u32,
    pub confirmations: u64,
    pub reward: u64,
    pub transactions: Vec<BlockTransaction>,
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

pub fn parse_bnt_amount(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let parts: Vec<&str> = s.split('.').collect();
    match parts.len() {
        1 => {
            let whole: u64 = parts[0].parse().ok()?;
            Some(whole.checked_mul(100_000_000)?)
        }
        2 => {
            let whole: u64 = parts[0].parse().ok()?;
            let frac_raw = parts[1];
            if frac_raw.len() > 8 {
                return None;
            }
            let frac_str = format!("{:0<8}", frac_raw);
            let frac: u64 = frac_str.parse().ok()?;
            Some(whole.checked_mul(100_000_000)?.checked_add(frac)?)
        }
        _ => None,
    }
}
