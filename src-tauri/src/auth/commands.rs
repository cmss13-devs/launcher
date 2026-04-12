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
        // Try to refresh
        let token_to_use = tokens
            .refresh_token
            .as_deref()
            .unwrap_or(&tokens.access_token);

        if let Ok(state) = refresh_tokens_internal(token_to_use).await {
            return Ok(state);
        }
        TokenStorage::clear_tokens()?;
        return Ok(AuthState::logged_out());
    }

    let config = crate::config::get_config();
    if config.urls.hub_api.is_some() {
        // Hub auth: refresh on launch to reset the 28-day expiry window.
        // If refresh fails (e.g. network issue), fall back to validating
        // the existing token so we don't log the user out unnecessarily.
        use super::hub_client::HubClient;
        match refresh_tokens_internal(&tokens.access_token).await {
            Ok(state) => Ok(state),
            Err(_) => {
                if let Ok(user_info) = HubClient::get_profile(&tokens.access_token).await {
                    Ok(AuthState::logged_in(user_info))
                } else {
                    TokenStorage::clear_tokens()?;
                    Ok(AuthState::logged_out())
                }
            }
        }
    } else {
        // OIDC auth: fetch userinfo with access token
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

// -- Hub (ss13hub session token) auth --

#[tauri::command]
pub async fn hub_login(
    app: AppHandle,
    username: String,
    password: String,
    totp_code: Option<String>,
) -> Result<AuthState, String> {
    use super::hub_client::{HubAuthError, HubClient};

    tracing::info!("Starting hub login for {}", username);

    let result = HubClient::login(&username, &password, totp_code.as_deref())
        .await
        .map_err(|e| match e {
            HubAuthError::Requires2FA => "requires_2fa".to_string(),
            other => other.to_string(),
        })?;

    // Parse expiry from ISO 8601 string
    let expires_at = chrono::DateTime::parse_from_rfc3339(&result.expire_time)
        .map(|dt| dt.timestamp())
        .unwrap_or_else(|_| chrono::Utc::now().timestamp() + 86400 * 30);

    // Store session token (using access_token field, no refresh token or id_token)
    TokenStorage::store_tokens(&result.token, None, "", expires_at)?;

    // Fetch full profile
    let user_info = HubClient::get_profile(&result.token)
        .await
        .map_err(|e| e.to_string())?;

    let auth_state = AuthState::logged_in(user_info);
    app.emit("auth-state-changed", &auth_state).ok();

    Ok(auth_state)
}

#[tauri::command]
pub async fn get_hub_oauth_providers() -> Result<Vec<String>, String> {
    use super::hub_client::HubClient;

    let config = HubClient::get_hub_config()
        .await
        .map_err(|e| e.to_string())?;

    let providers = config["oauth_providers"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    Ok(providers)
}

#[tauri::command]
pub async fn hub_oauth_login(app: AppHandle, provider: String) -> Result<AuthState, String> {
    use super::hub_client::HubClient;

    tracing::info!("Starting hub OAuth login for provider: {}", provider);

    let config = crate::config::get_config();
    let hub_api = config
        .urls
        .hub_api
        .ok_or("Hub API URL not configured")?;

    let server = CallbackServer::start_without_state()?;
    let redirect_uri = server.redirect_uri();

    let encoded_redirect: String =
        url::form_urlencoded::byte_serialize(redirect_uri.as_bytes()).collect();
    let authorize_url = format!(
        "{}/api/auth/oauth/{}/authorize?redirect_after={}",
        hub_api.trim_end_matches('/'),
        provider,
        encoded_redirect,
    );

    crate::open_url::open(&authorize_url)?;

    let callback_result = tokio::task::spawn_blocking(move || server.wait_for_callback())
        .await
        .map_err(|e| format!("Callback task failed: {e}"))??;

    tracing::info!("OAuth callback received, exchanging code");

    let result = HubClient::oauth_exchange(&callback_result.code)
        .await
        .map_err(|e| e.to_string())?;

    let expires_at = chrono::DateTime::parse_from_rfc3339(&result.expire_time)
        .map(|dt| dt.timestamp())
        .unwrap_or_else(|_| chrono::Utc::now().timestamp() + 86400 * 30);

    TokenStorage::store_tokens(&result.token, None, "", expires_at)?;

    let user_info = HubClient::get_profile(&result.token)
        .await
        .map_err(|e| e.to_string())?;

    let auth_state = AuthState::logged_in(user_info);
    app.emit("auth-state-changed", &auth_state).ok();

    Ok(auth_state)
}

// -- Shared internals --

async fn refresh_tokens_internal(token: &str) -> Result<AuthState, String> {
    let config = crate::config::get_config();

    if config.urls.hub_api.is_some() {
        // Hub auth: refresh via ss13hub
        use super::hub_client::HubClient;

        let result = HubClient::refresh(token).await.map_err(|e| e.to_string())?;

        let expires_at = chrono::DateTime::parse_from_rfc3339(&result.expire_time).map_or_else(
            |_| chrono::Utc::now().timestamp() + 86400 * 30,
            |dt| dt.timestamp(),
        );

        TokenStorage::store_tokens(&result.token, None, "", expires_at)?;

        let user_info = HubClient::get_profile(&result.token)
            .await
            .map_err(|e| e.to_string())?;

        Ok(AuthState::logged_in(user_info))
    } else {
        // OIDC auth: refresh via OIDC provider
        let token_result = OidcClient::refresh_tokens(token).await?;

        TokenStorage::store_tokens(
            &token_result.access_token,
            token_result.refresh_token.as_deref(),
            token_result.id_token.as_deref().unwrap_or(""),
            token_result.expires_at,
        )?;

        let user_info = OidcClient::get_userinfo(&token_result.access_token).await?;

        Ok(AuthState::logged_in(user_info))
    }
}

pub async fn background_refresh_task(app: AppHandle) {
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    loop {
        if TokenStorage::should_refresh() {
            if let Ok(Some(tokens)) = TokenStorage::get_tokens() {
                tracing::info!("Background refreshing tokens");
                // For hub auth, access_token IS the session token to refresh
                // For OIDC auth, we need the refresh_token
                let token_to_use = tokens
                    .refresh_token
                    .as_deref()
                    .unwrap_or(&tokens.access_token);

                match refresh_tokens_internal(token_to_use).await {
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

        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}
