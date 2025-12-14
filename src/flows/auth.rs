use anyhow::{anyhow, Result};
use crate::vaultwarden::VaultwardenClient;
use crate::crypto::{derive_master_key_pbkdf2, stretch_key_hkdf, decrypt_aes256_cbc_hmac};
use base64::{Engine as _, engine::general_purpose};


use crate::infra::k8s::get_k8s_secret;

fn strip_cipher_prefix(s: &str) -> &str {
    let bytes = s.as_bytes();
    if bytes.len() > 2 && bytes[1] == b'.' && bytes[0].is_ascii_digit() {
        &s[2..]
    } else {
        s
    }
}

/// Helper to resolve credentials from Env -> K8s -> Prompt
pub fn resolve_credentials() -> Result<(String, Option<String>)> {
    let email = std::env::var("BW_EMAIL").ok();
    let password = std::env::var("BW_PASSWORD").ok();

    if let (Some(e), Some(p)) = (email.clone(), password.clone()) {
        return Ok((e, Some(p)));
    }

    // Try K8s
    if let (Ok(k8s_user), Ok(k8s_pass)) = (
        get_k8s_secret("vaultwarden-admin-user", "vaultwarden-prod", "username"),
        get_k8s_secret("vaultwarden-admin-user", "vaultwarden-prod", "password")
    ) {
         println!("Fetched credentials from Kubernetes Secret (vaultwarden-admin-user)");
         return Ok((k8s_user, Some(k8s_pass)));
    }

    // Fallback to Env email + Interactive Password or Prompt for both
    if let Some(e) = email {
         return Ok((e, None));
    }

    // Interactive
    Err(anyhow!("Credentials not found in Env (BW_EMAIL/BW_PASSWORD) or Kubernetes (vaultwarden-test-user)"))
}

