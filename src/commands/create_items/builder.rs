use anyhow::{Result};
use std::collections::HashMap;
use crate::crypto::encrypt_aes256_cbc_hmac;
use crate::infra::k8s::get_k8s_secret;
use super::types::SecretConfig;

pub fn build_payload(
    secret: &SecretConfig,
    org_id: &str,
    org_key: &[u8],
    collection_ids: &[String],
    folder_id: Option<String>,
    notes: Option<String>,
) -> Result<serde_json::Value> {
    let _env_str = secret.env.as_deref().unwrap_or("prod");

    let label = if let Some(env) = &secret.env {
        let prefix = format!("{}-", env);
        let base_name = if secret.name.starts_with(&prefix) {
            &secret.name[prefix.len()..]
        } else {
            &secret.name
        };
        format!("{}/{}", base_name, env)
    } else {
        secret.name.clone()
    };
    
    println!("Creating item for {}", label);

    // Construct fields
    let mut fields = Vec::new();
    let keys_list: Vec<String> = if let Some(ks) = &secret.keys {
        ks.iter().map(|k| k.name().to_string()).collect()
    } else if let Some(sf) = &secret.source_files {
         sf.keys().cloned().collect()
    } else {
        Vec::new()
    };

    let mut fetched_values = HashMap::new();
    if let (Some(ns), Some(sn)) = (&secret.namespace, &secret.secret_name) {
         for k in &keys_list {
             if let Ok(val) = get_k8s_secret(sn, ns, k) {
                 println!("    Found value for key '{}' in K8s", k);
                 fetched_values.insert(k.clone(), val);
             }
         }
    }

    for k in keys_list {
        let val = fetched_values.get(&k).cloned().unwrap_or_default();
        
        fields.push(serde_json::json!({
            "name": format!("{}:{}:current", label, k),
            "value": val, 
            "type": 1
        }));
        fields.push(serde_json::json!({
            "name": format!("{}:{}:previous", label, k),
            "value": "",
            "type": 1
        }));
    }

    // Encrypt Name and Notes
    let name_enc = encrypt_aes256_cbc_hmac(label.as_bytes(), org_key)?;
    let notes_enc = encrypt_aes256_cbc_hmac(notes.unwrap_or_default().as_bytes(), org_key)?;

    let mut fields_enc = Vec::new();
    for f in fields {
        let name = f["name"].as_str().unwrap();
        let value = f["value"].as_str().unwrap();
        
        fields_enc.push(serde_json::json!({
            "name": encrypt_aes256_cbc_hmac(name.as_bytes(), org_key)?,
            "value": encrypt_aes256_cbc_hmac(value.as_bytes(), org_key)?,
            "type": 1
        }));
    }

    let username_val = fetched_values.get("username").cloned().unwrap_or_default();
    let password_val = fetched_values.get("password").cloned().unwrap_or_default();
    
    let username_enc = encrypt_aes256_cbc_hmac(username_val.as_bytes(), org_key)?;
    let password_enc = encrypt_aes256_cbc_hmac(password_val.as_bytes(), org_key)?;
    
    let login = serde_json::json!({
        "username": username_enc,
        "password": password_enc,
        "totp": null,
        "uri": null
    });

    Ok(serde_json::json!({
        "type": 1,
        "name": name_enc,
        "notes": notes_enc,
        "fields": fields_enc,
        "login": login,
        "organizationId": org_id,
        "collectionIds": collection_ids,
        "folderId": folder_id,
    }))
}
