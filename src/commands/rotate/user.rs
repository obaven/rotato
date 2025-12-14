use anyhow::Result;
use crate::models::{UserDefinition};
use crate::commands::rotate::authentik::generate_blueprint;
use pbkdf2::{pbkdf2_hmac};
use sha2::Sha256;
use base64::{Engine as _, engine::general_purpose};

pub async fn process_user(
    _client: &crate::vaultwarden::VaultwardenClient,
    user: UserDefinition,
    _git_root: &str,
    _org_key: &[u8],
    _org_id: &str,
    args: &crate::commands::rotate::RotateArgs,
) -> Result<()> {
    println!("Processing User: {}", user.username);

    // 1. Generate new password
    // We use a reasonably secure default length if not specified (users don't have key definitions like secrets)
    let new_password = crate::infra::random::get_random_string(32);

    // 2. Hash Password for Authentik (PBKDF2-SHA256, Django format)
    // Django format: pbkdf2_sha256$iterations$salt$hash
    let iterations = 260000;
    let salt = crate::infra::random::get_random_string(12); // alphanumeric salt
    let mut hash_output = [0u8; 32]; // SHA256 output size
    
    pbkdf2_hmac::<Sha256>(
        new_password.as_bytes(),
        salt.as_bytes(),
        iterations,
        &mut hash_output
    );
    
    let hash_b64 = general_purpose::STANDARD.encode(hash_output);
    let django_hash = format!("pbkdf2_sha256${}${}${}", iterations, salt, hash_b64);
    
    // 3. Update Vaultwarden
    // Logic similar to rotation: Find item by name/username, update login.password
    // We need to construct a "SecretDefinition" compatible struct or write custom logic.
    // Custom logic is safer for Users.
    
    // TODO: Implement Vaultwarden update logic here. 
    // For now, we print the would-be actions.
    if args.dry_run {
         println!("  [Dry Run] Would update Vaultwarden user {} with new password.", user.username);
    } else {
         // Create or Update logic (simplified)
         println!("  Updating Vaultwarden not yet fully implemented for Users. Skipping step.");
    }

    // 4. Generate Authentik Blueprint (with hashed password)
    if let Some(target) = &user.authentik {
        // We pass the HASHED password to the blueprint generator
        // The blueprint generator needs to know to put this in "password" field.
        // If the user's secret_field is "password", it works.
        generate_blueprint(target, &django_hash)?;
    }

    Ok(())
}
