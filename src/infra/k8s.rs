use anyhow::{anyhow, Result};
use std::process::Command;
use base64::{Engine as _, engine::general_purpose};

pub fn get_k8s_secret(secret_name: &str, namespace: &str, key: &str) -> Result<String> {
    println!("    Fetching fallback value from K8s Secret {}/{} key {}...", namespace, secret_name, key);
    let output = Command::new("kubectl")
        .args(&["get", "secret", secret_name, "-n", namespace, "-o", "json"])
        .output()
        .map_err(|e| anyhow!("Failed to execute kubectl: {}", e))?;

    if !output.status.success() {
        return Err(anyhow!("kubectl failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| anyhow!("Failed to parse K8s secret JSON: {}", e))?;

    let data = json.get("data")
        .and_then(|d| d.as_object())
        .ok_or_else(|| anyhow!("Secret {} has no data", secret_name))?;

    let b64_val = data.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Key {} not found in secret {}", key, secret_name))?;
    
    // Fix newlines if any
    let b64_clean = b64_val.replace('\n', "");

    let bytes = general_purpose::STANDARD.decode(&b64_clean)
        .map_err(|e| anyhow!("Failed to decode base64 from k8s: {}", e))?;
    
    Ok(String::from_utf8(bytes)?)
}
