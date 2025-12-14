use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::Value;

pub struct AdminClient {
    client: Client,
    base_url: String,
    token: String,
}

impl AdminClient {
    pub fn new(base_url: String, token: String) -> Self {
        // Enable cookie store for session management
        let client = Client::builder()
            .cookie_store(true)
            .build()
            .unwrap_or_else(|_| Client::new());
            
        Self {
            client,
            base_url,
            token,
        }
    }

    pub async fn login(&self) -> Result<()> {
        let url = format!("{}/admin", self.base_url);
        let params = [("token", &self.token)];
        
        // Simple retry loop for 429s
        let mut retries = 0;
        loop {
            let resp = self.client.post(&url)
                .form(&params)
                .send()
                .await?;
                
            if resp.status().is_success() {
                break;
            } else if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                if retries >= 5 {
                    return Err(anyhow!("Failed to login to Admin API after retries: {}", resp.status()));
                }
                println!("    [Admin Client] Hit 429 Too Many Requests, waiting {}s...", (retries + 1) * 2);
                tokio::time::sleep(std::time::Duration::from_secs(((retries + 1) * 2) as u64)).await;
                retries += 1;
                continue;
            } else {
                 return Err(anyhow!("Failed to login to Admin API: {}", resp.status()));
            }
        }
        
        Ok(())
    }

    pub async fn list_users(&self) -> Result<Vec<Value>> {
        let url = format!("{}/admin/users", self.base_url);
        let resp = self.client.get(&url)
            // .header("Admin-Token", &self.token) // Not needed if cookie works
            .send()
            .await?;

        if !resp.status().is_success() {
             return Err(anyhow!("Failed to list users: {}", resp.status()));
        }
        
        // Vaultwarden returns list of users directly
        let json: Value = resp.json().await?;
        if let Some(arr) = json.as_array() {
            Ok(arr.clone())
        } else {
            Ok(vec![])
        }
    }

    pub async fn delete_user(&self, uuid: &str) -> Result<()> {
        // Try standard REST DELETE first
        let url = format!("{}/admin/users/{}", self.base_url, uuid);
        let resp = self.client.delete(&url)
            .send()
            .await?;
            
         if !resp.status().is_success() {
             return Err(anyhow!("Failed to delete user {}: {}", uuid, resp.status()));
        }
        Ok(())
    }
    
    pub async fn invite_user(&self, email: &str) -> Result<()> {
         let url = format!("{}/admin/invite", self.base_url);
         let resp = self.client.post(&url)
             .json(&serde_json::json!({ "email": email }))
             .send()
             .await?;
             
         if !resp.status().is_success() {
             return Err(anyhow!("Failed to invite user: {}", resp.status()));
         }
         Ok(())
    }
}
