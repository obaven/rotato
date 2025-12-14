use anyhow::{anyhow, Result};
use std::process::Command;
use tempfile::NamedTempFile;

pub fn generate_ssh_keypair() -> Result<(String, String)> {
    // secure temp dir
    let temp_dir = tempfile::tempdir()?;
    let key_path = temp_dir.path().join("id_ed25519");
    let key_path_str = key_path.to_str().ok_or_else(|| anyhow!("Invalid temp path"))?;

    // ssh-keygen -t ed25519 -f <path> -N "" -q
    let output = Command::new("ssh-keygen")
        .arg("-t")
        .arg("ed25519")
        .arg("-f")
        .arg(key_path_str)
        .arg("-N")
        .arg("")
        .arg("-q")
        .output()?;

    if !output.status.success() {
        return Err(anyhow!("ssh-keygen failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    let private_key = std::fs::read_to_string(&key_path)?;
    let public_key_path = format!("{}.pub", key_path_str);
    let public_key = std::fs::read_to_string(&public_key_path)?;
    
    // Cleanup happens when temp_dir is dropped

    Ok((private_key.trim().to_string(), public_key.trim().to_string()))
}
