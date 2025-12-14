use anyhow::{anyhow, Result};
use crate::models::SecretDefinition;
use crate::models::KeyType;
use crate::logic::resolution::resolve_value;
use crate::logic::policy::should_rotate_by_policy;
use crate::logic::parsing::find_existing_encrypted_value;
use crate::infra::fs::get_file_value;
use crate::infra::k8s::get_k8s_secret;
use crate::infra::random::get_random_string;

use crate::crypto::encrypt_aes256_cbc_hmac;

pub async fn prepare_updated_item(
    item: &mut serde_json::Value,
    secret: &SecretDefinition,
    git_root: &str,
    org_key: &[u8],
    args_debug: bool,
    force: bool,
) -> Result<std::collections::HashMap<String, String>> {
    let mut secret_data = std::collections::HashMap::new();

    // Process each key
    for key_def in &secret.keys {
        let key = &key_def.name;
        
        let existing_val = if matches!(key_def.key_type, KeyType::Random) && !force {
             find_existing_encrypted_value(item, key, org_key)
        } else {
             None
        };

        let (value_to_use, _is_new, aux_values) = resolve_value(
            key_def,
            existing_val,
            |path, key_path| get_file_value(git_root, path, key_path),
            |k| get_k8s_secret(&secret.kubernetes.secret_name, &secret.kubernetes.namespace, k),
            get_random_string
        )?;

        if args_debug {
            println!("DEBUG: Plaintext for key '{}' is: {}", key, value_to_use);
        }
        secret_data.insert(key.clone(), value_to_use.clone());

        // Encrypt for Vaultwarden
        let encrypted_value = encrypt_aes256_cbc_hmac(value_to_use.as_bytes(), org_key)
            .map_err(|e| anyhow!("Failed to encrypt secret: {}", e))?;
        
        // Update Item JSON
        apply_key_to_item(item, key, &encrypted_value, org_key)?;
        
        // Handle auxiliary values (e.g. public key)
        if let Some(aux) = aux_values {
            for (aux_key, aux_val) in aux {
                 println!("    Found auxiliary value '{}' for key '{}'", aux_key, key);
                 // Add to text data for K8s secret
                 secret_data.insert(aux_key.clone(), aux_val.clone());
                 
                 // Encrypt and store in Vaultwarden fields
                 let enc_aux = encrypt_aes256_cbc_hmac(aux_val.as_bytes(), org_key)?;
                 apply_key_to_item(item, &aux_key, &enc_aux, org_key)?;
            }
        }
    }
    
    // Update Notes (Policy)
    let old_notes = item["notes"].as_str().unwrap_or("").to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let new_notes = if old_notes.contains("Last Rotated:") {
        let rest_of_notes = old_notes.lines()
            .filter(|l| !l.starts_with("Last Rotated:"))
            .collect::<Vec<_>>()
            .join("\n");
        format!("Last Rotated: {}\n{}", now, rest_of_notes)
    } else {
            format!("Last Rotated: {}\n{}", now, old_notes)
    };
    item["notes"] = serde_json::Value::String(new_notes);

    Ok(secret_data)
}

fn apply_key_to_item(item: &mut serde_json::Value, key: &str, encrypted_value: &str, org_key: &[u8]) -> Result<()> {
    if key == "password" || key == "username" {
        if item.get("login").is_none() {
            println!("    Initializing missing 'login' object for item.");
            item["login"] = serde_json::json!({});
        }
        if let Some(login) = item.get_mut("login") {
            login[key] = serde_json::Value::String(encrypted_value.to_string());
            println!("    Updated '{}.{}'", "login", key);
        }
    } else {
        // Find in fields
        let encrypted_name = encrypt_aes256_cbc_hmac(key.as_bytes(), org_key)?;
        let mut found = false;
        
        if let Some(fields) = item.get_mut("fields").and_then(|f| f.as_array_mut()) {
            for field in fields.iter_mut() {
                if let Some(n) = field["name"].as_str() {
                    // Decrypt name to check match
                    if let Ok(nb) = crate::crypto::decrypt_aes256_cbc_hmac(if n.starts_with("2.") { &n[2..] } else { n }, org_key) {
                        if String::from_utf8(nb).ok().as_deref() == Some(key) {
                            field["value"] = serde_json::Value::String(encrypted_value.to_string());
                            field["type"] = serde_json::json!(1); // Enforce Sensitive/Hidden
                            found = true;
                            println!("    Updated field '{}' (set to sensitive)", key);
                            break;
                        }
                    }
                }
            }
            if !found {
                println!("    Creating new field '{}' (sensitive)", key);
                fields.push(serde_json::json!({ "name": encrypted_name, "value": encrypted_value, "type": 1 }));
            }
        } else {
             println!("    Initializing 'fields' array and adding '{}' (sensitive)", key);
             item["fields"] = serde_json::json!([{ "name": encrypted_name, "value": encrypted_value, "type": 1 }]);
        }
    }
    Ok(())
}

pub fn check_rotation_needed(item: &serde_json::Value, force: bool) -> bool {
    if force { return true; }
    let notes = item["notes"].as_str().unwrap_or("");
    should_rotate_by_policy(notes, 30)
}
