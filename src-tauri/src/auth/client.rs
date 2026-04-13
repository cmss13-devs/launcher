use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, RefreshToken, Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};

use crate::config::{get_config, OidcConfig};
use crate::error::{CommandError, CommandResult};

fn get_oidc_config() -> CommandResult<OidcConfig> {
    get_config().oidc.ok_or_else(|| CommandError::NotConfigured {
        feature: "oidc".to_string(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct UserInfo {
    pub sub: String,
    pub name: Option<String>,
    pub preferred_username: Option<String>,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
}

pub struct AuthorizationRequest {
    pub auth_url: String,
    pub state: String,
    pub pkce_verifier: PkceCodeVerifier,
}

pub struct TokenResult {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub expires_at: i64,
}

fn http_client() -> CommandResult<oauth2::reqwest::Client> {
    oauth2::reqwest::ClientBuilder::new()
        .redirect(oauth2::reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| CommandError::Network(format!("Failed to build HTTP client: {e}")))
}

pub struct OidcClient;

impl OidcClient {
    pub fn create_authorization_request(
        redirect_uri_string: &str,
    ) -> CommandResult<AuthorizationRequest> {
        tracing::debug!(
            "Creating authorization request with redirect_uri: {}",
            redirect_uri_string
        );
        let oidc = get_oidc_config()?;
        let auth_url = AuthUrl::new(oidc.auth_url.to_string())
            .map_err(|e| CommandError::NotConfigured {
                feature: format!("oidc.auth_url ({e})"),
            })?;
        let token_url = TokenUrl::new(oidc.token_url.to_string())
            .map_err(|e| CommandError::NotConfigured {
                feature: format!("oidc.token_url ({e})"),
            })?;
        let redirect_url = RedirectUrl::new(redirect_uri_string.to_string())
            .map_err(|e| CommandError::Internal(format!("Invalid redirect URI: {e}")))?;

        let client = BasicClient::new(ClientId::new(oidc.client_id.to_string()))
            .set_auth_uri(auth_url)
            .set_token_uri(token_url)
            .set_redirect_uri(redirect_url);

        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (auth_url, csrf_token) = client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("offline_access".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        Ok(AuthorizationRequest {
            auth_url: auth_url.to_string(),
            state: csrf_token.secret().clone(),
            pkce_verifier,
        })
    }

    #[allow(clippy::similar_names)]
    pub async fn exchange_code(
        code: &str,
        redirect_uri: &str,
        pkce_verifier: PkceCodeVerifier,
    ) -> CommandResult<TokenResult> {
        tracing::debug!("Exchanging authorization code for tokens");
        let oidc = get_oidc_config()?;
        let auth_url = AuthUrl::new(oidc.auth_url.to_string())
            .map_err(|e| CommandError::NotConfigured {
                feature: format!("oidc.auth_url ({e})"),
            })?;
        let token_url = TokenUrl::new(oidc.token_url.to_string())
            .map_err(|e| CommandError::NotConfigured {
                feature: format!("oidc.token_url ({e})"),
            })?;
        let redirect_url = RedirectUrl::new(redirect_uri.to_string())
            .map_err(|e| CommandError::Internal(format!("Invalid redirect URI: {e}")))?;

        let client = BasicClient::new(ClientId::new(oidc.client_id.to_string()))
            .set_auth_uri(auth_url)
            .set_token_uri(token_url)
            .set_redirect_uri(redirect_url);

        let http = http_client()?;

        let token_response = client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .set_pkce_verifier(pkce_verifier)
            .request_async(&http)
            .await
            .map_err(|e| {
                tracing::warn!("OIDC code exchange failed: {e}");
                CommandError::InvalidCredentials
            })?;

        let access_token = token_response.access_token().secret().clone();

        let refresh_token = token_response.refresh_token().map(|t| t.secret().clone());

        let id_token = None;

        let expires_in = token_response
            .expires_in()
            .unwrap_or(std::time::Duration::from_secs(3600));
        #[allow(clippy::cast_possible_wrap)]
        let expires_at = chrono::Utc::now()
            .timestamp()
            .saturating_add(expires_in.as_secs() as i64);

        Ok(TokenResult {
            access_token,
            refresh_token,
            id_token,
            expires_at,
        })
    }

    #[allow(clippy::similar_names)]
    pub async fn refresh_tokens(refresh_token: &str) -> CommandResult<TokenResult> {
        tracing::debug!("Refreshing tokens");
        let oidc = get_oidc_config()?;
        let auth_url = AuthUrl::new(oidc.auth_url.to_string())
            .map_err(|e| CommandError::NotConfigured {
                feature: format!("oidc.auth_url ({e})"),
            })?;
        let token_url = TokenUrl::new(oidc.token_url.to_string())
            .map_err(|e| CommandError::NotConfigured {
                feature: format!("oidc.token_url ({e})"),
            })?;

        let client = BasicClient::new(ClientId::new(oidc.client_id.to_string()))
            .set_auth_uri(auth_url)
            .set_token_uri(token_url);

        let http = http_client()?;

        let token_response = client
            .exchange_refresh_token(&RefreshToken::new(refresh_token.to_string()))
            .request_async(&http)
            .await
            .map_err(|e| {
                tracing::warn!("OIDC token refresh failed: {e}");
                CommandError::TokenExpired
            })?;

        let access_token = token_response.access_token().secret().clone();

        let new_refresh_token = token_response
            .refresh_token()
            .map(|t| t.secret().clone())
            .or_else(|| Some(refresh_token.to_string()));

        let expires_in = token_response
            .expires_in()
            .unwrap_or(std::time::Duration::from_secs(3600));
        #[allow(clippy::cast_possible_wrap)]
        let expires_at = chrono::Utc::now()
            .timestamp()
            .saturating_add(expires_in.as_secs() as i64);

        Ok(TokenResult {
            access_token,
            refresh_token: new_refresh_token,
            id_token: None,
            expires_at,
        })
    }

    pub async fn get_userinfo(access_token: &str) -> CommandResult<UserInfo> {
        tracing::debug!("Fetching user info");
        let oidc = get_oidc_config()?;
        let client = reqwest::Client::new();
        let response = client
            .get(oidc.userinfo_url)
            .bearer_auth(access_token)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            if status == reqwest::StatusCode::UNAUTHORIZED {
                return Err(CommandError::TokenExpired);
            }
            return Err(CommandError::InvalidResponse(format!(
                "Userinfo request failed: {status}"
            )));
        }

        let userinfo: UserInfo = response.json().await.map_err(|e| {
            CommandError::InvalidResponse(format!("Failed to parse userinfo: {e}"))
        })?;

        Ok(userinfo)
    }
}
