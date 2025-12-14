use anyhow::{anyhow, Result};
use std::process::{Command, Stdio};
use std::io::Write;
use serde_json::Value;

use super::session::BwCli;

impl BwCli {
    pub fn get_org_collection(&self, col_id: &str) -> Result<Value> {
        let mut cmd = Command::new("bw");
        cmd.arg("get")
           .arg("org-collection")
           .arg(col_id)
           .env("BW_SESSION", &self.session)
           .arg("--nointeraction");
        
        if let Some(org_id) = &self.org_id {
            cmd.arg("--organizationid").arg(org_id);
        }

        let output = cmd.output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("bw get org-collection failed: {}", stderr));
        }

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let json: Value = serde_json::from_str(&stdout_str).map_err(|e| anyhow!("JSON Parse Error: {}. Output: '{}'", e, stdout_str))?;
        Ok(json)
    }

    pub fn encode(&self, data: &Value) -> Result<String> {
        let json_str = serde_json::to_string(data)?;
        
        let mut child = Command::new("bw")
            .arg("encode")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(json_str.as_bytes())?;
        }

        let output = child.wait_with_output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("bw encode failed: {}", stderr));
        }

        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    }

    pub fn edit_org_collection(&self, col_id: &str, encoded_data: &str) -> Result<()> {
        let mut cmd = Command::new("bw");
        cmd.arg("edit")
           .arg("org-collection")
           .arg(col_id)
           .arg(encoded_data)
           .env("BW_SESSION", &self.session)
           .arg("--nointeraction");
        
        if let Some(org_id) = &self.org_id {
            cmd.arg("--organizationid").arg(org_id);
        }

        let output = cmd.output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("bw edit org-collection failed: {}", stderr));
        }

        Ok(())
    }

    pub fn list_org_groups(&self) -> Result<Vec<Value>> {
        let mut cmd = Command::new("bw");
        cmd.arg("list")
           .arg("org-groups")
           .env("BW_SESSION", &self.session)
           .arg("--nointeraction");
        
        if let Some(org_id) = &self.org_id {
            cmd.arg("--organizationid").arg(org_id);
        }

        let output = cmd.output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("bw list org-groups failed: {}", stderr));
        }

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let json: Vec<Value> = serde_json::from_str(&stdout_str).map_err(|e| anyhow!("JSON Parse Error: {}. Output: '{}'", e, stdout_str))?;
        Ok(json)
    }

    pub fn list_org_members(&self) -> Result<Vec<Value>> {
        let mut cmd = Command::new("bw");
        cmd.arg("list")
           .arg("org-members")
           .env("BW_SESSION", &self.session)
           .arg("--nointeraction");
        
        if let Some(org_id) = &self.org_id {
            cmd.arg("--organizationid").arg(org_id);
        }

        let output = cmd.output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("bw list org-members failed: {}", stderr));
        }

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let json: Vec<Value> = serde_json::from_str(&stdout_str).map_err(|e| anyhow!("JSON Parse Error: {}. Output: '{}'", e, stdout_str))?;
        Ok(json)
    }

    pub fn create_org_collection(&self, name: &str) -> Result<String> {
        let payload = serde_json::json!({
            "name": name,
            "organizationId": self.org_id.clone().unwrap_or_default(),
            "externalId": null
        });
        let json_str = serde_json::to_string(&payload)?;

        let mut cmd = Command::new("bw");
        cmd.arg("create")
           .arg("org-collection")
           .arg(&json_str)
           .env("BW_SESSION", &self.session)
           .arg("--nointeraction");
        
        if let Some(org_id) = &self.org_id {
            cmd.arg("--organizationid").arg(org_id);
        }

        let output = cmd.output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("bw create org-collection failed: {}", stderr));
        }

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let json: Value = serde_json::from_str(&stdout_str).map_err(|e| anyhow!("JSON Parse Error: {}. Output: '{}'", e, stdout_str))?;
        
        if let Some(id) = json["id"].as_str() {
             Ok(id.to_string())
        } else {
             Err(anyhow!("Created collection but no ID returned via CLI"))
        }
    }
}
