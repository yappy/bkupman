use aes_gcm::{self, aead::Aead, AeadCore, Aes256Gcm, Key, KeyInit};
use anyhow::{anyhow, Result};
use rand::{rngs::OsRng, RngCore};

pub fn generate_random<const S: usize>() -> [u8; S] {
    let mut nonce: [u8; S] = [0; S];
    // cryptographically secure
    OsRng.fill_bytes(&mut nonce);

    nonce
}

pub fn hash_password() {}

pub const AesKeySize: usize = 32;
pub const AesNonceSize: usize = 12;
pub const AesTagSize: usize = 16;

pub type AesKey = [u8; AesKeySize];
pub type AesNonce = [u8; AesNonceSize];

/// key = 32 (AES 256 bit)
/// nonce = 12 (96 bit)
/// input = any
/// output = the same size as input
/// tag = 16
pub fn encrypt_aes256gcm(key: &AesKey, input: &[u8]) -> Result<(AesNonce, Vec<u8>)> {
    let key: &Key<Aes256Gcm> = key.into();
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let crypted = cipher.encrypt(&nonce, input).map_err(|err| anyhow!(err))?;

    Ok((nonce.into(), crypted))
}

pub fn decrypt_aes256gcm(key: &AesKey, nonce: AesNonce, input: &[u8]) -> Result<Vec<u8>> {
    let key: &Key<Aes256Gcm> = key.into();
    let cipher = Aes256Gcm::new(key);
    let plaintext = cipher
        .decrypt(&nonce.into(), input)
        .map_err(|err| anyhow!(err))?;

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aes256gcm() -> Result<()> {
        let key: AesKey = generate_random();
        let plaintext = b"hello";

        let (nonce, ciphertext) = encrypt_aes256gcm(&key, plaintext)?;
        assert_ne!(plaintext, &ciphertext.as_ref());

        let decrypted = decrypt_aes256gcm(&key, nonce, &ciphertext)?;
        assert_eq!(&decrypted.as_ref(), plaintext);

        Ok(())
    }
}
