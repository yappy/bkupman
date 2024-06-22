use crypto::{aead::AeadEncryptor, aes::KeySize, aes_gcm::AesGcm};
use rand::{rngs::OsRng, RngCore};

pub const KeySize: usize = 32;
pub const NonceSize: usize = 12;
pub const TagSize: usize = 16;

pub type Key = [u8; KeySize];
pub type Nonce = [u8; NonceSize];
pub type Tag = [u8; TagSize];

pub fn generate_key() -> Key {
    generate_random()
}

pub fn generate_nonce() -> Nonce {
    generate_random()
}

fn generate_random<const S: usize>() -> [u8; S] {
    let mut nonce: [u8; S] = [0; S];
    // cryptographically secure
    OsRng.fill_bytes(&mut nonce);

    nonce
}

/// key = 32 (AES 256 bit)
/// nonce = 12 (96 bit)
/// aad = any (empty ok)
/// input = any
/// output = the same size as input
/// tag = 16
pub fn encrypt(
    key: &[u8],
    nonce: &[u8],
    aad: &[u8],
    input: &[u8],
    output: &mut [u8],
    tag: &mut [u8],
) {
    let mut aes = AesGcm::new(KeySize::KeySize256, key, nonce, aad);
    aes.encrypt(input, output, tag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nonce() {
        let n1 = generate_nonce();
        let n2 = generate_nonce();
        assert_ne!(n1, n2);
    }
}
