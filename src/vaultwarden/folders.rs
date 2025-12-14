use anyhow::{anyhow, Result};
use super::client::VaultwardenClient;
use super::models::{Folder, FolderData};

impl VaultwardenClient {
    pub async fn list_folders(&self) -> Result<Vec<FolderData>> {
         // Folders come from sync usually, but check if there is endpoint?
         // /api/folders exists
         let url = format!("{}/api/folders", self.base_url);
         let resp = self.client.get(&url)
            .header("Authorization", format!("Bearer {}", self.token.as_ref().unwrap_or(&"".to_string())))
            .send()
            .await?;

        if !resp.status().is_success() {
             return Err(anyhow!("List folders failed: {}", resp.status()));
        }

        let json: serde_json::Value = resp.json().await?;
        if let Some(arr) = json["data"].as_array() {
             let folders: Vec<FolderData> = serde_json::from_value(serde_json::Value::Array(arr.clone()))?;
             Ok(folders)
        } else {
             // Maybe API returns direct array? Bitwarden API usually returns { "data": [], "object": "list", "continuationToken": null }
             let folders: Vec<FolderData> = serde_json::from_value(json)?;
             Ok(folders)
        }
    }

    pub async fn create_folder(&self, name_enc: &str) -> Result<String> {
        let url = format!("{}/api/folders", self.base_url);
        let folder = Folder { name: name_enc.to_string() };
        
        let resp = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", self.token.as_ref().unwrap_or(&"".to_string())))
            .json(&folder)
            .send()
            .await?;

        if !resp.status().is_success() {
             return Err(anyhow!("Create folder failed: {}", resp.status()));
        }

        let json: serde_json::Value = resp.json().await?;
        if let Some(id) = json["id"].as_str() {
            Ok(id.to_string())
        } else {
            Err(anyhow!("No id in create_folder response"))
        }
    }

    pub async fn resolve_folder_id(&self, name: &str, org_key: &[u8], create_if_missing: bool) -> Result<Option<String>> {
        // Folders are personal, so we use User Key usually, passed as org_key argument here for genericcrypto usage?
        // Actually caller passes "user_key" to this method usually. Or "org_key" if shared folder?
        // Folders are personal in Bitwarden. 
        
        let folders = self.list_folders().await?;
        
        for f in &folders {
             let name_enc = &f.name;
             let name_enc_clean = if name_enc.starts_with("2.") { &name_enc[2..] } else { name_enc };
             
             if let Ok(pt) = crate::crypto::decrypt_aes256_cbc_hmac(name_enc_clean, org_key) {
                  if let Ok(s) = String::from_utf8(pt) {
                      if s == name {
                          return Ok(Some(f.id.clone()));
                      }
                  }
             }
        }
        
        if create_if_missing {
             println!("    Folder '{}' not found, creating...", name);
             let name_enc = crate::crypto::encrypt_aes256_cbc_hmac(name.as_bytes(), org_key)?;
             let id = self.create_folder(&name_enc).await?;
             Ok(Some(id))
        } else {
            Ok(None)
        }
    }
}
