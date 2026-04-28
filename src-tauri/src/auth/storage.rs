use serde::{Deserialize, Serialize};

use crate::error::{CommandError, CommandResult};

const KEYRING_USER: &str = "auth_tokens";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub id_token: String,
    pub expires_at: i64,
}

pub struct TokenStorage;

impl TokenStorage {
    fn entry() -> CommandResult<keyring::Entry> {
        let config = crate::config::get_config();
        keyring::Entry::new(config.app_identifier, KEYRING_USER)
            .map_err(|e| CommandError::Io(format!("failed to create keyring entry: {e}")))
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

        let json = serde_json::to_string(&tokens)
            .map_err(|e| CommandError::Io(format!("failed to serialize tokens: {e}")))?;

        let entry = Self::entry()?;
        entry
            .set_password(&json)
            .map_err(|e| CommandError::Io(format!("failed to store tokens in keychain: {e}")))?;

        tracing::debug!("Tokens stored in OS keychain");

        Ok(())
    }

    pub fn get_tokens() -> CommandResult<Option<StoredTokens>> {
        let entry = Self::entry()?;

        let json = match entry.get_password() {
            Ok(json) => json,
            Err(keyring::Error::NoEntry) => return Ok(None),
            Err(e) => {
                tracing::warn!("Failed to read tokens from keychain: {e}");
                return Ok(None);
            }
        };

        let tokens: StoredTokens = serde_json::from_str(&json).map_err(|e| {
            CommandError::InvalidResponse(format!("Failed to parse stored tokens: {e}"))
        })?;

        Ok(Some(tokens))
    }

    pub fn clear_tokens() -> CommandResult<()> {
        let entry = Self::entry()?;

        match entry.delete_credential() {
            Ok(()) => tracing::debug!("Tokens cleared from keychain"),
            Err(keyring::Error::NoEntry) => {}
            Err(e) => {
                return Err(CommandError::Io(format!(
                    "failed to clear tokens from keychain: {e}"
                )));
            }
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
