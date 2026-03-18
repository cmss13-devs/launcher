use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const AUTH_FILE: &str = "auth.dat";
const NONCE_SIZE: usize = 12;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub id_token: String,
    pub expires_at: i64,
}

pub struct TokenStorage;

impl TokenStorage {
    fn get_encryption_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        let seed = b"cm-launcher-rs-token-encryption-key-v1";
        for (i, byte) in seed.iter().cycle().take(32).enumerate() {
            key[i] = *byte;
        }
        key
    }

    fn get_auth_file_path() -> Result<PathBuf, String> {
        let config = crate::config::get_config();
        let data_dir = dirs::data_local_dir()
            .ok_or("Failed to get local data directory")?
            .join(config.app_identifier);

        fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create data directory: {}", e))?;

        Ok(data_dir.join(AUTH_FILE))
    }

    fn encrypt(data: &[u8]) -> Result<Vec<u8>, String> {
        let key = Self::get_encryption_key();
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| format!("Failed to create cipher: {}", e))?;

        let mut nonce_bytes = [0u8; NONCE_SIZE];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, data)
            .map_err(|e| format!("Failed to encrypt data: {}", e))?;

        let mut result = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend(ciphertext);

        Ok(result)
    }

    fn decrypt(data: &[u8]) -> Result<Vec<u8>, String> {
        if data.len() < NONCE_SIZE {
            return Err("Invalid encrypted data: too short".to_string());
        }

        let key = Self::get_encryption_key();
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| format!("Failed to create cipher: {}", e))?;

        let nonce = Nonce::from_slice(&data[..NONCE_SIZE]);
        let ciphertext = &data[NONCE_SIZE..];

        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| format!("Failed to decrypt data: {}", e))
    }

    pub fn store_tokens(
        access_token: &str,
        refresh_token: Option<&str>,
        id_token: &str,
        expires_at: i64,
    ) -> Result<(), String> {
        let tokens = StoredTokens {
            access_token: access_token.to_string(),
            refresh_token: refresh_token.map(|s| s.to_string()),
            id_token: id_token.to_string(),
            expires_at,
        };

        let json = serde_json::to_vec(&tokens)
            .map_err(|e| format!("Failed to serialize tokens: {}", e))?;

        let encrypted = Self::encrypt(&json)?;

        let path = Self::get_auth_file_path()?;
        fs::write(&path, &encrypted).map_err(|e| format!("Failed to write auth file: {}", e))?;

        tracing::debug!("Tokens stored securely");

        Ok(())
    }

    pub fn get_tokens() -> Result<Option<StoredTokens>, String> {
        let path = Self::get_auth_file_path()?;

        if !path.exists() {
            return Ok(None);
        }

        let encrypted = fs::read(&path).map_err(|e| format!("Failed to read auth file: {}", e))?;

        let decrypted = match Self::decrypt(&encrypted) {
            Ok(data) => data,
            Err(_) => {
                fs::remove_file(&path).ok();
                return Ok(None);
            }
        };

        let tokens: StoredTokens = serde_json::from_slice(&decrypted)
            .map_err(|e| format!("Failed to parse tokens: {}", e))?;

        Ok(Some(tokens))
    }

    pub fn clear_tokens() -> Result<(), String> {
        let path = Self::get_auth_file_path()?;

        if path.exists() {
            fs::remove_file(&path).map_err(|e| format!("Failed to delete auth file: {}", e))?;
            tracing::debug!("Tokens cleared");
        }

        Ok(())
    }

    pub fn is_expired() -> bool {
        match Self::get_tokens() {
            Ok(Some(tokens)) => {
                let now = chrono::Utc::now().timestamp();
                tokens.expires_at <= now + 60
            }
            _ => true,
        }
    }

    pub fn should_refresh() -> bool {
        match Self::get_tokens() {
            Ok(Some(tokens)) => {
                let now = chrono::Utc::now().timestamp();
                tokens.expires_at <= now + 300
            }
            _ => false,
        }
    }
}
