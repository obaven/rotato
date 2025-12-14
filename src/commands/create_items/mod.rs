use anyhow::Result;
use std::path::Path;
use crate::vaultwarden::Member;

pub mod types;
pub use types::*;
pub mod access;
pub mod builder;
pub mod existence;
pub mod process;
use process::process_secret;

pub async fn run(args: CreateItemsArgs) -> Result<()> {
    let (email, password_opt) = crate::flows::resolve_credentials()?;
    
    let session_key = std::env::var("BW_SESSION").ok();

    println!("Authenticating to Vaultwarden...");
    let (client, org_id, org_key, user_key) = crate::flows::get_org_key(
        &args.base_url,
        &email,
        session_key.clone(),
        password_opt.clone(),
        args.org_id.as_deref(),
    ).await?;

    // Load secrets (Scan or Config)
    use walkdir::WalkDir;
    use crate::models::{RotationManifest, SecretDefinition};
    use crate::commands::create_items::types::{SecretConfig, VaultwardenConfig}; // Local types

    let mut secrets: Vec<SecretConfig> = if args.scan {
        println!("Scanning for rotation.yaml files...");
        let apps_dir = crate::commands::rotate::utils::find_monorepo_root()?.join("apps");
        let mut found_secrets = Vec::new();
        
        if apps_dir.exists() {
             for entry in WalkDir::new(apps_dir).into_iter().filter_map(|e| e.ok()) {
                if entry.file_name() == "rotation.yaml" {
                    let content = std::fs::read_to_string(entry.path())?;
                    // Parse as RotationManifest
                    let manifest: RotationManifest = serde_yaml::from_str(&content)
                        .map_err(|e| anyhow::anyhow!("Failed to parse {:?}: {}", entry.path(), e))?;
                    
                    // Convert to SecretConfig
                    for s in manifest.secrets {
                        let vw = s.vaultwarden;
                        found_secrets.push(SecretConfig {
                            name: s.name,
                            env: None, 
                            vaultwarden: Some(VaultwardenConfig {
                                cipher_id: vw.cipher_id,
                                collection_ids: vw.collection_ids,
                                collections: vw.collections,
                                folder: vw.folder,
                                notes: vw.notes,
                            }),
                            keys: None,
                            kubernetes: None,
                            source_files: None,
                            namespace: Some(s.kubernetes.namespace),
                            secret_name: Some(s.kubernetes.secret_name),
                            argocd_app: None,
                            notes: None,
                            repo: None,
                            path: None,
                            access_users: s.access_users,
                        });
                    }
                }
            }
        }
        found_secrets
    } else {
        let config_path = Path::new(&args.config);
        let file = std::fs::File::open(config_path)?;
        let config: Config = serde_yaml::from_reader(file)?;
        config.secrets
    };

    // Filter
    if let Some(pattern) = &args.secret {
        println!("Filtering secrets by pattern: '{}'", pattern);
        secrets.retain(|s| s.name.contains(pattern));
    }
    
    let mut modified = false;

    // Fetch collections once for resolution
    let all_collections = client.list_collections(&org_id).await?;
    // Fetch members once for access control
    // Initialize BW CLI wrapper if session available, or prepare for lazy unlock
    let mut output_cli: Option<crate::bw_cli::BwCli> = if let Some(key) = &session_key {
         Some(crate::bw_cli::BwCli::new(key.clone(), Some(org_id.clone())))
    } else {
         None
    };

    // Fetch members once for access control
    let mut all_members = client.list_members(&org_id).await?;
    if all_members.is_empty() {
        println!("Wait: Member list from API is empty (likely 404 or permission). Trying BW CLI...");
        
        // If we don't have a CLI yet (no session key), try to unlock with password
        if output_cli.is_none() {
            if let Some(password) = &password_opt {
                 println!("    Unlocking BW CLI with password...");
                 match crate::bw_cli::BwCli::unlock(&email, password) {
                     Ok(c) => output_cli = Some(c),
                     Err(e) => println!("Warning: Failed to unlock BW CLI: {}", e),
                 }
            } else {
                 println!("Warning: No session key or password available to unlock CLI");
            }
        }

        if let Some(cli) = &mut output_cli {
            // Ensure org context
            cli.set_org_id(org_id.clone());
            
            match cli.list_org_members() {
                Ok(json_members) => {
                     println!("BW CLI returned {} members.", json_members.len());
                     for val in json_members {
                         match serde_json::from_value::<Member>(val) {
                             Ok(m) => all_members.push(m),
                             Err(e) => println!("Warning: Failed to parse member from CLI: {}", e),
                         }
                     }
                },
                Err(e) => println!("Warning: BW CLI list-org-members failed: {}", e),
            }
        }
    }

    for secret in &mut secrets {
        if process_secret(secret, &client, &org_id, &org_key, &user_key, &all_collections, &all_members, output_cli.as_ref()).await? {
            modified = true;
        }
    }

    if modified && !args.scan {
        println!("Updating config file with new cipher IDs...");
        let config_path = std::path::Path::new(&args.config);
        let f = std::fs::File::create(config_path)?;
        // We need to adhere to Config struct
        let config = crate::commands::create_items::types::Config { secrets };
        serde_yaml::to_writer(f, &config)?;
        println!("Config file updated.");
    } else if modified {
         println!("Warning: Items were created/modified, but writing back to decentralized rotation.yaml files is not yet supported. Please manually update cipherIds if desired, or rely on name lookup.");
    }

    Ok(())
}
