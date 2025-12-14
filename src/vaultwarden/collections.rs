use anyhow::{anyhow, Result};
use super::client::VaultwardenClient;
use super::models::{Collection, Member};

impl VaultwardenClient {
    pub async fn list_collections(&self, org_id: &str) -> Result<Vec<Collection>> {
        let url = format!("{}/api/organizations/{}/collections", self.base_url, org_id);
        let resp = self.client.get(&url)
            .header("Authorization", format!("Bearer {}", self.token.as_ref().unwrap_or(&"".to_string())))
            .send()
            .await?;

        if !resp.status().is_success() {
             return Err(anyhow!("List collections failed: {}", resp.status()));
        }

        let json: serde_json::Value = resp.json().await?;
        if let Some(arr) = json["data"].as_array() {
             let cols: Vec<Collection> = serde_json::from_value(serde_json::Value::Array(arr.clone()))?;
             Ok(cols)
        } else {
             Ok(vec![])
        }
    }

    pub async fn create_collection(&self, org_id: &str, name: &str) -> Result<String> {
        let url = format!("{}/api/organizations/{}/collections", self.base_url, org_id);
        
        let payload = serde_json::json!({
            "name": name,
            "organizationId": org_id,
            "externalId": null
        });

        let resp = self.client.post(&url)
             .header("Authorization", format!("Bearer {}", self.token.as_ref().unwrap_or(&"".to_string())))
             .json(&payload)
             .send()
             .await?;
             
        if !resp.status().is_success() {
             return Err(anyhow!("Create collection failed: {}", resp.status()));
        }
        
        let json: serde_json::Value = resp.json().await?;
        // Return the new ID
        if let Some(id) = json["id"].as_str() {
             Ok(id.to_string())
        } else {
             Err(anyhow!("Created collection but no ID returned"))
        }
    }

    pub async fn update_collections(&self, item_id: &str, collection_ids: &[String]) -> Result<()> {
        let url = format!("{}/api/ciphers/{}/collections", self.base_url, item_id);
        
        let payload = serde_json::json!({
            "collectionIds": collection_ids
        });

        // Use PUT or POST? Typical is PUT /ciphers/{id}/collections or PUT /ciphers/{id}/collections-admin
        
        let resp = self.client.put(&url)
             .header("Authorization", format!("Bearer {}", self.token.as_ref().unwrap_or(&"".to_string())))
             .json(&payload)
             .send()
             .await?;
             
        if !resp.status().is_success() {
             return Err(anyhow!("Update collections failed: {}", resp.status()));
        }
        Ok(())
    }
    
    pub async fn list_members(&self, org_id: &str) -> Result<Vec<Member>> {
        let url = format!("{}/api/organizations/{}/members", self.base_url, org_id);
        let resp = self.client.get(&url)
            .header("Authorization", format!("Bearer {}", self.token.as_ref().unwrap_or(&"".to_string())))
            .send()
            .await?;
            
        if resp.status().as_u16() == 404 {
            // Some self-hosted Vaultwarden setups do not expose the members endpoint; treat as empty.
            println!("WARNING: Members endpoint returned 404 for org {} (continuing without member resolution)", org_id);
            return Ok(vec![]);
        }

        if !resp.status().is_success() {
             return Err(anyhow!("List members failed: {}", resp.status()));
        }
        
        let json: serde_json::Value = resp.json().await?;
        if let Some(arr) = json["data"].as_array() {
            let members: Vec<Member> = serde_json::from_value(serde_json::Value::Array(arr.clone()))?;
            Ok(members)
        } else {
            Ok(vec![])
        }
    }
    
    // Using BW CLI for groups listing now, so no need to add list_groups here unless we fix API call.
}
