use anyhow::{anyhow, Result};
use reqwest::Client;
use std::time::Duration;

use super::models::{SyncData};

#[derive(Clone)]
pub struct VaultwardenClient {
    pub client: Client,
    pub base_url: String,
    pub token: Option<String>,
    pub debug_api: bool,
}

impl VaultwardenClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .cookie_store(true)
                .build()
                .unwrap(),
            base_url: base_url.to_string(),
            token: None,
            debug_api: false,
        }
    }

    pub async fn retry<F, T>(&self, operation: F) -> Result<T> 
    where F: std::future::Future<Output = Result<T>> {
        // Simple placeholder for retry logic
        // In real impl we would loop
         operation.await
    }

    pub async fn sync(&self) -> Result<SyncData> {
        let url = format!("{}/api/sync", self.base_url);
        let resp = self.client.get(&url)
            .header("Authorization", format!("Bearer {}", self.token.as_ref().unwrap_or(&"".to_string())))
            .send()
            .await?;

        if !resp.status().is_success() {
             let status = resp.status();
             let text = resp.text().await.unwrap_or_default();
             return Err(anyhow!("Sync failed: {} - {}", status, text));
        }

        let data: SyncData = resp.json().await?;
        Ok(data)
    }
}
