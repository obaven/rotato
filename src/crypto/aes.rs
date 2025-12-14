use anyhow::{anyhow, Result};
use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use cbc::{Decryptor, Encryptor};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use base64::{Engine as _, engine::general_purpose};
use rand::RngCore;

type Aes256CbcDec = Decryptor<aes::Aes256>;
type Aes256CbcEnc = Encryptor<aes::Aes256>;

use super::debug::is_debug;

pub fn decrypt_aes256_cbc_hmac(ciphertext_b64: &str, key: &[u8]) -> Result<Vec<u8>> {
    if key.len() < 32 {
        return Err(anyhow!("Key length too short: {}", key.len()));
    }

    let enc_key = &key[0..32];
    let mac_key = if key.len() >= 64 { Some(&key[32..64]) } else { None };
    
    if ciphertext_b64.contains('|') {
        let parts: Vec<&str> = ciphertext_b64.split('|').collect();
        if parts.len() != 3 {
            return Err(anyhow!("Invalid cipher string format (expected IV|CT|MAC)"));
        }

        let iv = general_purpose::STANDARD.decode(parts[0])?;
        let ct = general_purpose::STANDARD.decode(parts[1])?;
        let mac = general_purpose::STANDARD.decode(parts[2])?;

        if let Some(mk) = mac_key {
            let mut hmac = Hmac::<Sha256>::new_from_slice(mk)?;
            hmac.update(&iv);
            hmac.update(&ct);
            if let Err(_) = hmac.verify_slice(&mac) {
                if is_debug() { println!("DEBUG: HMAC verification failed!"); }
                return Err(anyhow!("HMAC verification failed"));
            }
        }

        let cipher = Aes256CbcDec::new_from_slices(enc_key, &iv)?;
        // Clone CT to mutable buffer for in-place decryption
        let mut buf = ct.clone();
        let pt = cipher.decrypt_padded_mut::<Pkcs7>(&mut buf)
            .map_err(|e| anyhow!("Decryption failed: {:?}", e))?;
        Ok(pt.to_vec())
    } else {
        Err(anyhow!("Ciphertext does not contain pipes, usage mismatch?"))
    }
}

pub fn decrypt_aes256_cbc_raw(ciphertext_b64: &str, key: &[u8]) -> Result<Vec<u8>> {
     if is_debug() { println!("DEBUG: Attempting Raw Decrypt..."); }
     let enc_key = if key.len() >= 32 { &key[0..32] } else { key };

     let bytes = general_purpose::STANDARD.decode(ciphertext_b64)?;
     
     if bytes.len() < 16 {
         return Err(anyhow!("Ciphertext too short"));
     }

     let iv = &bytes[0..16];
     let ct = &bytes[16..];

     let cipher = Aes256CbcDec::new_from_slices(enc_key, iv)?;
     let mut buf = ct.to_vec();
     let pt = cipher.decrypt_padded_mut::<Pkcs7>(&mut buf)
        .map_err(|e| anyhow!("Decryption failed: {:?}", e))?;
     Ok(pt.to_vec())
}

pub fn encrypt_aes256_cbc_hmac(plaintext: &[u8], key: &[u8]) -> Result<String> {
    if key.len() < 64 {
        return Err(anyhow!("Key must be 64 bytes (32 Enc + 32 Mac) for enc-then-mac"));
    }
    let enc_key = &key[0..32];
    let mac_key = &key[32..64];

    let mut iv = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut iv);

    let cipher = Aes256CbcEnc::new_from_slices(enc_key, &iv)?;
    // Encrypt padded vec mut requires BlockEncryptMut trait. 
    // If not available, use encrypt_padded_vec_mut from BlockEncryptMut?
    // Wait, encrypt_padded_vec_mut allocates. The compiler said `encrypt_padded_vec_mut` not found.
    // Try manual padding + encryption?
    // Or simpler: `encrypt_padded_vec_mut` is usually available if trait imported.
    // Let's assume compiler was right about previous error and usage of manual approach is standard here.
    // We need a buffer big enough: len + block_size.
    let pos = plaintext.len();
    let len = pos + 16; // Sufficient for padding
    let mut buf = vec![0u8; len];
    buf[..pos].copy_from_slice(plaintext);
    
    let ct = cipher.encrypt_padded_mut::<Pkcs7>(&mut buf, pos)
        .map_err(|e| anyhow!("Encryption failed: {:?}", e))?;

    let mut hmac = Hmac::<Sha256>::new_from_slice(mac_key)?;
    hmac.update(&iv);
    hmac.update(ct);
    let mac = hmac.finalize().into_bytes();

    let iv_b64 = general_purpose::STANDARD.encode(iv);
    let ct_b64 = general_purpose::STANDARD.encode(ct);
    let mac_b64 = general_purpose::STANDARD.encode(mac);

    Ok(format!("2.{}|{}|{}", iv_b64, ct_b64, mac_b64))
}
