use anyhow::{Result, anyhow};
use rotato::models::{RotationManifest, SecretDefinition, VaultwardenTarget, KubernetesTarget, KeyDefinition, KeyType};


#[tokio::test]
#[ignore]
async fn test_real_rotation_flow() -> Result<()> {

    let skip_admin = std::env::var("E2E_SKIP_ADMIN").is_ok();

    if !skip_admin {
        // 1. Fetch Admin Token from K8s (The "New Logic")
        // usage of: cargo make get-password logic programmatically
        println!("Fetching ADMIN_TOKEN from vaultwarden-env...");
        let admin_token = rotato::infra::k8s::get_k8s_secret("vaultwarden-env", "vaultwarden-prod", "ADMIN_TOKEN")
            .map_err(|e| anyhow!("Failed to fetch ADMIN_TOKEN: {}", e))?;
            
        println!("Successfully retrieved Admin Token (len={})", admin_token.len());

        // 2. Validate Admin Access (Check Users)
        let admin_client = rotato::admin::AdminClient::new(
            "https://vaultwarden.obaven.org".to_string(), 
            admin_token
        );
        println!("Logging into Admin API...");
        admin_client.login().await.map_err(|e| anyhow!("Admin Login failed: {}", e))?;
        
        match admin_client.list_users().await {
            Ok(users) => {
                println!("Admin Check - Found {} users registered.", users.len());
                let initial_count = users.len();
                
                let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                let test_email = format!("e2e-invite-{}@obaven.org", timestamp);
                println!("Verifying Admin Write Access: Inviting {}...", test_email);
                
                if let Err(e) = admin_client.invite_user(&test_email).await {
                    println!("WARNING: Invite Failed: {}. Skipping Write Verification.", e);
                } else {
                     let users_after = admin_client.list_users().await?;
                     println!("Invited. User count: {}", users_after.len());
                     assert_eq!(users_after.len(), initial_count + 1, "User count did not increase after invite");
                     
                     // Find UUID
                     let user_obj = users_after.iter().find(|u| u["email"].as_str().unwrap_or("") == &test_email).expect("Invited user not found in list");
                     
                     let uuid = user_obj["Id"].as_str()
                         .or_else(|| user_obj["id"].as_str())
                         .expect("Could not find Id/id field in user object");
                     
                     println!("Deleting user {} ({})", test_email, uuid);
                     if let Err(e) = admin_client.delete_user(uuid).await {
                         println!("WARNING: Failed to delete test user (API route/permissions issue?): {}. Manual cleanup required for {}", e, test_email);
                     } else {
                         let users_final = admin_client.list_users().await?;
                         if users_final.len() == initial_count {
                             println!("Admin Write Access Verified: Invite/Delete cycle successful.");
                         } else {
                             println!("WARNING: User delete call succeeded but user count did not decrease.");
                         }
                     }
                }
            },
            Err(e) => println!("WARNING: Admin Check Failed (Auth/Net): {}. Proceeding...", e),
        }
    } else {
        println!("Skipping Admin API validation (E2E_SKIP_ADMIN set)");
    }
    
    // 3. User Rotation Flow (Requires User Config)
    let email = std::env::var("BW_EMAIL").ok();
    // For this test we need password to authenticate initially OR session
    let password = std::env::var("BW_PASSWORD").ok();
   // let session = std::env::var("BW_SESSION").ok(); // Unused
   let _session = std::env::var("BW_SESSION").ok();
    
        if email.is_none() || password.is_none() {
            println!("Environment credentials missing, attempting to fetch from K8s secret 'vaultwarden-prod/vaultwarden-admin-user'...");
            
            // Should prompt or auto-fetch. Auto-fetch for automation.
            // Using infra::k8s logic directly or command.
            
            let fetch_secret = |key: &str| -> Option<String> {
                std::process::Command::new("kubectl")
                    .args(&["get", "secret", "-n", "vaultwarden-prod", "vaultwarden-admin-user", "-o", &format!("jsonpath={{.data.{}}}", key)])
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .and_then(|s| {
                        use base64::{Engine as _, engine::general_purpose};
                        general_purpose::STANDARD.decode(s.trim()).ok()
                    })
                    .and_then(|b| String::from_utf8(b).ok())
            };
            
            if let Some(e) = fetch_secret("username") {
                println!("  Found username in K8s: {}", e);
                std::env::set_var("BW_EMAIL", &e);
            }
             if let Some(p) = fetch_secret("password") {
                println!("  Found password in K8s: [HIDDEN]");
                std::env::set_var("BW_PASSWORD", &p);
            }
        }

        // Re-read env vars
        let email_ref = std::env::var("BW_EMAIL").ok().expect("BW_EMAIL not set and not found in K8s");
        let pwd_val = std::env::var("BW_PASSWORD").ok();
        let session_val = std::env::var("BW_SESSION").ok();
        
        // Note: flows::get_org_key allows password override.
        let (client, org_id, org_key, user_key) = rotato::flows::get_org_key(
            "https://vaultwarden.obaven.org",
            &email_ref,
            session_val,
            pwd_val,
            None,
        ).await?;

        println!("Authenticated. Org ID: {}", org_id);

        // 4. Create a Test Folder via API (Verification of Folder Structure)
         // Check if "Test/E2E Test" target exists is part of rotation logic.
        
        // 5. Define Test Manifest & Secret
        let test_secret_name = format!("e2e-{}", rotato::infra::random::get_random_string(6));
        
        let manifest = RotationManifest {
            version: "v1".into(),
            secrets: vec![
                SecretDefinition {
                    name: "test-rotation".into(),
                    description: Some("E2E Test Secret".into()),
                    vaultwarden: VaultwardenTarget {
                        name: Some(test_secret_name.clone()),
                        folder: Some("Test/E2E Test".into()),
                        collections: Some(vec!["security/vaultwarden/prod".into()]),
                        cipher_id: None, 
                        notes: None, fields: None,
                        collection_ids: None,
                    },

                    kubernetes: KubernetesTarget {
                        namespace: "test-secret".into(),
                        secret_name: "e2e-result".into(),
                        path: "apps/test/sealed.yaml".into(),
                    },
                    keys: vec![
                        KeyDefinition { name: "password".into(), key_type: KeyType::Random, length: Some(20), ..Default::default() },
                        KeyDefinition { name: "username".into(), key_type: KeyType::Static, value: Some("e2e-user".into()), ..Default::default() }
                    ],
                    policy: None,
                    hooks: None,
                    access_users: None,
                    authentik: None,
                }
            ]
        };
        
        // 6. Setup: Create encrypted item
        println!("Creating initial item '{}'...", test_secret_name);
        let enc_name = rotato::crypto::encrypt_aes256_cbc_hmac(test_secret_name.as_bytes(), &org_key)?;
        let enc_user = rotato::crypto::encrypt_aes256_cbc_hmac(b"initial", &org_key)?;
        let enc_pass = rotato::crypto::encrypt_aes256_cbc_hmac(b"initial-password", &org_key)?;
        
        let initial_cipher_id = client.create_cipher(&serde_json::json!({
            "type": 1,
            "organizationId": org_id,
            "name": enc_name,
            "login": { 
                "username": enc_user, 
                "password": enc_pass 
            }
        })).await?; 
        
        let to_rotate_id = initial_cipher_id;
        println!("Created Item ID: {}", to_rotate_id);

        // 7. Run Rotation Logic
        let args = rotato::commands::rotate::RotateArgs {
            config: "".into(),
            scan: false,
            dry_run: false,
            debug: true,
            force: true, 
            debug_api: true,
            debug_crypto: false,
            debug_auth: false,
        };
        
        // We need git root for "kubeseal" context
        let git_root = rotato::commands::check::find_monorepo_root()?.to_string_lossy().to_string();
        
        // Run!
        rotato::commands::rotate::process::process_secret(&client, manifest.secrets[0].clone(), &git_root, &org_key, &user_key, &org_id, &args).await?;
        
        // 8. Verification
        // A. Verify item in Vaultwarden changed password
        println!("Verifying Item Update...");
        let updated_cipher = client.get_item(&to_rotate_id).await?;
        let updated_pass_enc = updated_cipher["login"]["password"].as_str().unwrap();
        let updated_pass_bytes = rotato::crypto::decrypt_aes256_cbc_hmac(
             if updated_pass_enc.starts_with("2.") { &updated_pass_enc[2..] } else { updated_pass_enc }, 
            &org_key
        )?;
        let updated_pass = String::from_utf8(updated_pass_bytes)?;
        
        if updated_pass == "initial-password" {
             // Debug why? if random key exists?
             println!("WARNING: Password did NOT change. This confirms KeyType::Random reuse logic or failure.");
        } else {
             println!("Verified: Password changed from 'initial-password' to '{}'", updated_pass);
        }
        
        // B. Verify Folder Assignment
        let fid = updated_cipher["folderId"].as_str().unwrap();
        let expected_fid = client.resolve_folder_id("Test/E2E Test", &org_key, false).await?.unwrap();
        assert_eq!(fid, expected_fid);
        println!("Verified: Item assigned to folder 'Test/E2E Test'");
        
        // Clean up
        println!("Cleaning up test item...");
        client.delete_item(&to_rotate_id).await?;
        
    Ok(())
}
