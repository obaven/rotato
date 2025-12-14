use anyhow::{anyhow, Result};
use rsa::{Oaep, Pkcs1v15Encrypt, RsaPrivateKey};
use rsa::pkcs8::DecodePrivateKey;
use sha1::Sha1;
use base64::{Engine as _, engine::general_purpose};

pub fn decrypt_rsa(ciphertext_b64: &str, private_key_pem: &str) -> Result<Vec<u8>> {
    let private_key = RsaPrivateKey::from_pkcs8_pem(private_key_pem)?;
    decrypt_rsa_internal(ciphertext_b64, &private_key)
}

pub fn decrypt_rsa_der(ciphertext_b64: &str, private_key_der: &[u8]) -> Result<Vec<u8>> {
    let private_key = RsaPrivateKey::from_pkcs8_der(private_key_der)?;
    decrypt_rsa_internal(ciphertext_b64, &private_key)
}

fn decrypt_rsa_internal(ciphertext_b64: &str, private_key: &RsaPrivateKey) -> Result<Vec<u8>> {
    let ciphertext = general_purpose::STANDARD.decode(ciphertext_b64)?;
    
    // Try OAEP SHA1 first (standard for BW?)
    let padding = Oaep::new::<Sha1>();
    if let Ok(pt) = private_key.decrypt(padding, &ciphertext) {
        return Ok(pt);
    }
    
    // Try PKCS1v15
    if let Ok(pt) = private_key.decrypt(Pkcs1v15Encrypt, &ciphertext) {
        return Ok(pt);
    }

    Err(anyhow!("RSA decryption failed with supported paddings."))
}
