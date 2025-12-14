use anyhow::{anyhow, Result};
use crate::vaultwarden::VaultwardenClient;
use crate::models::SecretDefinition;

pub async fn resolve_cipher_id(
    client: &VaultwardenClient,
    secret: &SecretDefinition,
    org_key: &[u8],
    debug_mode: bool,
) -> Result<String> {
    if let Some(id) = &secret.vaultwarden.cipher_id {
        if id != "00000000-0000-0000-0000-000000000000" {
            return Ok(id.clone());
        }
    }

    // Dynamic Lookup
    let target_name = secret.vaultwarden.name.as_ref().unwrap_or(&secret.name);
    println!("  Looking up Cipher ID for '{}'...", target_name);

    let sync_data = client.sync().await?;
    
    if let Some(ciphers) = sync_data.ciphers {
        for cipher in ciphers {
            let name_enc = cipher.name;
            let name_enc_clean = if name_enc.starts_with("2.") { &name_enc[2..] } else { &name_enc };
            
            // Try HMAC
            if let Ok(pt) = crate::crypto::decrypt_aes256_cbc_hmac(name_enc_clean, org_key) {
                if let Ok(s) = String::from_utf8(pt) {
                    if debug_mode { println!("    DEBUG: Found cipher: {}", s); }
                    if s == *target_name {
                        return Ok(cipher.id);
                    }
                }
            } 
            // Try Raw
            else if let Ok(pt) = crate::crypto::decrypt_aes256_cbc_raw(name_enc_clean, org_key) {
                 if let Ok(s) = String::from_utf8(pt) {
                     if s == *target_name {
                        return Ok(cipher.id);
                    }
                }
            }
        }
    }
    
    Err(anyhow!("Could not find cipher with name '{}' in Vaultwarden", target_name))
}
