use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use super::client::OidcClient;
pub use super::client::UserInfo;
use super::server::CallbackServer;
use super::storage::TokenStorage;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthState {
    pub logged_in: bool,
    pub user: Option<UserInfo>,
    pub loading: bool,
    pub error: Option<String>,
}

impl AuthState {
    pub fn logged_out() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn loading() -> Self {
        Self {
            loading: true,
            ..Default::default()
        }
    }

    pub fn logged_in(user: UserInfo) -> Self {
        Self {
            logged_in: true,
            user: Some(user),
            loading: false,
            error: None,
        }
    }

    #[allow(dead_code)]
    pub fn error(message: String) -> Self {
        Self {
            error: Some(message),
            ..Default::default()
        }
    }
}

#[tauri::command]
pub async fn start_login(app: AppHandle) -> Result<AuthState, String> {
    tracing::info!("Starting login flow");
    let mut server = CallbackServer::start_without_state()?;
    let redirect_uri = server.redirect_uri();

    let auth_request = OidcClient::create_authorization_request(&redirect_uri)?;

    server.set_expected_state(auth_request.state.clone());

    crate::open_url::open(&auth_request.auth_url)?;

    let callback_result = tokio::task::spawn_blocking(move || server.wait_for_callback())
        .await
        .map_err(|e| format!("Callback task failed: {e}"))??;

    tracing::info!("Callback received, exchanging code");

    let token_result = OidcClient::exchange_code(
        &callback_result.code,
        &redirect_uri,
        auth_request.pkce_verifier,
    )
    .await?;

    TokenStorage::store_tokens(
        &token_result.access_token,
        token_result.refresh_token.as_deref(),
        token_result.id_token.as_deref().unwrap_or(""),
        token_result.expires_at,
    )?;

    let user_info = OidcClient::get_userinfo(&token_result.access_token).await?;

    let auth_state = AuthState::logged_in(user_info);

    app.emit("auth-state-changed", &auth_state).ok();

    Ok(auth_state)
}

#[tauri::command]
pub async fn logout(app: AppHandle) -> Result<AuthState, String> {
    tracing::info!("Logging out");
    TokenStorage::clear_tokens()?;

    let auth_state = AuthState::logged_out();
    app.emit("auth-state-changed", &auth_state).ok();

    Ok(auth_state)
}

#[tauri::command]
pub async fn get_auth_state() -> Result<AuthState, String> {
    let Some(tokens) = TokenStorage::get_tokens()? else {
        return Ok(AuthState::logged_out());
    };

    if TokenStorage::is_expired() {
        if let Some(refresh_token) = &tokens.refresh_token {
            if let Ok(state) = refresh_tokens_internal(refresh_token).await {
                return Ok(state);
            }
            TokenStorage::clear_tokens()?;
            return Ok(AuthState::logged_out());
        }
        TokenStorage::clear_tokens()?;
        return Ok(AuthState::logged_out());
    }

    match OidcClient::get_userinfo(&tokens.access_token).await {
        Ok(user_info) => Ok(AuthState::logged_in(user_info)),
        Err(_) => {
            if let Some(refresh_token) = &tokens.refresh_token {
                refresh_tokens_internal(refresh_token).await
            } else {
                TokenStorage::clear_tokens()?;
                Ok(AuthState::logged_out())
            }
        }
    }
}

#[tauri::command]
pub async fn refresh_auth(app: AppHandle) -> Result<AuthState, String> {
    tracing::info!("Manually refreshing auth");
    let Some(tokens) = TokenStorage::get_tokens()? else {
        return Ok(AuthState::logged_out());
    };

    let refresh_token = tokens.refresh_token.ok_or("No refresh token available")?;

    let auth_state = refresh_tokens_internal(&refresh_token).await?;
    app.emit("auth-state-changed", &auth_state).ok();

    Ok(auth_state)
}

#[tauri::command]
pub async fn get_access_token() -> Result<Option<String>, String> {
    match TokenStorage::get_tokens()? {
        Some(tokens) if !TokenStorage::is_expired() => Ok(Some(tokens.access_token)),
        _ => Ok(None),
    }
}

async fn refresh_tokens_internal(refresh_token: &str) -> Result<AuthState, String> {
    let token_result = OidcClient::refresh_tokens(refresh_token).await?;

    TokenStorage::store_tokens(
        &token_result.access_token,
        token_result.refresh_token.as_deref(),
        token_result.id_token.as_deref().unwrap_or(""),
        token_result.expires_at,
    )?;

    let user_info = OidcClient::get_userinfo(&token_result.access_token).await?;

    Ok(AuthState::logged_in(user_info))
}

pub async fn background_refresh_task(app: AppHandle) {
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    loop {
        if TokenStorage::should_refresh() {
            if let Ok(Some(tokens)) = TokenStorage::get_tokens() {
                if let Some(refresh_token) = &tokens.refresh_token {
                    tracing::info!("Background refreshing tokens");
                    match refresh_tokens_internal(refresh_token).await {
                        Ok(auth_state) => {
                            app.emit("auth-state-changed", &auth_state).ok();
                        }
                        Err(e) => {
                            tracing::warn!("Background refresh failed: {}", e);
                            TokenStorage::clear_tokens().ok();
                            app.emit("auth-state-changed", &AuthState::logged_out())
                                .ok();
                        }
                    }
                }
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}
