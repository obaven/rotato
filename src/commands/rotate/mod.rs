use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;
use crate::models::{RotationManifest, SecretDefinition, UserDefinition};
use futures::stream::{self, StreamExt};
use crate::infra::git::run_git_command;

pub mod types;
pub use types::RotateArgs;
pub mod authentik;
pub mod user; // [NEW]
pub mod utils;
use utils::find_monorepo_root;
pub mod hooks;
pub mod process;
pub mod lookup;
pub mod kube;
pub mod item_ops;
pub mod collections;

use process::process_secret;
use user::process_user; // [NEW]

pub async fn run(args: RotateArgs) -> Result<()> {
    // Configure Debug Flags
    crate::crypto::set_debug(args.debug_crypto);
    
    if args.debug {
        println!("Debug Mode: Logic=ON, API={}, Crypto={}, Auth={}", args.debug_api, args.debug_crypto, args.debug_auth);
    }

    println!("Authenticating to Vaultwarden...");
    
    let (email, password_opt) = crate::flows::resolve_credentials()?;
    let session_key = std::env::var("BW_SESSION").ok();
    
    let (client, org_id, org_key, user_key) = crate::flows::get_org_key(
        "https://vaultwarden.obaven.org",
        &email,
        session_key,
        password_opt,
        None
    ).await?;
    
    let mut client = client;
    client.debug_api = args.debug_api;
    
    println!("Obtained Organization Key for {}", org_id);

    // 1. Load configuration (Or Scan)
    let (secrets, users): (Vec<SecretDefinition>, Vec<UserDefinition>) = if args.scan {
        println!("Scanning for rotation.yaml files...");
        
        let apps_dir = find_monorepo_root()?.join("apps");
        println!("  Scanning in: {:?}", apps_dir);

        let mut found_secrets = Vec::new();
        let mut found_users = Vec::new();

        if apps_dir.exists() {
             for entry in WalkDir::new(apps_dir).into_iter().filter_map(|e| e.ok()) {
                if entry.file_name() == "rotation.yaml" {
                    println!("  Found manifest: {:?}", entry.path());
                    let content = fs::read_to_string(entry.path())?;
                    let manifest: RotationManifest = serde_yaml::from_str(&content)
                        .map_err(|e| anyhow!("Failed to parse {:?}: {}", entry.path(), e))?;
                    
                    found_secrets.extend(manifest.secrets);
                    found_users.extend(manifest.users);
                }
            }
        } else {
             println!("  Warning: 'apps' directory not found at {:?}", apps_dir);
        }
        
        if found_secrets.is_empty() {
            return Err(anyhow!("No rotation.yaml files found in scanning mode."));
        }
        println!("Found {} secret definitions.", found_secrets.len());
        (found_secrets, found_users)

    } else {
         let config_path = Path::new(&args.config);
         if !config_path.exists() {
             return Err(anyhow!("Config file {} not found. Use --scan to find rotation.yaml files.", args.config));
         }
         let config_content = fs::read_to_string(config_path)
             .map_err(|e| anyhow!("Failed to read config file {}: {}", args.config, e))?;
             
         let manifest: RotationManifest = serde_yaml::from_str(&config_content)
             .map_err(|e| anyhow!("Failed to parse config file as RotationManifest: {}", e))?;
          (manifest.secrets, manifest.users)
    };

    // Filter if requested
    let secrets = if let Some(pattern) = &args.secret {
        println!("Filtering secrets by pattern: '{}'", pattern);
        secrets.into_iter().filter(|s| s.name.contains(pattern)).collect()
    } else {
        secrets
    };

    let git_root_path = find_monorepo_root()?;
    let git_root = git_root_path.to_string_lossy().to_string();
    println!("Git Root: {}", git_root);

    println!("Rotating {} secrets with concurrency limit 5...", secrets.len());

    let results = stream::iter(secrets)
        .map(|secret| {
            let client = client.clone();
            let org_key = org_key.clone();
            let user_key = user_key.clone();
            let org_id = org_id.clone(); 
            let git_root = git_root.clone();
            let args = args.clone();
            
            async move {
                process_secret(&client, secret, &git_root, &org_key, &user_key, &org_id, &args).await
            }
        })
        .buffer_unordered(5)
        .collect::<Vec<Result<()>>>()
        .await;

    // Process Users (Sequential is fine for now, or parallel)
    println!("Processing {} users...", users.len());
    let user_results = stream::iter(users)
        .map(|user| {
             let client = client.clone();
             let org_key = org_key.clone();
             let org_id = org_id.clone();
             let git_root = git_root.clone();
             let args = args.clone();
             
             async move {
                 process_user(&client, user, &git_root, &org_key, &org_id, &args).await
             }
        })
        .buffer_unordered(5)
        .collect::<Vec<Result<()>>>()
        .await;

    let mut success_count = 0;
    let mut failure_count = 0;
    for res in results.into_iter().chain(user_results.into_iter()) {
        match res {
            Ok(_) => success_count += 1,
            Err(e) => {
                failure_count += 1;
                eprintln!("Error rotating item: {}", e);
            }
        }
    }
    
    println!("Rotation Manifest Summary:");
    println!("  Success: {}", success_count);
    println!("  Failure: {}", failure_count);
    
    if failure_count > 0 {
        return Err(anyhow!("Rotation failed for {} secrets.", failure_count));
    }

    if !args.dry_run {
        println!("Committing changes to Git...");
         let status = Command::new("git").args(&["status", "--porcelain"]).current_dir(&git_root).output()?;
         if !status.stdout.is_empty() {
            run_git_command(&["add", "."], &git_root)?;
            let diff_cached = Command::new("git").args(&["diff", "--cached", "--quiet"]).current_dir(&git_root).status()?;
            if !diff_cached.success() {
                run_git_command(&["commit", "-m", "Rotate secrets (Decentralized) [skip ci]"], &git_root)?;
            } else {
                println!("No staged changes to commit.");
            }
        } else { println!("No changes to commit."); }
    }
    
    println!("Rotation completed successfully!");
    Ok(())
}
