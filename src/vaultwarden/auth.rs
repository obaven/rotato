use anyhow::{anyhow, Result};
use super::client::VaultwardenClient;
use super::models::KdfInfo;

impl VaultwardenClient {
    pub async fn prelogin(&self, email: &str) -> Result<KdfInfo> {
        let url = format!("{}/api/accounts/prelogin", self.base_url);
        let resp = self.client.post(&url)
            .json(&serde_json::json!({ "email": email }))
            .send()
            .await?;

        if !resp.status().is_success() {
             return Err(anyhow!("Prelogin failed: {}", resp.status()));
        }

        let info: KdfInfo = resp.json().await?;
        Ok(info)
    }

    pub async fn login_password(&mut self, email: &str, master_password_hash: &str) -> Result<()> {
        let url = format!("{}/identity/connect/token", self.base_url);
        let params = [
            ("grant_type", "password"),
            ("username", email),
            ("password", master_password_hash),
            ("scope", "api offline_access"),
            ("client_id", "web"),
            ("device_type", "2"), // 2 = Browser usually
            ("device_identifier", "rotator-helper"), 
            ("device_name", "rotator-helper"),
        ];

        let resp = self.client.post(&url)
            .form(&params)
            .send()
            .await?;

        if !resp.status().is_success() {
             return Err(anyhow!("Login failed: {}", resp.status()));
        }

        let json: serde_json::Value = resp.json().await?;
        if let Some(token) = json["access_token"].as_str() {
            self.token = Some(token.to_string());
            Ok(())
        } else {
            Err(anyhow!("No access_token in login response"))
        }
    }

    pub async fn register(&self, _email: &str, _master_password_hash: &str, _key_enc: &str) -> Result<()> {
        // Implementation omitted for brevity as it was just a stub or rarely used
         Err(anyhow!("Register not implemented"))
    }
}
