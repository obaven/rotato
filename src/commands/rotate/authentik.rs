use anyhow::{Result, Context};
use std::fs;
use std::path::Path;
use serde_yaml::Mapping;
use serde_yaml::Value;
use crate::models::{AuthentikTarget, AuthentikMetadata}; // Updated imports

pub fn generate_blueprint(target: &AuthentikTarget, secret_value: &str) -> Result<()> {
    println!("Generating Authentik Blueprint at: {}", target.path);

    // Construct the blueprint YAML structure
    let mut blueprint = Mapping::new();
    blueprint.insert(Value::String("version".to_string()), Value::Number(1.into()));

    let mut entry = Mapping::new();
    entry.insert(Value::String("model".to_string()), Value::String(target.metadata.model.clone()));
    
    // Identifiers
    let mut identifiers = Mapping::new();
    for (k, v) in &target.metadata.identifiers {
        identifiers.insert(Value::String(k.clone()), Value::String(v.clone()));
    }
    entry.insert(Value::String("identifiers".to_string()), Value::Mapping(identifiers));

    // Attributes (where the secret goes)
    let mut attributes = Mapping::new();
    attributes.insert(Value::String(target.metadata.secret_field.clone()), Value::String(secret_value.to_string()));
    entry.insert(Value::String("attrs".to_string()), Value::Mapping(attributes));

    let entries = vec![Value::Mapping(entry)];
    blueprint.insert(Value::String("entries".to_string()), Value::Sequence(entries));

    // Ensure directory exists
    if let Some(parent) = Path::new(&target.path).parent() {
        fs::create_dir_all(parent).context("Failed to create parent directory for blueprint")?;
    }

    // Write file
    let file = fs::File::create(&target.path).context("Failed to create blueprint file")?;
    serde_yaml::to_writer(file, &blueprint).context("Failed to write blueprint YAML")?;

    println!("Successfully generated Authentik Blueprint.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::tempdir;

    #[test]
    fn test_generate_blueprint() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("blueprint.yaml");
        let file_path_str = file_path.to_str().unwrap().to_string();

        let mut identifiers = HashMap::new();
        identifiers.insert("slug".to_string(), "test-slug".to_string());

        let target = AuthentikTarget {
            path: file_path_str.clone(),
            metadata: AuthentikMetadata {
                name: "test-auth".to_string(),
                model: "authentik_providers_oauth2.oauth2provider".to_string(),
                identifiers,
                secret_field: "client_secret".to_string(),
            },
        };

        generate_blueprint(&target, "my-super-secret").expect("Failed to generate blueprint");

        assert!(file_path.exists());
        let content = fs::read_to_string(file_path).unwrap();
        
        let yaml: serde_yaml::Value = serde_yaml::from_str(&content).expect("Failed to parse generated YAML");
        
        assert_eq!(yaml["version"], 1);
        let entries = yaml["entries"].as_sequence().unwrap();
        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry["model"], "authentik_providers_oauth2.oauth2provider");
        assert_eq!(entry["identifiers"]["slug"], "test-slug");
        assert_eq!(entry["attrs"]["client_secret"], "my-super-secret");
    }
}
