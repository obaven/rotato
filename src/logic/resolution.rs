use anyhow::{anyhow, Result};
use crate::models::{KeyDefinition, KeyType};
use crate::vaultwarden::Collection;

pub fn resolve_collection_ids(
    requests: &[String], 
    all_collections: &[Collection], 
    org_key: &[u8]
) -> Vec<String> {
    let mut resolved = Vec::new();
     for col_name in requests {
         // 1. ID Match
         if let Some(c) = all_collections.iter().find(|c| c.id == *col_name) {
             resolved.push(c.id.clone());
         } else {
             // 2. Name Match (Plaintext)
             if let Some(c) = all_collections.iter().find(|c| c.name == *col_name) {
                 resolved.push(c.id.clone());
             } else {
                 // 3. Encrypted Name Match
                  if let Some(c) = all_collections.iter().find(|c| {
                      if c.name.starts_with("2.") {
                          // Note: crypto::decrypt_aes256_cbc_hmac handles "2." prefix now
                          if let Ok(b) = crate::crypto::decrypt_aes256_cbc_hmac(&c.name[2..], org_key) {
                              if let Ok(decrypted_name) = String::from_utf8(b) {
                                  if crate::crypto::is_debug() {
                                      println!("    DEBUG: Found collection: '{}' ({})", decrypted_name, c.id);
                                  }
                                  
                                  if decrypted_name == *col_name {
                                      return true;
                                  }
                              }
                          }
                      }
                      false 
                  }) {
                      resolved.push(c.id.clone());
                  } else {
                      if crate::crypto::is_debug() {
                          println!("    DEBUG: Failed to find collection '{}'", col_name);
                      }
                  }
             }
         }
     }
     resolved
}

pub fn resolve_value<F, K, R>(
    key_def: &KeyDefinition,
    existing_value: Option<String>,
    file_resolver: F,
    k8s_resolver: K,
    random_generator: R,
) -> Result<(String, bool, Option<std::collections::HashMap<String, String>>)>
where
    F: Fn(&str, &str) -> Result<String>,
    K: Fn(&str) -> Result<String>,
    R: Fn(usize) -> String,
{
    match &key_def.key_type {
        KeyType::Static => {
            if let Some(val) = &key_def.value {
                println!("    Using static value for '{}'", key_def.name);
                Ok((val.clone(), false, None))
            } else {
                Err(anyhow!("Key '{}' is Static but has no value", key_def.name))
            }
        },
        KeyType::File => {
             if let Some(source) = &key_def.source {
                println!("    Retrieving value for '{}' from file...", key_def.name);
                let val = file_resolver(&source.path, &source.key_path)?;
                println!("    Successfully read '{}' from file", key_def.name);
                Ok((val, false, None))
             } else {
                 Err(anyhow!("Key '{}' is File but has no source", key_def.name))
             }
        },
        KeyType::K8s => {
            let val = k8s_resolver(&key_def.name)?;
            println!("    Retrieved '{}' from K8s secret", key_def.name);
            Ok((val, false, None))
        },
        KeyType::Random => {
            if let Some(val) = existing_value {
                Ok((val, false, None))
            } else {
                let len = key_def.length.unwrap_or(32);
                let new_val = random_generator(len);
                println!("    Generated new random value for '{}'", key_def.name);
                Ok((new_val, true, None))
            }
        },
        KeyType::Ssh => {
            // SSH keys are always rotated (new generation) for now, or could check existing.
            // But usually we want to rotate keypair.
            println!("    Generating new SSH keypair (Ed25519) for '{}'...", key_def.name);
            let (private, public) = crate::infra::ssh::generate_ssh_keypair()?;
            
            let mut aux = std::collections::HashMap::new();
            aux.insert("public_key".to_string(), public);
            
            Ok((private, true, Some(aux)))
        }
    }
}

#[cfg(test)]
mod resolve_tests {
    use super::*;
    use crate::models::FileSource;

