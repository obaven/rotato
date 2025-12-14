use clap::Args;
use anyhow::{anyhow, Result};
use std::process::Command;
use crate::models::RotationManifest;
use walkdir::WalkDir;
use colored::*;

#[derive(Args)]
pub struct CheckArgs {
    #[arg(long, help = "Path to config file or 'scan' mode")]
    pub config: Option<String>,
}

pub async fn run(_args: CheckArgs) -> Result<()> {
    println!("{}", "Running Pre-flight Checks...".bold().blue());

    let mut all_passed = true;

    // 1. Credentials Check
    println!("\n{}", "1. Credentials".bold());
    match crate::flows::resolve_credentials() {
        Ok((email, _)) => println!("  {} Credentials found for '{}' (Env or K8s)", "✔".green(), email),
        Err(_) => {
            println!("  {} No credentials found in Env (BW_EMAIL) or K8s (vaultwarden-admin-user). You will be prompted.", "⚠".yellow());
        }
    }
    
    match std::env::var("BW_SESSION") {
        Ok(_) => println!("  {} BW_SESSION is set.", "✔".green()),
        Err(_) => {
            println!("  {} BW_SESSION is missing. You will need to login interactively.", "⚠".yellow());
        }
    }

    // 2. Dependencies Check
    println!("\n{}", "2. Dependencies".bold());
    if check_command("kubectl") {
        println!("  {} kubectl found.", "✔".green());
    } else {
        println!("  {} kubectl NOT found in PATH.", "✘".red());
        all_passed = false;
    }

    if check_command("kubeseal") {
        println!("  {} kubeseal found.", "✔".green());
    } else {
        println!("  {} kubeseal NOT found in PATH (Sealing will fail).", "⚠".yellow());
        // Warning only, maybe they don't use sealing? (But our rotate logic assumes it for kubernetes block)
    }
    
    // 3. Configuration Check
    println!("\n{}", "3. Configuration Scan".bold());
    let repo_root = find_monorepo_root()?;
    let apps_dir = repo_root.join("apps");
    
    if !apps_dir.exists() {
         println!("  {} 'apps' directory not found at {:?}", "✘".red(), apps_dir);
         all_passed = false;
    } else {
        let mut count = 0;
        let mut failures = 0;
        
        for entry in WalkDir::new(&apps_dir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_name() == "rotation.yaml" {
                count += 1;
                match std::fs::read_to_string(entry.path()) {
                    Ok(content) => {
                        match serde_yaml::from_str::<RotationManifest>(&content) {
                            Ok(manifest) => {
                                println!("  {} Loaded: {:?} ({} secrets)", "✔".green(), entry.path(), manifest.secrets.len());
                            },
                            Err(e) => {
                                println!("  {} Failed to parse {:?}: {}", "✘".red(), entry.path(), e);
                                failures += 1;
                            }
                        }
                    },
                    Err(e) => {
                         println!("  {} Failed to read {:?}: {}", "✘".red(), entry.path(), e);
                         failures += 1;
                    }
                }
            }
        }
        
        if failures > 0 {
            all_passed = false;
            println!("  {} Found {} valid and {} invalid manifests.", "✘".red(), count - failures, failures);
        } else {
            println!("  {} Found {} valid configuration manifests.", "✔".green(), count);
        }
    }

    // 4. Connectivity (Optional - using VaultwardenClient logic if we can)
    // We won't instantiate full client here to keep 'check' lighweight and not force login just yet,
    // unless we want to verification. Let's keep it static checks for now.

    println!("\n{}", "Summary".bold());
    if all_passed {
        println!("{}", "All checks passed! Ready for rotation.".green().bold());
        Ok(())
    } else {
        println!("{}", "Some checks failed. Please review above.".red().bold());
        Err(anyhow!("Pre-flight check failed."))
    }
}

fn check_command(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// Duplicated from rotate.rs - implies we should move this to logic/infra shared
pub fn find_monorepo_root() -> Result<std::path::PathBuf> {
   if let Ok(output) = Command::new("git").args(&["rev-parse", "--show-toplevel"]).output() {
        if let Ok(s) = String::from_utf8(output.stdout) {
             let path = std::path::PathBuf::from(s.trim());
             if path.join("apps").exists() { return Ok(path); }
        }
    }
    // Fallback
    let mut current = std::env::current_dir()?;
    loop {
        if current.join("apps").exists() { return Ok(current); }
        if !current.pop() { break; }
    }
    Err(anyhow!("Could not find monorepo root"))
}
