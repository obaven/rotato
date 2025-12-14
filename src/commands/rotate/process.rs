use anyhow::Result;
use crate::models::{SecretDefinition};

use super::types::RotateArgs;
use super::hooks::execute_hooks;
use super::lookup::resolve_cipher_id;
use super::item_ops::{prepare_updated_item, check_rotation_needed};
use super::collections::{handle_folder, handle_collections};
use super::kube::generate_and_write_secret;

use super::authentik::generate_blueprint; // [NEW]

pub async fn process_secret(
    client: &crate::vaultwarden::VaultwardenClient,
    secret: SecretDefinition,
    git_root: &str,
    org_key: &[u8],
    user_key: &[u8],
    org_id: &str,
    args: &RotateArgs,
) -> Result<()> {
    // 0. Pre-Rotation Hooks
    if let Some(hooks) = &secret.hooks {
        if let Some(pre_hooks) = &hooks.pre {
            println!("  Executing PRE-rotation hooks...");
            let empty_data = std::collections::HashMap::new();
            execute_hooks(pre_hooks, &git_root, args.dry_run, &empty_data)?;
        }
    }

    println!("Rotating secret: {}/{}", secret.kubernetes.namespace, secret.name);
    
    // 1. Resolve Cipher ID
    let cipher_id = resolve_cipher_id(client, &secret, org_key, args.debug).await?;

    // 2. Fetch Item
    let mut item = match client.get_item(&cipher_id).await {
        Ok(i) => i,
        Err(e) => return Err(anyhow::anyhow!("Failed to get item by ID {}: {}", cipher_id, e)),
    };
        
    println!("  Fetched item '{}' ({})", item["name"].as_str().unwrap_or("?"), item["id"].as_str().unwrap_or("?"));

    // 3. Check policy before mutating secrets
    let should_rotate = check_rotation_needed(&item, args.force);
    if !should_rotate {
        println!("  Skipping rotation due to policy (no Vaultwarden/K8s updates or hooks).");
        return Ok(());
    }

    // 4. Prepare Updated Item (Resolve values, encrypt, update JSON)
    let secret_data = prepare_updated_item(
        &mut item, 
        &secret, 
        git_root, 
        org_key, 
        args.debug, 
        args.force
    ).await?;



    // 5. Handle Collections & Folders
    handle_folder(client, &mut item, &secret, user_key).await?;
    let collection_ids_to_update = handle_collections(client, &mut item, &secret, org_id, org_key).await?;
    
    // 6. Push to Vaultwarden
    if should_rotate {
        let item_id = item["id"].as_str().unwrap().to_string();
        if !args.dry_run {
            println!("  Updating Vaultwarden item {}...", item_id);
            client.update_item(&item_id, &item).await?;
            
            if let Some(ids) = collection_ids_to_update {
                if !ids.is_empty() {
                    println!("  Updating collection assignments ({} collections)...", ids.len());
                    client.update_collections(&item_id, &ids).await?;
                }
            }
        } else {
            println!("  [Dry Run] Would update Vaultwarden item {}", item_id);
        }
    } else {
        println!("  Skipping Vaultwarden update due to policy (updating local files only).");
    }

    // 7. Write Kubernetes Secret
    // 7. Write Kubernetes Secret
    generate_and_write_secret(&secret.kubernetes, &secret_data, git_root, args.dry_run, args.debug)?;
    
    if let Some(additional) = &secret.additional_kubernetes {
        for target in additional {
            generate_and_write_secret(target, &secret_data, git_root, args.dry_run, args.debug)?;
        }
    }
    
    // 8. Post-Rotation Hooks
    if let Some(hooks) = &secret.hooks {
        if let Some(post_hooks) = &hooks.post {
            println!("  Executing POST-rotation hooks...");
            execute_hooks(post_hooks, &git_root, args.dry_run, &secret_data)?;
        }
    }

    // 6. Generate Authentik Blueprint (if configured)
    if let Some(authentik_target) = &secret.authentik {
         // Assuming we want to use the "password" key for the secret field
         // If a specific key is needed, we might need to extend the config.
         // For now, we use the first password-like key found in the resolved map.
         
         // We need to resolve which value to put in the blueprint. 
         // Strategy: Look for "client_secret" in fields, or fallback to the generated "password" or "value" from keys.
         
         // In standard rotation, we have `new_values: HashMap<String, String>` (generated keys)
         // And `final_item: cipher data`.
         
         // Let's use `new_values` for simplicity. 
         // Typically the rotated value is "password" or "client_secret".
         
         let secret_val = secret_data.get("client_secret")
            .or_else(|| secret_data.get("password"))
            .or_else(|| secret_data.get("secret"));

         if let Some(val) = secret_val {
             generate_blueprint(authentik_target, val)?;
         } else {
             println!("WARNING: Authentik target defined but no suitable secret value (client_secret, password, secret) found in generated keys.");
         }
    }

    println!("Rotation complete for {}", secret.name);
    
    Ok(())
}
