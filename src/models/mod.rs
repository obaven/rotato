pub mod config;
pub mod vaultwarden_target;
pub mod k8s;
pub mod keys;
pub mod authentik;
pub mod user;

pub use config::*;
pub use vaultwarden_target::*;
pub use k8s::*;
pub use keys::*;
pub use authentik::*;
pub use user::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rotation_manifest_deserialization() {
        let yaml = r#"
version: v1
secrets:
  - name: test-secret
    vaultwarden:
      cipherId: "123"
    kubernetes:
      namespace: default
      secretName: test
      path: secrets/test.yaml
    keys:
      - name: pass
        type: random
        length: 32
"#;
        let manifest: RotationManifest = serde_yaml::from_str(yaml).expect("Failed to parse YAML");
        assert_eq!(manifest.secrets.len(), 1);
        assert_eq!(manifest.secrets[0].vaultwarden.cipher_id, Some("123".to_string()));
        assert!(matches!(manifest.secrets[0].keys[0].key_type, KeyType::Random));
    }

    #[test]
    fn test_roundtrip_serialization() {
       let manifest = RotationManifest {
           version: "v1".to_string(),
           secrets: vec![
               SecretDefinition {
                   name: "demo".to_string(),
                   description: None,
                   vaultwarden: VaultwardenTarget {
                       cipher_id: Some("abc".to_string()),
                       name: None,
                       collections: None,
                       folder: None,
                       notes: None,
                       fields: None,
                       collection_ids: None,
                   },
                   kubernetes: KubernetesTarget {
                       namespace: "ns".to_string(),
                       secret_name: "sec".to_string(),
                       path: "path".to_string(),
                   },
                   keys: vec![
                       KeyDefinition {
                           name: "key1".to_string(),
                           key_type: KeyType::Static,
                           value: Some("val".to_string()),
                           length: None,
                           generator: None,
                           source: None,
                       }
                   ],
                   policy: None,
                   hooks: None,
                   access_users: None,
                   authentik: None,
               }
           ]
       };
       
       let yaml = serde_yaml::to_string(&manifest).expect("Failed to serialize");
       let deserialized: RotationManifest = serde_yaml::from_str(&yaml).expect("Failed to deserialize");
       assert_eq!(deserialized.secrets[0].name, "demo");
    }
}
