use serde::{Deserialize, Serialize};

use super::client::UserInfo;

/// Client for ss13hub session token authentication.
pub struct HubClient {
    base_url: String,
    http: reqwest::Client,
}

#[derive(Serialize)]
struct LoginRequest {
    username_or_email: String,
    password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    totp_code: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub expire_time: String,
    pub user_id: String,
    pub username: String,
}

#[derive(Deserialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Deserialize)]
struct Requires2FAResponse {
    requires_2fa: Option<bool>,
}

impl HubClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
        }
    }

    fn from_config() -> Result<Self, String> {
        let config = crate::config::get_config();
        let base_url = config
            .urls
            .hub_api
            .ok_or("Hub API URL not configured")?;
        Ok(Self::new(base_url))
    }

    /// Log in with username/email and password. Returns session token and user info.
    pub async fn login(
        username_or_email: &str,
        password: &str,
        totp_code: Option<&str>,
    ) -> Result<LoginResponse, HubAuthError> {
        let client = Self::from_config().map_err(|e| HubAuthError::Config(e))?;

        let response = client
            .http
            .post(format!("{}/api/auth/login", client.base_url))
            .json(&LoginRequest {
                username_or_email: username_or_email.to_string(),
                password: password.to_string(),
                totp_code: totp_code.map(String::from),
            })
            .send()
            .await
            .map_err(|e| HubAuthError::Network(format!("Failed to connect: {e}")))?;

        let status = response.status();

        if status.is_success() {
            return response
                .json::<LoginResponse>()
                .await
                .map_err(|e| HubAuthError::Network(format!("Invalid response: {e}")));
        }

        let body = response.text().await.unwrap_or_default();

        // Check for 2FA requirement
        if status == reqwest::StatusCode::UNAUTHORIZED {
            if let Ok(r) = serde_json::from_str::<Requires2FAResponse>(&body) {
                if r.requires_2fa == Some(true) {
                    return Err(HubAuthError::Requires2FA);
                }
            }
        }

        let message = serde_json::from_str::<ErrorResponse>(&body)
            .map(|e| e.error)
            .unwrap_or_else(|_| format!("HTTP {status}"));

        match status {
            s if s == reqwest::StatusCode::UNAUTHORIZED => {
                Err(HubAuthError::InvalidCredentials)
            }
            s if s == reqwest::StatusCode::FORBIDDEN => {
                Err(HubAuthError::AccountLocked)
            }
            _ => Err(HubAuthError::Server(message)),
        }
    }

    /// Refresh a session token. Returns new token and expiry.
    pub async fn refresh(token: &str) -> Result<LoginResponse, HubAuthError> {
        let client = Self::from_config().map_err(|e| HubAuthError::Config(e))?;

        let response = client
            .http
            .post(format!("{}/api/auth/refresh", client.base_url))
            .header("Authorization", format!("SS13Auth {token}"))
            .send()
            .await
            .map_err(|e| HubAuthError::Network(format!("Failed to connect: {e}")))?;

        if !response.status().is_success() {
            return Err(HubAuthError::TokenExpired);
        }

        response
            .json::<LoginResponse>()
            .await
            .map_err(|e| HubAuthError::Network(format!("Invalid response: {e}")))
    }

    /// Request an auth ticket for connecting to a game server.
    pub async fn join(
        token: &str,
        server_ip: &str,
        server_port: i32,
        hwid: Option<&str>,
    ) -> Result<String, HubAuthError> {
        let client = Self::from_config().map_err(HubAuthError::Config)?;

        let response = client
            .http
            .post(format!("{}/api/session/join", client.base_url))
            .header("Authorization", format!("SS13Auth {token}"))
            .json(&serde_json::json!({
                "server_ip": server_ip,
                "server_port": server_port,
                "hwid": hwid,
            }))
            .send()
            .await
            .map_err(|e| HubAuthError::Network(format!("Failed to connect: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(HubAuthError::Server(format!("Join failed (HTTP {status}): {body}")));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| HubAuthError::Network(format!("Invalid response: {e}")))?;

        body["auth_ticket"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| HubAuthError::Server("missing auth_ticket in response".into()))
    }

    /// Fetch user profile using a session token.
    pub async fn get_profile(token: &str) -> Result<UserInfo, HubAuthError> {
        let client = Self::from_config().map_err(|e| HubAuthError::Config(e))?;

        let response = client
            .http
            .get(format!("{}/api/account", client.base_url))
            .header("Authorization", format!("SS13Auth {token}"))
            .send()
            .await
            .map_err(|e| HubAuthError::Network(format!("Failed to connect: {e}")))?;

        if !response.status().is_success() {
            return Err(HubAuthError::TokenExpired);
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| HubAuthError::Network(format!("Invalid response: {e}")))?;

        let user = &body["user"];
        Ok(UserInfo {
            sub: user["id"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            name: user["username"].as_str().map(String::from),
            preferred_username: user["username"].as_str().map(String::from),
            email: user["email"].as_str().map(String::from),
            email_verified: user["email_confirmed"].as_bool(),
        })
    }
}

#[derive(Debug)]
pub enum HubAuthError {
    InvalidCredentials,
    Requires2FA,
    AccountLocked,
    TokenExpired,
    Network(String),
    Server(String),
    Config(String),
}

impl std::fmt::Display for HubAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCredentials => write!(f, "Invalid username or password"),
            Self::Requires2FA => write!(f, "2FA code required"),
            Self::AccountLocked => write!(f, "Account is locked"),
            Self::TokenExpired => write!(f, "Session expired, please log in again"),
            Self::Network(msg) => write!(f, "{msg}"),
            Self::Server(msg) => write!(f, "{msg}"),
            Self::Config(msg) => write!(f, "{msg}"),
        }
    }
}
