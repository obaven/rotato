use anyhow::{anyhow, Result};
use std::process::{Command};

pub struct BwCli {
    pub session: String,
    pub org_id: Option<String>,
}

impl BwCli {
    pub fn new(session: String, org_id: Option<String>) -> Self {
        Self { session, org_id }
    }

    /// Unlocks the vault (or logs in) and returns a BwCli instance with the session key.
    pub fn unlock(email: &str, password: &str) -> Result<Self> {
        // First try standard unlock
        let output = Command::new("sh")
            .arg("-c")
            .arg(format!("bw unlock '{}' --raw --nointeraction", password))
            .output()?;

        let mut session = if output.status.success() {
             String::from_utf8(output.stdout)?.trim().to_string()
        } else {
             String::new()
        };

        if session.is_empty() {
             let stderr = String::from_utf8_lossy(&output.stderr);
             println!("DEBUG: bw unlock returned empty/failed. Stderr: {}. Attempting Logout/Login fallback...", stderr);
             
             // Fallback: Logout and Login
             let _ = Command::new("bw").arg("logout").output(); 
             
             // Login
             let login_output = Command::new("sh")
                .arg("-c")
                .arg(format!("bw login '{}' '{}' --raw --nointeraction", email, password))
                .output()?;
            
             if !login_output.status.success() {
                 let login_stderr = String::from_utf8_lossy(&login_output.stderr);
                 return Err(anyhow!("bw login failed: {}", login_stderr));
             }
             
             session = String::from_utf8(login_output.stdout)?.trim().to_string();
        }

        if session.is_empty() {
            return Err(anyhow!("Failed to acquire session key via unlock or login."));
        }

        println!("DEBUG: Session Key Length: {}", session.len());
        println!("DEBUG: Session Key Unmasked (Partial): {}...", &session.chars().take(5).collect::<String>()); 
        
        let _ = Command::new("bw")
            .arg("sync")
            .env("BW_SESSION", &session)
            .arg("--nointeraction")
            .output();

        Ok(Self::new(session, None))
    }

    pub fn set_org_id(&mut self, org_id: String) {
        self.org_id = Some(org_id);
    }
}
