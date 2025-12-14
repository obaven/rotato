use anyhow::Result;
use clap::Args;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use crate::models::{RotationManifest, SecretDefinition, VaultwardenTarget, KubernetesTarget, KeyDefinition, KeyType};

#[derive(Args)]
pub struct ScaffoldArgs {
    #[arg(long)]
    app: Option<String>,
    #[arg(long)]
    namespace: Option<String>,
    #[arg(long, default_value = "prod")]
    env: String,
    #[arg(long)]
    output: Option<String>,
}

pub async fn run(args: ScaffoldArgs) -> Result<()> {
    println!("Scaffolding new rotation.yaml...");

    // 1. gather inputs (interactive or flags)
    let app_name = get_input("Application Name", args.app)?;
    let env = args.env;
    let namespace = get_input("Namespace", args.namespace).unwrap_or(format!("{}-{}", app_name, env));
    
    // Guess path
    let default_path = format!("apps/<category>/{}/rotation.yaml", app_name);
    let output_path = get_input("Output Path (e.g. apps/security/authentik/rotation.yaml)", args.output).unwrap_or(default_path);

    // 2. Build default structure
    let secret = SecretDefinition {
        name: format!("{}-admin-credentials", app_name),
        description: Some(format!("Admin credentials for {} ({})", app_name, env)),
        vaultwarden: VaultwardenTarget {
            cipher_id: None, // User needs to fill this
            name: None,
            collection_ids: None,
            collections: Some(vec!["infrastructure-team".to_string()]),
            folder: None, // Optional
            notes: Some(format!("Rotated by GitOps Rotator for {}", app_name)),
            fields: None,
        },
        kubernetes: KubernetesTarget {
            namespace: namespace.clone(),
            secret_name: format!("{}-admin-credentials", app_name),
            path: format!("overlays/{}/components/sealed_secrets/sealed-admin-credentials.yaml", env), // Guess
            labels: None,
        },
        additional_kubernetes: None,
        authentik: None, // Optional
        keys: vec![
            KeyDefinition {
                name: "password".to_string(),
                key_type: KeyType::Random,
                length: Some(32),
                value: None,
                generator: None,
                source: None,
            },
            KeyDefinition {
                name: "username".to_string(),
                key_type: KeyType::Static,
                length: None,
                value: Some("admin".to_string()),
                generator: None,
                source: None,
            },
        ],
        policy: None,
        hooks: None,
        access_users: None,
    };

    let manifest = RotationManifest {
        version: "v1".to_string(),
        secrets: vec![secret],
        users: vec![],
    };

    // 3. Serialize and Write
    let yaml = serde_yaml::to_string(&manifest)?;

    let path = Path::new(&output_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Check if exists
    if path.exists() {
        print!("File {} already exists. Overwrite? [y/N]: ", output_path);
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    fs::write(path, yaml)?;
    println!("\nSuccess! Scaffolding written to {}", output_path);
    println!("Next steps:");
    println!("1. Edit the file to set 'vaultwarden.cipherId'.");
    println!("2. Verify 'kubernetes.path' points to your real SealedSecret source file.");

    Ok(())
}

fn get_input(prompt: &str, arg: Option<String>) -> Result<String> {
    if let Some(v) = arg {
        return Ok(v);
    }
    print!("{}: ", prompt);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scaffold_run_non_interactive() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("rotation.yaml");
        let output_str = file_path.to_str().unwrap().to_string();

        let args = ScaffoldArgs {
            app: Some("integration-app".to_string()),
            namespace: Some("integration-ns".to_string()),
            env: "test".to_string(),
            output: Some(output_str),
        };

        run(args).await.expect("Scaffold run failed");

        assert!(file_path.exists());
        let content = fs::read_to_string(file_path).unwrap();
        assert!(content.contains("integration-app-admin-credentials"));
        assert!(content.contains("overlays/test/components/sealed_secrets/sealed-admin-credentials.yaml")); // derived path
    }
}
