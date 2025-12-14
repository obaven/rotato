use anyhow::{anyhow, Result};
use super::client::VaultwardenClient;

impl VaultwardenClient {
    pub async fn create_cipher(&self, data: &serde_json::Value) -> Result<String> {
        let url = format!("{}/api/ciphers", self.base_url);
        if self.debug_api { println!("DEBUG: POST {}", url); }
        
        let resp = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", self.token.as_ref().unwrap_or(&"".to_string())))
            .json(data)
            .send()
            .await?;

        if !resp.status().is_success() {
             return Err(anyhow!("Create cipher failed: {}", resp.status()));
        }

        let json: serde_json::Value = resp.json().await?;
        if let Some(id) = json["id"].as_str() {
            Ok(id.to_string())
        } else {
            Err(anyhow!("No id in create_cipher response"))
        }
    }

    pub async fn update_item(&self, item_id: &str, data: &serde_json::Value) -> Result<()> {
        let url = format!("{}/api/ciphers/{}", self.base_url, item_id);
        if self.debug_api { println!("DEBUG: PUT {}", url); }

        let resp = self.client.put(&url)
            .header("Authorization", format!("Bearer {}", self.token.as_ref().unwrap_or(&"".to_string())))
            .json(data)
            .send()
            .await?;

        if !resp.status().is_success() {
             return Err(anyhow!("Update item usage failed: {}", resp.status()));
        }

        Ok(())
    }

    pub async fn get_item(&self, item_id: &str) -> Result<serde_json::Value> {
        let url = format!("{}/api/ciphers/{}", self.base_url, item_id);
        let resp = self.client.get(&url)
            .header("Authorization", format!("Bearer {}", self.token.as_ref().unwrap_or(&"".to_string())))
            .send()
            .await?;

        if !resp.status().is_success() {
             return Err(anyhow!("Get item failed: {}", resp.status()));
        }
        
        let json: serde_json::Value = resp.json().await?;
        Ok(json)
    }

    pub async fn delete_item(&self, item_id: &str) -> Result<()> {
        let url = format!("{}/api/ciphers/{}", self.base_url, item_id);
        let resp = self.client.delete(&url)
            .header("Authorization", format!("Bearer {}", self.token.as_ref().unwrap_or(&"".to_string())))
            .send()
            .await?;
            
        if !resp.status().is_success() {
             return Err(anyhow!("Delete item failed: {}", resp.status()));
        }
        Ok(())
    }

    pub async fn delete_folder(&self, folder_id: &str) -> Result<()> {
        let url = format!("{}/api/folders/{}", self.base_url, folder_id);
         let resp = self.client.delete(&url)
            .header("Authorization", format!("Bearer {}", self.token.as_ref().unwrap_or(&"".to_string())))
            .send()
            .await?;
            
        if !resp.status().is_success() {
             return Err(anyhow!("Delete folder failed: {}", resp.status()));
        }
        Ok(())
    }
}
