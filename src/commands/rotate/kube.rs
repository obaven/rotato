use anyhow::{anyhow, Result};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use crate::models::SecretDefinition;

pub fn generate_and_write_secret(
    target: &crate::models::KubernetesTarget,
    secret_data: &std::collections::HashMap<String, String>,
    git_root: &str,
    dry_run: bool,
    debug: bool,
) -> Result<()> {
    let mut labels = std::collections::HashMap::new();
    labels.insert("managed-by".to_string(), "rotator-helper".to_string());
    
    if let Some(l) = &target.labels {
        labels.extend(l.clone());
    }

    let secret_json = serde_json::json!({
        "apiVersion": "v1",
        "kind": "Secret",
        "metadata": {
            "name": target.secret_name,
            "namespace": target.namespace,
            "labels": labels
        },
        "type": "Opaque",
        "stringData": secret_data
    });

    let secret_manifest = serde_yaml::to_string(&secret_json)?;

    if !dry_run {
        println!("  Sealing secret...");
        
        let cert_path = Path::new(git_root).join("apps/security/sealed-secrets/secrets/sealed-secrets-public-key.crt");
        
        let mut cmd = Command::new("kubeseal");
        cmd.arg("--format=yaml");
        if cert_path.exists() { cmd.arg(format!("--cert={}", cert_path.display())); }

        let mut child = cmd
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(secret_manifest.as_bytes())?;
        }
        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Err(anyhow!("kubeseal failed: {}", String::from_utf8_lossy(&output.stderr)));
        }
        let sealed_secret = String::from_utf8(output.stdout)?;

        // Write file
        let source_file = Path::new(git_root).join(&target.path);
        println!("  Updating file: {:?}", source_file);
        if let Some(parent) = source_file.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&source_file, &sealed_secret)?;
    } else {
        println!("  [Dry Run] Would seal and write to {:?}", target.path);
    }
    
    Ok(())
}