pub async fn get_org_key(
    base_url: &str,
    email: &str,
    session_key_b64: Option<String>,
    password_override: Option<String>,
    org_id_filter: Option<&str>,
) -> Result<(VaultwardenClient, String, Vec<u8>, Vec<u8>)> { // Client, OrgId, OrgKey, UserKey
    
    let mut client = VaultwardenClient::new(base_url);
    let debug_auth = std::env::var("ROTATOR_DEBUG_AUTH").is_ok();

    // 1. Prelogin
    println!("Fetching KDF info...");
    let kdf_info = client.prelogin(email).await?;
    if debug_auth {
        println!(
            "[auth-debug] kdf={} iterations={} memory={:?} parallelism={:?}",
            kdf_info.kdf, kdf_info.kdf_iterations, kdf_info.kdf_memory, kdf_info.kdf_parallelism
        );
    }

    // 2. Get Password
    let password = if let Some(p) = password_override {
        p
    } else {
        rpassword::prompt_password("Master Password: ")?
    };

    // 3. Derive Keys
    println!("Deriving Master Key for login...");
    let master_key = match kdf_info.kdf {
        0 => derive_master_key_pbkdf2(password.as_bytes(), email.to_lowercase().as_bytes(), kdf_info.kdf_iterations),
        1 => {
            let memory = kdf_info.kdf_memory.unwrap_or(64); // Default 64MB
            let parallelism = kdf_info.kdf_parallelism.unwrap_or(4); // Default 4 threads
            crate::crypto::derive_master_key_argon2id(password.as_bytes(), email.to_lowercase().as_bytes(), kdf_info.kdf_iterations, memory, parallelism)?
        },
        _ => return Err(anyhow!("Unknown KDF type: {}", kdf_info.kdf)),
    };
    
    // Derive Master Password Hash
    let master_password_hash_bytes = derive_master_key_pbkdf2(&master_key, password.as_bytes(), 1);
    let master_password_hash = general_purpose::STANDARD.encode(&master_password_hash_bytes);

    // 4. Login
    println!("Logging in...");
    client.login_password(email, &master_password_hash).await?;

    // 5. Sync
    println!("Syncing...");
    let sync_data = client.sync().await?;
    
    // 6. Decrypt Profile Key
    let mut candidate_keys = Vec::new();
    
    if let Some(sk) = session_key_b64 {
        let sk_bytes = general_purpose::STANDARD.decode(sk)?;
        candidate_keys.push(("Provided Session Key", sk_bytes));
    }
    
    let stretched_key = stretch_key_hkdf(&master_key);
    candidate_keys.push(("Derived Stretched Key", stretched_key));
    
    let stretched_from_hash = stretch_key_hkdf(&master_password_hash_bytes);
    candidate_keys.push(("HKDF from Master Password Hash", stretched_from_hash));
    candidate_keys.push(("Master Password Hash (Raw)", master_password_hash_bytes.clone()));
    candidate_keys.push(("Derived Master Key (Legacy)", master_key.clone()));

    let profile_key_enc = sync_data.profile.key;
    let profile_key_enc_clean = strip_cipher_prefix(&profile_key_enc);
    if debug_auth {
        let parts: Vec<&str> = profile_key_enc_clean.split('|').collect();
        let part_lens: Vec<_> = parts.iter().map(|p| p.len()).collect();
        println!(
            "[auth-debug] profile_key: len={} prefix='{}' contains_pipe={} parts={:?}",
            profile_key_enc.len(),
            profile_key_enc.chars().next().unwrap_or('?'),
            profile_key_enc.contains('|'),
            part_lens
        );
        if let Some(iv) = parts.get(0) {
            println!("[auth-debug] iv part='{}'", iv);
        }
        for (idx, part) in parts.iter().enumerate() {
            match general_purpose::STANDARD.decode(part) {
                Ok(bytes) => println!("[auth-debug] part {} base64 ok ({} bytes)", idx, bytes.len()),
                Err(e) => println!("[auth-debug] part {} base64 decode error: {}", idx, e),
            }
        }
    }
    println!("Decrypting User Symmetric Key...");
    let mut user_symmetric_key = Vec::new();
    for (name, key) in candidate_keys {
        if debug_auth {
            println!("[auth-debug] Trying {} (len {})", name, key.len());
        }
        match decrypt_aes256_cbc_hmac(profile_key_enc_clean, &key) {
            Ok(pt) => {
                println!("SUCCESS! Decrypted User Key with {}", name);
                user_symmetric_key = pt;
                break;
            },
            Err(e) => {
                if debug_auth {
                    println!("[auth-debug] {} decrypt failed: {}", name, e);
                }
                // Try raw
                match crate::crypto::decrypt_aes256_cbc_raw(profile_key_enc_clean, &key) {
                    Ok(pt) => {
                        println!("SUCCESS! (Raw) Decrypted User Key with {}", name);
                        user_symmetric_key = pt;
                        break;
                    },
                    Err(e) => {
                        if debug_auth {
                            println!("[auth-debug] {} failed: {}", name, e);
                        }
                    }
                }
            }
        }
    }

    if user_symmetric_key.is_empty() {
        return Err(anyhow!("Failed to decrypt User Symmetric Key"));
    }

    // 7. Find Org Key
    if let Some(orgs) = sync_data.profile.organizations {
        for org in orgs {
            // Filter if needed
            if let Some(filter) = org_id_filter {
                if org.id != filter && org.name != filter {
                    continue;
                }
            }
            
            if let Some(key_enc) = org.key {
                if key_enc.starts_with("4.") {
                    // RSA
                    if let Some(pk_enc) = sync_data.profile.private_key.as_ref() {
                         let pk_enc_clean = strip_cipher_prefix(pk_enc);
                         
                         let private_key_bytes = match decrypt_aes256_cbc_hmac(pk_enc_clean, &user_symmetric_key) {
                             Ok(pt) => pt,
                             Err(_) => crate::crypto::decrypt_aes256_cbc_raw(pk_enc_clean, &user_symmetric_key)?
                         };
                         
                         let rsa_cipher = strip_cipher_prefix(&key_enc);
                         match crate::crypto::decrypt_rsa_der(rsa_cipher, &private_key_bytes) {
                             Ok(org_key) => return Ok((client, org.id, org_key, user_symmetric_key)),
                             Err(e) => println!("Failed to decrypt RSA org key for {}: {}", org.name, e),
                         }

                    } else {
                        println!("No private key found for RSA org key");
                    }
                } else {
                    // Symmetric
                    let key_enc_clean = strip_cipher_prefix(&key_enc);
                    match decrypt_aes256_cbc_hmac(key_enc_clean, &user_symmetric_key) {
                        Ok(org_key) => return Ok((client, org.id, org_key, user_symmetric_key)),
                        Err(_) => {
                             if let Ok(org_key) = crate::crypto::decrypt_aes256_cbc_raw(key_enc_clean, &user_symmetric_key) {
                                 return Ok((client, org.id, org_key, user_symmetric_key));
                             }
                        }
                    }
                }
            }
        }
    }
    
    Err(anyhow!("Organization Key not found or decryption failed"))
}
