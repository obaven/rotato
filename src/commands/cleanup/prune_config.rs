use anyhow::Result;
use std::io::Write;
use crate::vaultwarden::VaultwardenClient;
use super::types::Config;

pub async fn prune_from_config(config: Config, client: &VaultwardenClient, org_key: &[u8], user_key: &[u8]) -> Result<()> {
    println!("Processing {} secrets from config...", config.secrets.len());
        
    let mut folders_to_delete = std::collections::HashSet::new();

    for secret in config.secrets {
        if let Some(vw) = secret.vaultwarden {
            let target_id = if let Some(id) = vw.cipher_id {
                Some(id)
            } else {
                 let target_name = vw.name.clone().unwrap_or_else(|| {
                     if let Some(env) = &secret.env {
                         let prefix = format!("{}-", env);
                         let base_name = if secret.name.starts_with(&prefix) {
                             &secret.name[prefix.len()..]
                         } else {
                             &secret.name
                         };
                         format!("{}/{}", base_name, env)
                     } else {
                         secret.name.clone()
                     }
                 });

                 println!("Resolving ID for '{}'...", target_name);
                 
                 let sync_data = match client.sync().await {
                     Ok(d) => d,
                     Err(e) => {
                         println!("Failed to sync vault for lookup: {}", e);
                         continue;
                     }
                 };

                 let mut found = None;
                 if let Some(ciphers) = sync_data.ciphers {
                     for cipher in ciphers {
                        let name_enc = cipher.name;
                        let name_enc_clean = if name_enc.starts_with("2.") { &name_enc[2..] } else { &name_enc };
                        
                        // Try decrypt with HMAC
                        if let Ok(pt) = crate::crypto::decrypt_aes256_cbc_hmac(name_enc_clean, &org_key) {
                             if let Ok(s) = String::from_utf8(pt) {
                                 if s == target_name {
                                     found = Some(cipher.id);
                                     break;
                                 }
                             }
                        }
                        // Try raw (just in case)
                        else if let Ok(pt) = crate::crypto::decrypt_aes256_cbc_raw(name_enc_clean, &org_key) {
                             if let Ok(s) = String::from_utf8(pt) {
                                  if s == target_name {
                                     found = Some(cipher.id);
                                     break;
                                 }
                             }
                        }
                     }
                 }
                 found
            };

            if let Some(folder_name) = &vw.folder {
                 folders_to_delete.insert(folder_name.clone());
            }

            if let Some(id) = target_id {
                print!("Deleting item for {} ({}) ... ", secret.name, id);
                std::io::stdout().flush()?;
                
                match client.delete_item(&id).await {
                    Ok(_) => println!("OK"),
                    // If 404, consider it success
                    Err(e) => println!("Failed: {}", e),
                }
            } else {
                println!("Skipping {}: Could not resolve ID (Name not found or ID missing)", secret.name);
            }
        }
    }

    println!("Processing {} folders to delete from config...", folders_to_delete.len());
    for folder_name in folders_to_delete {
        print!("Resolving folder '{}'... ", folder_name);
        // Folders are personal, use user_key
         match client.resolve_folder_id(&folder_name, &user_key, false).await {
             Ok(Some(id)) => {
                 println!("Found ({})", id);
                 print!("Deleting folder '{}' ({}) ... ", folder_name, id);
                 std::io::stdout().flush()?;
                 match client.delete_folder(&id).await {
                     Ok(_) => println!("OK"),
                     Err(e) => println!("Failed: {}", e),
                 }
             },
             Ok(None) => println!("Not found."),
             Err(e) => println!("Error resolving: {}", e),
         }
    }

    Ok(())
}
