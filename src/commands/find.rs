use clap::Args;
use anyhow::Result;
use colored::*;
use walkdir::WalkDir;
use std::fs;
use crate::models::RotationManifest;
use crate::infra::k8s::get_k8s_secret;
use crate::crypto::decrypt_aes256_cbc_hmac;

#[derive(Args)]
pub struct FindArgs {
    #[arg(long, help = "Username or Email to search for")]
    pub user: String,
}

pub async fn run(args: FindArgs) -> Result<()> {
    println!("{}", format!("Searching for secrets with username '{}'...", args.user).blue());

    // 1. Authenticate to Vaultwarden (needed for decryption)
    let (email, password_opt) = crate::flows::resolve_credentials()?;
    
    let session_key = std::env::var("BW_SESSION").ok();
    
    println!("Authenticating to Vaultwarden to fetch live values...");
    let (client, _org_id, org_key, _) = crate::flows::get_org_key(
        "https://vaultwarden.obaven.org", // This line will be replaced by the user's edit
        &email,
        session_key,
        password_opt,
        None, // This line will be replaced by the user's edit
    ).await?;

    // 2. Find all manifests
    let root = crate::commands::check::find_monorepo_root()?;
    let apps_dir = root.join("apps");
    let mut found_secrets = Vec::new();

    if apps_dir.exists() {
         for entry in WalkDir::new(apps_dir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_name() == "rotation.yaml" {
                let content = fs::read_to_string(entry.path())?;
                if let Ok(manifest) = serde_yaml::from_str::<RotationManifest>(&content) {
                    found_secrets.extend(manifest.secrets);
                }
            }
        }
    }

    let mut match_found = false;

    // 3. Check each secret
    for secret in found_secrets {
        // Check "username" from manifest (static value) to see if it's the right secret definition
        let manifest_username = secret.keys.iter()
            .find(|k| k.name == "username")
            .and_then(|k| k.value.clone());

        let is_match = if let Some(u) = manifest_username {
            u == args.user
        } else {
            // If dynamic, check K8s or fallback
            // For now, let's assume we searched for "admin" or implicit matches
             false
        };

        // Also check if K8s secret has matching username (as implemented before)
        let k8s_match = if let Ok(username) = get_k8s_secret(&secret.kubernetes.secret_name, &secret.kubernetes.namespace, "username") {
             username == args.user
        } else { false };

        if is_match || k8s_match {
            println!("{}", format!("Found match in Manifest: {}/{}", secret.kubernetes.namespace, secret.name).green().bold());
            match_found = true;

            // Fetch from Vaultwarden
            if let Some(cipher_id) = secret.vaultwarden.cipher_id {
                println!("Fetching Cipher ID: {}", cipher_id);
                match client.get_item(&cipher_id).await {
                    Ok(item) => {
                         if let Some(login) = item.get("login") {
                             if let Some(enc_pass) = login.get("password").and_then(|p| p.as_str()) {
                                 // Decrypt
                                 let clean_enc = if enc_pass.starts_with("2.") { &enc_pass[2..] } else { enc_pass };
                                 match decrypt_aes256_cbc_hmac(clean_enc, &org_key) {
                                     Ok(pt_bytes) => {
                                         let pass = String::from_utf8(pt_bytes)?;
                                         println!("Decrypted Password from Vaultwarden: {}", pass.bold().on_yellow());
                                     },
                                     Err(e) => println!("Failed to decrypt password: {}", e),
                                 }
                             } else {
                                 println!("Item has no login.password");
                             }
                         }
                    },
                    Err(e) => println!("Failed to fetch item: {}", e),
                }
            } else {
                println!("No Cipher ID hardcoded in manifest. Skipping (dynamic lookup lookup not impl in find yet).");
            }
        }
    }

    if !match_found {
        println!("{}", "No matching secret found managed by rotation.yaml.".yellow());
    }

    // Always display Global Admin Token for recovery
    println!("{}", "\n--- Global Recovery ---".blue());
    if let Ok(token) = get_k8s_secret("vaultwarden-env", "vaultwarden-prod", "ADMIN_TOKEN") {
         println!("Vaultwarden Admin Token: {}", token.bold());
    } else {
         println!("Could not retrieve vaultwarden-prod/vaultwarden-env ADMIN_TOKEN");
    }

    Ok(())
}
