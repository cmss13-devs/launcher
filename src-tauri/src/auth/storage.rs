use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::error::{CommandError, CommandResult};

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
    #[allow(clippy::indexing_slicing)] // i is bounded by take(32) and key is [u8; 32]
    fn get_encryption_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        let seed = b"cm-launcher-rs-token-encryption-key-v1";
        for (i, byte) in seed.iter().cycle().take(32).enumerate() {
            key[i] = *byte;
        }
        key
    }

    fn get_auth_file_path() -> CommandResult<PathBuf> {
        let config = crate::config::get_config();
        let data_dir = dirs::data_local_dir()
            .ok_or_else(|| CommandError::Io("local data directory unavailable".to_string()))?
            .join(config.app_identifier);

        fs::create_dir_all(&data_dir)?;

        Ok(data_dir.join(AUTH_FILE))
    }

    fn encrypt(data: &[u8]) -> CommandResult<Vec<u8>> {
        let key = Self::get_encryption_key();
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| CommandError::Internal(format!("Failed to create cipher: {e}")))?;

        let mut nonce_bytes = [0u8; NONCE_SIZE];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, data)
            .map_err(|e| CommandError::Internal(format!("Failed to encrypt data: {e}")))?;

        let mut result = Vec::with_capacity(NONCE_SIZE.saturating_add(ciphertext.len()));
        result.extend_from_slice(&nonce_bytes);
        result.extend(ciphertext);

        Ok(result)
    }

    #[allow(clippy::indexing_slicing)] // length checked above
    fn decrypt(data: &[u8]) -> CommandResult<Vec<u8>> {
        if data.len() < NONCE_SIZE {
            return Err(CommandError::InvalidResponse(
                "stored auth file is too short to be valid".to_string(),
            ));
        }

        let key = Self::get_encryption_key();
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| CommandError::Internal(format!("Failed to create cipher: {e}")))?;

        let nonce = Nonce::from_slice(&data[..NONCE_SIZE]);
        let ciphertext = &data[NONCE_SIZE..];

        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| CommandError::InvalidResponse(format!("Failed to decrypt data: {e}")))
    }

    pub fn store_tokens(
        access_token: &str,
        refresh_token: Option<&str>,
        id_token: &str,
        expires_at: i64,
    ) -> CommandResult<()> {
        let tokens = StoredTokens {
            access_token: access_token.to_string(),
            refresh_token: refresh_token.map(std::string::ToString::to_string),
            id_token: id_token.to_string(),
            expires_at,
        };

        let json = serde_json::to_vec(&tokens)
            .map_err(|e| CommandError::Internal(format!("Failed to serialize tokens: {e}")))?;

        let encrypted = Self::encrypt(&json)?;

        let path = Self::get_auth_file_path()?;
        fs::write(&path, &encrypted)?;

        tracing::debug!("Tokens stored securely");

        Ok(())
    }

    pub fn get_tokens() -> CommandResult<Option<StoredTokens>> {
        let path = Self::get_auth_file_path()?;

        if !path.exists() {
            return Ok(None);
        }

        let encrypted = fs::read(&path)?;

        let Ok(decrypted) = Self::decrypt(&encrypted) else {
            fs::remove_file(&path).ok();
            return Ok(None);
        };

        let tokens: StoredTokens = serde_json::from_slice(&decrypted).map_err(|e| {
            CommandError::InvalidResponse(format!("Failed to parse stored tokens: {e}"))
        })?;

        Ok(Some(tokens))
    }

    pub fn clear_tokens() -> CommandResult<()> {
        let path = Self::get_auth_file_path()?;

        if path.exists() {
            fs::remove_file(&path)?;
            tracing::debug!("Tokens cleared");
        }

        Ok(())
    }

    pub fn is_expired() -> bool {
        match Self::get_tokens() {
            Ok(Some(tokens)) => {
                let now = chrono::Utc::now().timestamp();
                tokens.expires_at <= now.saturating_add(60)
            }
            _ => true,
        }
    }

    pub fn should_refresh() -> bool {
        match Self::get_tokens() {
            Ok(Some(tokens)) => {
                let now = chrono::Utc::now().timestamp();
                tokens.expires_at <= now.saturating_add(300)
            }
            _ => false,
        }
    }
}