    #[test]
    fn test_resolve_static() {
        let def = KeyDefinition {
            name: "test".to_string(),
            key_type: KeyType::Static,
            value: Some("static-val".to_string()),
            length: None, generator: None, source: None,
        };
        
        let res = resolve_value(
            &def, 
            None, 
            |_, _| panic!("Should not call file"),
            |_| panic!("Should not call k8s"),
            |_| panic!("Should not call random")
        ).unwrap();
        
        assert_eq!(res.0, "static-val");
        assert_eq!(res.1, false);
    }

    #[test]
    fn test_resolve_random_new() {
        let def = KeyDefinition {
            name: "test".to_string(),
            key_type: KeyType::Random,
            value: None,
            length: Some(10), generator: None, source: None,
        };
        
        // Mock random generator
        let res = resolve_value(
            &def, 
            None, 
            |_, _| panic!("no file"),
            |_| panic!("no k8s"),
            |len| "a".repeat(len)
        ).unwrap();
        
        assert_eq!(res.0, "aaaaaaaaaa");
        assert_eq!(res.1, true); // is_new
    }

    #[test]
    fn test_resolve_random_existing() {
        let def = KeyDefinition {
            name: "test".to_string(),
            key_type: KeyType::Random,
            value: None,
            length: Some(10), generator: None, source: None,
        };
        
        // Mock random generator
        let res = resolve_value(
            &def, 
            Some("existing".to_string()), 
            |_, _| panic!("no file"),
            |_| panic!("no k8s"),
            |_| panic!("no random needed")
        ).unwrap();
        
        assert_eq!(res.0, "existing");
        assert_eq!(res.1, false);
        assert_eq!(res.2, None);
    }
    
    #[test]
    fn test_resolve_file() {
         let def = KeyDefinition {
            name: "test".to_string(),
            key_type: KeyType::File,
            value: None,
            length: None, generator: None, 
            source: Some(FileSource { path: "p".into(), key_path: "k".into() }),
        };
        
        let res = resolve_value(
            &def, 
            None, 
            |p, k| Ok(format!("{}:{}", p, k)),
            |_| panic!("no k8s"),
            |_| panic!("no random")
        ).unwrap();
        
        assert_eq!(res.0, "p:k");
    }
}

#[cfg(test)]
mod collection_resolution_tests {
    use super::*;

    #[test]
    fn test_resolve_by_id() {
        let collections = vec![
            Collection { id: "col-123".into(), name: "EncryptedStuff".into(), external_id: None, organization_id: "".into() },
            Collection { id: "col-456".into(), name: "Startups".into(), external_id: None, organization_id: "".into() },
        ];
        let reqs = vec!["col-123".to_string()];
        let ids = resolve_collection_ids(&reqs, &collections, &[]);
        assert_eq!(ids, vec!["col-123"]);
    }

    #[test]
    fn test_resolve_by_plaintext_name() {
        let collections = vec![
            Collection { id: "col-123".into(), name: "Security/Prod".into(), external_id: None, organization_id: "".into() },
            Collection { id: "col-456".into(), name: "Other".into(), external_id: None, organization_id: "".into() },
        ];
        let reqs = vec!["Security/Prod".to_string()];
        let ids = resolve_collection_ids(&reqs, &collections, &[]);
        assert_eq!(ids, vec!["col-123"]);
    }

    #[test]
    fn test_resolve_not_found() {
         let collections = vec![
            Collection { id: "col-123".into(), name: "Stuff".into(), external_id: None, organization_id: "".into() },
        ];
        let reqs = vec!["Missing".to_string()];
        let ids = resolve_collection_ids(&reqs, &collections, &[]);
        assert!(ids.is_empty());
    }

    #[test]
    fn test_resolve_mixed() {
        let collections = vec![
            Collection { id: "id-1".into(), name: "Name1".into(), external_id: None, organization_id: "".into() },
            Collection { id: "id-2".into(), name: "Name2".into(), external_id: None, organization_id: "".into() },
        ];
        let reqs = vec!["id-1".to_string(), "Name2".to_string(), "Missing".to_string()];
        let ids = resolve_collection_ids(&reqs, &collections, &[]);
        
        // Order is not guaranteed to be preserved if implementation changes, but currently it appends
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"id-1".to_string()));
        assert!(ids.contains(&"id-2".to_string()));
    }
}

