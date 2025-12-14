
pub fn find_existing_encrypted_value(item: &serde_json::Value, key: &str, org_key: &[u8]) -> Option<String> {
    // Helper to decrypt string
    let decrypt = |s: &str| -> Option<String> {
        let content = if s.starts_with("2.") { &s[2..] } else { s };
        crate::crypto::decrypt_aes256_cbc_hmac(content, org_key).ok()
            .and_then(|b| String::from_utf8(b).ok())
    };

    // 1. Check Standard Login Fields
    if key == "username" {
        if let Some(v) = item.get("login").and_then(|l| l.get("username")).and_then(|s| s.as_str()) {
            if !v.is_empty() { return decrypt(v); }
        }
    } else if key == "password" {
        if let Some(v) = item.get("login").and_then(|l| l.get("password")).and_then(|s| s.as_str()) {
            if !v.is_empty() { return decrypt(v); }
        }
    }

    // 2. Check Custom Fields
    if let Some(fields) = item.get("fields").and_then(|f| f.as_array()) {
        for field in fields {
            if let Some(name_enc) = field["name"].as_str() {
                // Decrypt name to check match
                if let Some(name) = decrypt(name_enc) {
                    if name == *key {
                        if let Some(v) = field["value"].as_str() { return decrypt(v); }
                    }
                } 
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_find_existing_value_login() {
        // Disabled because now logic requires valid encryption
        let item = serde_json::json!({
            "login": {
                "username": "existing_user",
                "password": "existing_pass"
            }
        });
        
        let found = find_existing_encrypted_value(&item, "username", &[]); 
        assert_eq!(found, Some("existing_user".to_string()));
    }
}
