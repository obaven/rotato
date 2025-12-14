use anyhow::{Result};
use super::types::{SecretConfig};
use crate::vaultwarden::{VaultwardenClient, Member, Collection};

use super::builder::build_payload;
use super::existence::check_exists_and_update_folder;
use super::access::grant_collection_access;

pub async fn process_secret(
    secret: &mut SecretConfig,
    client: &VaultwardenClient,
    org_id: &str,
    org_key: &[u8],
    user_key: &[u8],
    all_collections: &[Collection],
    all_members: &[Member],
    cli: Option<&crate::bw_cli::BwCli>
) -> Result<bool> {
    // Returns true if modified (i.e. cipher created/updated in config)
    
    // Scoped block to extract Values needed for Checks BEFORE mutating or holding mutable ref
    let (folder_name, collection_inputs, notes) = {
        let vw_cfg = match &secret.vaultwarden {
            Some(c) => c,
            None => return Ok(false),
        };
        (
            vw_cfg.folder.clone(),
            vw_cfg.collection_ids.clone().or(vw_cfg.collections.clone()),
            vw_cfg.notes.clone()
        )
    };

    // 1. Resolve Folder ID
    let folder_id = if let Some(name) = folder_name {
         match client.resolve_folder_id(&name, &user_key, true).await? {
             Some(id) => Some(id),
             None => {
                 println!("Failed to create/resolve folder '{}'", name);
                 None
             }
         }
    } else {
        None
    };

    // 2. Resolve Collections
    // 2. Resolve Collections (Create if missing)
    let collection_ids = match collection_inputs {
        Some(inputs) => {
            let mut ids = Vec::new();
            for identifier in inputs {
                let resolved = crate::logic::resolution::resolve_collection_ids(&[identifier.clone()], &all_collections, &org_key);
                if !resolved.is_empty() {
                    ids.extend(resolved);
                } else {
                    println!("Collection '{}' not found. Creating...", identifier);
                    match client.create_collection(org_id, &identifier).await {
                        Ok(new_id) => {
                             println!("    Created collection '{}' (ID: {})", identifier, new_id);
                             ids.push(new_id);
                        },
                        Err(e) => {
                             println!("    HTTP create failed: {}. Trying BW CLI if available...", e);
                             if let Some(cli_ref) = cli {
                                 match cli_ref.create_org_collection(&identifier) {
                                     Ok(cid) => {
                                         println!("    Created collection via CLI: '{}' (ID: {})", identifier, cid);
                                         ids.push(cid);
                                     },
                                     Err(ec) => println!("    CLI create failed too: {}", ec),
                                 }
                             } else {
                                 println!("    CLI not available to retry creation.");
                             }
                        }
                    }
                }
            }
            if ids.is_empty() {
                println!("Skipping {}: Failed to resolve or create any collections", secret.name);
                return Ok(false);
            }
            ids
        },
        None => {
            println!("Skipping {}: No collectionIds or collections", secret.name);
            return Ok(false);
        }
    };

    // 3. Access Control
    if let Some(target_emails) = &secret.access_users {
        grant_collection_access(target_emails, all_members, &collection_ids, org_id)?;
    }

    // 4. Check Existence
    // Pass immutable ref here
    if check_exists_and_update_folder(secret, client, folder_id.as_ref()).await? {
        return Ok(false);
    }
    
    // If we are here, we need to create it.
    // NOW we can mutate
    if let Some(vw_cfg) = &mut secret.vaultwarden {
         vw_cfg.cipher_id = None;
    }

    // 5. Build Payload
    // Requires immutable ref again
    let payload = build_payload(
        secret, 
        org_id, 
        org_key, 
        &collection_ids, 
        folder_id, 
        notes
    )?;

    // 6. Create Cipher
    match client.create_cipher(&payload).await {
        Ok(id) => {
            println!("Created cipher: {}", id);
             if let Some(vw_cfg) = &mut secret.vaultwarden {
                vw_cfg.cipher_id = Some(id);
             }
            Ok(true)
        },
        Err(e) => {
            println!("Failed to create cipher: {}", e);
            Ok(false)
        }
    }
}
