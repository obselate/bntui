use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};

pub struct ApiClient {
    client: reqwest::Client,
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: &str, cookie_path: &str) -> Result<Self, String> {
        let token = std::fs::read_to_string(cookie_path)
            .map_err(|e| format!("can't read cookie: {}", e))?;

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", token.trim()))
                .map_err(|e| format!("bad token: {}", e))?,
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| format!("client build failed: {}", e))?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        })
    }

    pub async fn get_status(&self) -> Result<crate::types::DaemonStats, reqwest::Error> {
        self.client
            .get(format!("{}/api/status", self.base_url))
            .send()
            .await?
            .json()
            .await
    }

    pub async fn get_mempool(&self) -> Result<crate::types::MempoolStats, reqwest::Error> {
        self.client
            .get(format!("{}/api/mempool", self.base_url))
            .send()
            .await?
            .json()
            .await
    }

    pub async fn get_balance(&self) -> Result<crate::types::BalanceResponse, reqwest::Error> {
        self.client
            .get(format!("{}/api/wallet/balance", self.base_url))
            .send()
            .await?
            .json()
            .await
    }

    pub async fn get_address(&self) -> Result<crate::types::AddressResponse, reqwest::Error> {
        self.client
            .get(format!("{}/api/wallet/address", self.base_url))
            .send()
            .await?
            .json()
            .await
    }

    pub async fn get_mining(&self) -> Result<crate::types::MiningStatus, reqwest::Error> {
        self.client
            .get(format!("{}/api/mining", self.base_url))
            .send()
            .await?
            .json()
            .await
    }

    pub async fn start_mining(&self) -> Result<(), reqwest::Error> {
        self.client
            .post(format!("{}/api/mining/start", self.base_url))
            .send()
            .await?;
        Ok(())
    }

    pub async fn stop_mining(&self) -> Result<(), reqwest::Error> {
        self.client
            .post(format!("{}/api/mining/stop", self.base_url))
            .send()
            .await?;
        Ok(())
    }

    pub async fn set_threads(&self, threads: u32) -> Result<(), reqwest::Error> {
        self.client
            .post(format!("{}/api/mining/threads", self.base_url))
            .json(&serde_json::json!({"threads": threads}))
            .send()
            .await?;
        Ok(())
    }

    pub async fn send_to(&self, address: &str, amount: u64) -> Result<String, String> {
        let resp = self
            .client
            .post(format!("{}/api/wallet/send", self.base_url))
            .json(&serde_json::json!({
                "address": address,
                "amount": amount
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if status.is_success() {
            // Try to extract txid from JSON response
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                if let Some(txid) = json
                    .get("txid")
                    .or(json.get("hash"))
                    .and_then(|v| v.as_str())
                {
                    return Ok(txid.to_string());
                }
            }
            // Maybe bare txid string
            let trimmed = body.trim().trim_matches('"');
            if !trimmed.is_empty() && trimmed.len() <= 64 {
                Ok(trimmed.to_string())
            } else {
                Ok("sent".to_string())
            }
        } else {
            Err(if body.is_empty() {
                format!("Send failed (HTTP {})", status)
            } else {
                body
            })
        }
    }

    pub async fn get_block(
        &self,
        height: u64,
    ) -> Result<crate::types::BlockResponse, reqwest::Error> {
        self.client
            .get(format!("{}/api/block/{}", self.base_url, height))
            .send()
            .await?
            .json()
            .await
    }
}
