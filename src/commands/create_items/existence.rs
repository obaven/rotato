use anyhow::Result;
use crate::vaultwarden::VaultwardenClient;
use super::types::SecretConfig;

pub async fn check_exists_and_update_folder(
    secret: &SecretConfig, 
    client: &VaultwardenClient, 
    folder_id: Option<&String>
) -> Result<bool> {
    // Returns true if item exists
    if let Some(vw_cfg) = &secret.vaultwarden {
        if let Some(cipher_id) = &vw_cfg.cipher_id {
            if !cipher_id.is_empty() {
                // Check if it exists
                match client.get_item(cipher_id).await {
                    Ok(existing_item) => {
                        // Check if folder needs update
                        if let Some(fid) = folder_id {
                             let current_folder = existing_item["folderId"].as_str();
                             if current_folder != Some(fid.as_str()) {
                                 println!("Updating folder for {} from {:?} to {}", secret.name, current_folder, fid);
                                 
                                 let mut updated_item = existing_item.clone();
                                 updated_item["folderId"] = serde_json::Value::String(fid.clone());
                                 
                                 match client.update_item(cipher_id, &updated_item).await {
                                     Ok(_) => println!("Updated folder for {}", secret.name),
                                     Err(e) => println!("Failed to update folder for {}: {}", secret.name, e),
                                 }
                             }
                        }
                        
                        println!("Item {} already exists (ID: {}), checking next.", secret.name, cipher_id);
                        return Ok(true);
                    },
                    Err(_) => {
                        println!("Item {} has ID {} in config but not found in Vaultwarden. Recreating...", secret.name, cipher_id);
                        // Caller needs to handle resetting cipher_id if it's not present... 
                        // Actually since we pass &SecretConfig, we can't mutate it easily here to reset it.
                        // But returning false implies we proceed to creation.
                        return Ok(false);
                    }
                }
            }
        }
    }
    Ok(false)
}
