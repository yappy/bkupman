use aes_gcm::{self, aead::Aead, AeadCore, Aes256Gcm, Key, KeyInit};
use anyhow::{anyhow, Result};
use argon2::Argon2;
use rand::{rngs::OsRng, RngCore};

/// cryptographically secure
pub fn generate_random<const S: usize>() -> [u8; S] {
    let mut buf: [u8; S] = [0; S];
    OsRng.fill_bytes(&mut buf);

    buf
}

/// The author recommends 128 bit
pub const ARGON2_SALT_SIZE: usize = 16;
/// Memory: 19 MiB
pub const ARGON2_MCOST: u32 = argon2::Params::DEFAULT_M_COST;
/// Time(iterations): 2
pub const ARGON2_TCOST: u32 = argon2::Params::DEFAULT_T_COST;
/// Parallelism: 1
pub const ARGON2_PCOST: u32 = argon2::Params::DEFAULT_P_COST;

pub type Argon2Salt = [u8; ARGON2_SALT_SIZE];

pub fn aeskey_new_from_password(pwd: &str) -> (Argon2Salt, u32, u32, u32, AesKey) {
    let salt: Argon2Salt = generate_random();
    let m_cost = ARGON2_MCOST;
    let t_cost = ARGON2_TCOST;
    let p_cost = ARGON2_PCOST;
    let key = aeskey_from_password(salt, m_cost, t_cost, p_cost, pwd).unwrap();

    (salt, m_cost, t_cost, p_cost, key)
}

pub fn aeskey_from_password(
    salt: Argon2Salt,
    m_cost: u32,
    t_cost: u32,
    p_cost: u32,
    pwd: &str,
) -> Result<AesKey> {
    let mut res = AesKey::default();

    let params = argon2::Params::new(m_cost, t_cost, p_cost, Some(AesKeySize))
        .map_err(|err| anyhow!(err))?;
    let argon2 = Argon2::new(Default::default(), Default::default(), params);
    argon2
        .hash_password_into(pwd.as_bytes(), &salt, &mut res)
        .map_err(|err| anyhow!(err))?;

    Ok(res)
}

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
    fn test_password_hash_key() -> Result<()> {
        let pwd: &str = "password";
        let (salt, m, t, p, key1) = aeskey_new_from_password(&pwd);
        let key2 = aeskey_from_password(salt, m, t, p, pwd)?;
        assert_eq!(key1, key2);

        Ok(())
    }

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
