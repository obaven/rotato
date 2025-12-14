use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;

pub fn get_file_value(git_root: &str, rel_path: &str, key_path: &str) -> Result<String> {
    let full_path = Path::new(git_root).join(rel_path);
    println!("    Reading value from file {:?} key {}...", full_path, key_path);
    
    let content = fs::read_to_string(&full_path)
        .map_err(|e| anyhow!("Failed to read file {:?}: {}", full_path, e))?;
        
    let yaml: serde_yaml::Value = serde_yaml::from_str(&content)
        .map_err(|e| anyhow!("Failed to parse YAML: {}", e))?;
        
    let parts: Vec<&str> = key_path.split('.').collect();
    let mut current = &yaml;
    
    for part in parts {
        current = current.get(part)
            .ok_or_else(|| anyhow!("Key path {} not found in file", key_path))?;
    }
    
    match current {
        serde_yaml::Value::String(s) => Ok(s.clone()),
        serde_yaml::Value::Number(n) => Ok(n.to_string()),
        serde_yaml::Value::Bool(b) => Ok(b.to_string()),
        _ => Err(anyhow!("Value at {} is not a simple string/number/bool", key_path)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_file_value_yaml_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("values.yaml");
        fs::write(&file_path, "
db:
  auth:
    user: admin
    port: 5432
").unwrap();

        let root = dir.path().to_str().unwrap();
        
        let val = get_file_value(root, "values.yaml", "db.auth.user").unwrap();
        assert_eq!(val, "admin");

        let port = get_file_value(root, "values.yaml", "db.auth.port").unwrap();
        assert_eq!(port, "5432");

        assert!(get_file_value(root, "values.yaml", "db.missing").is_err());
    }
}
