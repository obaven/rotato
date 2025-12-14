use anyhow::{Result, anyhow};
use std::collections::HashMap;
use crate::vaultwarden::Member;

pub fn grant_collection_access(
    target_emails: &[String], 
    all_members: &[Member], 
    collection_ids: &[String],
    org_id: &str
) -> Result<()> {
    println!("[Access Control] Processing {} users", target_emails.len());
    
    // Map emails to member IDs
    let mut target_member_ids = HashMap::new();
    for email in target_emails {
        if let Some(m) = all_members.iter().find(|m| {
             m.email.as_deref() == Some(email) || 
             m.name.as_deref() == Some(email) ||
             Some(m.id.as_str()) == Some(email)
        }) {
            target_member_ids.insert(email.clone(), m.id.clone());
        } else {
             println!("    [Warning] User {} not found in organization members.", email);
        }
    }
    
    // Initialize BW CLI Wrapper if we have targets
    if !target_member_ids.is_empty() {
        println!("[Access Control] Initializing BW CLI Wrapper...");
        
        let session_key = std::env::var("BW_SESSION").ok();
        let cli_result = if let Some(key) = session_key {
            println!("    Using existing BW_SESSION from environment.");
            Ok(crate::bw_cli::BwCli::new(key, None))
        } else {
             let (email, password_opt) = crate::flows::resolve_credentials()?;
             if let Some(password) = password_opt {
                 crate::bw_cli::BwCli::unlock(&email, &password)
             } else {
                 Err(anyhow!("No password available"))
             }
        };

        match cli_result {
            Ok(mut bw) => {
                     bw.set_org_id(org_id.to_string());
                     
                     for col_id in collection_ids {
                         println!("    Checking collection {} via CLI...", col_id);
                         match bw.get_org_collection(col_id) {
                             Ok(mut collection) => {
                                 let mut changed = false;
                                 
                                 // Ensure "users" array exists. Note: collection is serde_json::Value
                                 if collection.get("users").is_none() || !collection["users"].is_array() {
                                     collection["users"] = serde_json::json!([]);
                                 }
        
                                 if let Some(users) = collection["users"].as_array_mut() {
                                    for (email, mem_id) in &target_member_ids {
                                        // Find existing user entry index
                                        let existing_idx = users.iter().position(|u| u["id"].as_str() == Some(mem_id));
                                        
                                        if let Some(idx) = existing_idx {
                                            // Update existing
                                            let user_obj = &mut users[idx];
                                            let current_hide = user_obj["hidePasswords"].as_bool().unwrap_or(true);
                                            if current_hide {
                                                println!("    Updating access for user {} (unhiding passwords) in collection {}", email, col_id);
                                                user_obj["hidePasswords"] = serde_json::json!(false);
                                                changed = true;
                                            }
                                        } else {
                                            // Add new
                                            println!("    Granting access to collection {} for user {}", col_id, email);
                                            users.push(serde_json::json!({
                                                "id": mem_id,
                                                "readOnly": false,
                                                "hidePasswords": false,
                                                "manage": false
                                            }));
                                            changed = true;
                                        }
                                    }
                                 }
        
                                 if changed {
                                     println!("    Encoding and updating collection {}...", col_id);
                                     match bw.encode(&collection) {
                                         Ok(encoded) => {
                                             if let Err(e) = bw.edit_org_collection(col_id, &encoded) {
                                                 println!("    [Error] Failed to update collection via CLI: {}", e);
                                             } else {
                                                 println!("    Successfully updated collection {} via CLI.", col_id);
                                             }
                                         },
                                         Err(e) => println!("    [Error] Failed to encode collection JSON: {}", e),
                                     }
                                 } else {
                                     println!("    No changes needed for collection {}.", col_id);
                                 }
                             },
                             Err(e) => println!("    [Error] Failed to get org-collection via CLI: {}", e),
                         }
                     }
                 },
                 Err(e) => println!("[Error] Failed to unlock BW CLI: {}", e),
             }
    }
    Ok(())
}
