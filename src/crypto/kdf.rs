use anyhow::{anyhow, Result};
use pbkdf2::pbkdf2;
use sha2::Sha256;
use hkdf::Hkdf;

pub fn derive_master_key_pbkdf2(password: &[u8], salt: &[u8], iterations: u32) -> Vec<u8> {
    let mut key = [0u8; 32];
    pbkdf2::<hmac::Hmac<Sha256>>(password, salt, iterations, &mut key).expect("PBKDF2 failed");
    key.to_vec()
}

pub fn derive_master_key_argon2id(password: &[u8], salt: &[u8], iterations: u32, memory: u32, parallelism: u32) -> Result<Vec<u8>> {
    use argon2::{Argon2, Params, Algorithm, Version};
    
    let m_cost = memory * 1024; 
    let t_cost = iterations;
    let p_cost = parallelism;

    let params = Params::new(m_cost, t_cost, p_cost, Some(32))
        .map_err(|e| anyhow!("Argon2 params error: {}", e))?;
        
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut key = [0u8; 32];
    argon2.hash_password_into(password, salt, &mut key)
        .map_err(|e| anyhow!("Argon2 hash error: {}", e))?;
    
    Ok(key.to_vec())
}

pub fn stretch_key_hkdf(master_key: &[u8]) -> Vec<u8> {
    // Bitwarden treats the KDF output as the HKDF PRK and skips the extract step.
    let hk = Hkdf::<Sha256>::from_prk(master_key).expect("Invalid HKDF PRK length");
    
    let mut enc_key = [0u8; 32];
    hk.expand(b"enc", &mut enc_key).expect("HKDF expand failed");
    let mut mac_key = [0u8; 32];
    hk.expand(b"mac", &mut mac_key).expect("HKDF expand failed");
    
    let mut combined = Vec::new();
    combined.extend_from_slice(&enc_key);
    combined.extend_from_slice(&mac_key);
    combined
}
