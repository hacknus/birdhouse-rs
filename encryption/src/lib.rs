use aes::Aes256;
use base64::{engine::general_purpose, Engine as _};
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

type Aes256Cbc = Cbc<Aes256, Pkcs7>;

#[derive(Debug, PartialEq)]
pub enum EncryptionError {
    ExpiredTimestamp,
    InvalidTimestampLen,
    ReplayAttack,
    UTF8Error,
}

pub struct Cipher {
    key: Vec<u8>,
    nonce_set: HashSet<(Vec<u8>, Vec<u8>)>,
    nonce_expiration_seconds: u64,
}

impl Cipher {
    pub fn new(key: &str, nonce_expiration_seconds: u64) -> Self {
        Self {
            key: key.as_bytes().to_vec(),
            nonce_set: HashSet::default(),
            nonce_expiration_seconds,
        }
    }

    pub fn encrypt_message(&self, message: &str) -> String {
        let message = message.to_string().into_bytes();
        // pad(&mut message);
        let mut iv = [0u8; 8];
        getrandom::fill(&mut iv).expect("Failed to generate IV");
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let timestamp_bytes = timestamp.as_secs().to_le_bytes();
        let mut timestamp_iv = Vec::with_capacity(16);
        timestamp_iv.extend_from_slice(&timestamp_bytes);
        timestamp_iv.extend_from_slice(&iv);
        let cipher = Aes256Cbc::new_from_slices(&self.key, &timestamp_iv)
            .expect("Failed to create AES cipher");
        let ciphertext = cipher.encrypt_vec(&message);
        let mut output = timestamp_iv;
        output.extend_from_slice(&ciphertext);
        general_purpose::STANDARD.encode(output)
    }

    pub fn cleanup_expired_nonces(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        self.nonce_set.retain(|(timestamp, _)| {
            let timestamp_bytes: [u8; 8] = {
                let mut arr = [0; 8];
                arr.copy_from_slice(&timestamp[..8]); // Slice to ensure it's not bigger than 8 bytes
                arr
            };
            let timestamp_int = u64::from_le_bytes(timestamp_bytes);
            timestamp_int + self.nonce_expiration_seconds >= now
        });
    }

    pub fn nonce_is_used(&mut self, timestamp_iv: (Vec<u8>, Vec<u8>)) -> bool {
        self.cleanup_expired_nonces();
        self.nonce_set.contains(&timestamp_iv)
    }

    pub fn decrypt_message(&mut self, ciphertext: &str) -> Result<String, EncryptionError> {
        let ciphertext = general_purpose::STANDARD
            .decode(ciphertext)
            .expect("Failed to decode ciphertext");
        let timestamp_iv = (ciphertext[0..8].to_vec(), ciphertext[8..16].to_vec());
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        let timestamp_bytes = {
            if timestamp_iv.0.len() != 8 {
                return Err(EncryptionError::InvalidTimestampLen);
            }
            let mut arr = [0; 8];
            arr.copy_from_slice(&timestamp_iv.0);
            arr
        };
        let timestamp = u64::from_le_bytes(timestamp_bytes);
        if timestamp > now + self.nonce_expiration_seconds
            || timestamp < now - self.nonce_expiration_seconds
        {
            return Err(EncryptionError::ExpiredTimestamp);
        }
        if self.nonce_is_used(timestamp_iv.clone()) {
            return Err(EncryptionError::ReplayAttack);
        }
        self.nonce_set.insert(timestamp_iv.clone());
        let cipher = Aes256Cbc::new_from_slices(&self.key, &ciphertext[..16])
            .expect("Failed to create AES cipher");
        let decrypted_message = cipher
            .decrypt_vec(&ciphertext[16..])
            .expect("Failed to decrypt message");
        String::from_utf8(decrypted_message).map_err(|_e| EncryptionError::UTF8Error)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Cipher, EncryptionError};

    #[test]
    fn test_encryption() {
        let mut cipher = Cipher::new("e10adc3949ba59abbe56e057f20f883e", 30);
        let message = "[CMD] setTemperature=30.0\r\n";
        let ciphertext1 = cipher.encrypt_message(message);
        let decrypted_message = cipher.decrypt_message(&ciphertext1).unwrap();
        assert_eq!(decrypted_message, message);

        let ciphertext2 = cipher.encrypt_message(message);
        let decrypted_message = cipher.decrypt_message(&ciphertext2).unwrap();
        assert_eq!(decrypted_message, message);

        assert_ne!(ciphertext1, ciphertext2);

        let python_message = "HyLVZQAAAABVkcNp23ABVv3LJxZ6ru2HeryYbEY3joC1cKvP0yrVVzI7fJYbp7K7";
        let decrypted_message = cipher.decrypt_message(python_message);
        assert_eq!(
            decrypted_message.unwrap_err(),
            EncryptionError::ExpiredTimestamp
        );
    }
}
