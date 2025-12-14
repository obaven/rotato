use anyhow::{anyhow, Result};
use std::fs;
use std::io::Write;
use std::path::Path;
use crate::infra::k8s::get_k8s_secret;

pub mod types;
pub use types::*;
pub mod purge_vault;
pub mod prune_config;

pub async fn run(args: CleanupArgs) -> Result<()> {
    println!("=== Vaultwarden Cleanup ===");

    // 1. Load configuration
    let config_path = Path::new(&args.config);
    if !config_path.exists() {
        return Err(anyhow!("Config file '{}' not found.", args.config));
    }
    
    let config_content = fs::read_to_string(config_path)
        .map_err(|e| anyhow!("Failed to read config file {}: {}", args.config, e))?;
    let config: Config = serde_yaml::from_str(&config_content)
        .map_err(|e| anyhow!("Failed to parse config file {}: {}", args.config, e))?;

    // 2. Authenticate
    let email = std::env::var("BW_EMAIL").or_else(|_| {
        get_k8s_secret("vaultwarden-admin-user", "vaultwarden-prod", "username")
    }).unwrap_or_else(|_| {
        print!("Bitwarden Email: ");
        std::io::stdout().flush().unwrap();
        let mut buffer = String::new();
        std::io::stdin().read_line(&mut buffer).unwrap();
        buffer.trim().to_string()
    });
    
    let session_key = std::env::var("BW_SESSION").ok();
    let password_opt = std::env::var("BW_PASSWORD").ok().or_else(|| {
        get_k8s_secret("vaultwarden-admin-user", "vaultwarden-prod", "password").ok()
    });

    println!("Authenticating to Vaultwarden...");
    let (client, _, org_key, user_key) = crate::flows::get_org_key(
        "https://vaultwarden.obaven.org", 
        &email,
        session_key,
        password_opt,
        None,
    ).await?;

    // 3. Execute Cleanup Strategy
    if args.all {
        purge_vault::purge_all(&client, &org_key, &user_key).await?;
    } else {
        prune_config::prune_from_config(config, &client, &org_key, &user_key).await?;
    }

    println!("Cleanup completed.");
    Ok(())
}
