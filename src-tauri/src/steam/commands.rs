use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::error::{CommandError, CommandResult};
use crate::steam::get_steam_app_name;

use super::SteamState;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct SteamUserInfo {
    pub steam_id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct SteamLaunchOptions {
    pub raw: String,
    pub server_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct SteamAuthResult {
    pub success: bool,
    pub user_exists: bool,
    pub access_token: Option<String>,
    pub requires_linking: bool,
    pub linking_url: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SteamAuthRequest {
    ticket: String,
    steam_id: String,
    instance: String,
    display_name: String,
    create_account_if_missing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SteamAuthResponse {
    success: bool,
    user_exists: bool,
    access_token: Option<String>,
    requires_linking: bool,
    linking_url: Option<String>,
    error: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub async fn get_steam_user_info(
    steam_state: State<'_, Arc<SteamState>>,
) -> CommandResult<SteamUserInfo> {
    let steam_id = steam_state.get_steam_id().to_string();
    let display_name = steam_state.get_display_name();

    Ok(SteamUserInfo {
        steam_id,
        display_name,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn get_steam_auth_ticket(
    steam_state: State<'_, Arc<SteamState>>,
) -> CommandResult<String> {
    tracing::debug!("Generating Steam auth ticket");
    let ticket_bytes = steam_state.get_auth_session_ticket().await?;
    Ok(hex::encode(ticket_bytes))
}

#[tauri::command]
#[specta::specta]
pub async fn cancel_steam_auth_ticket(
    steam_state: State<'_, Arc<SteamState>>,
) -> CommandResult<()> {
    tracing::debug!("Cancelling Steam auth ticket");
    steam_state.cancel_auth_ticket();
    Ok(())
}

pub async fn authenticate_with_steam(
    steam_state: &Arc<SteamState>,
    create_account_if_missing: bool,
) -> CommandResult<SteamAuthResult> {
    tracing::info!("Starting Steam authentication");
    let steam_id = steam_state.get_steam_id().to_string();
    let display_name = steam_state.get_display_name();

    let ticket_bytes = steam_state.get_auth_session_ticket().await?;
    let ticket = hex::encode(&ticket_bytes);

    let client = reqwest::Client::new();
    let request = SteamAuthRequest {
        ticket,
        steam_id,
        instance: get_steam_app_name(),
        display_name,
        create_account_if_missing,
    };

    let config = crate::config::get_config();
    let steam_auth_url = config.urls.steam_auth.ok_or(CommandError::NotConfigured {
        feature: "steam_auth".to_string(),
    })?;

    let response = client.post(steam_auth_url).json(&request).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        steam_state.cancel_auth_ticket();
        return Err(CommandError::InvalidResponse(format!(
            "Auth server error ({status}): {body}"
        )));
    }

    let auth_response: SteamAuthResponse = response.json().await.map_err(|e| {
        CommandError::InvalidResponse(format!("Failed to parse auth response: {e}"))
    })?;

    if !auth_response.success {
        steam_state.cancel_auth_ticket();
    }

    if auth_response.access_token.is_some() {
        tracing::debug!("Received access token from Steam auth");
    }

    Ok(SteamAuthResult {
        success: auth_response.success,
        user_exists: auth_response.user_exists,
        access_token: auth_response.access_token,
        requires_linking: auth_response.requires_linking,
        linking_url: auth_response.linking_url,
        error: auth_response.error,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn steam_authenticate(
    steam_state: State<'_, Arc<SteamState>>,
    create_account_if_missing: bool,
) -> CommandResult<SteamAuthResult> {
    authenticate_with_steam(&steam_state, create_account_if_missing).await
}

fn parse_server_name(command_line: &str) -> Option<String> {
    let trimmed = command_line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let decoded = trimmed.replace('+', " ");
    let decoded = percent_encoding::percent_decode_str(&decoded)
        .decode_utf8_lossy()
        .to_string();
    if decoded.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn get_steam_launch_options(
    steam_state: State<'_, Arc<SteamState>>,
) -> CommandResult<SteamLaunchOptions> {
    let raw = steam_state.get_launch_command_line();
    let server_name = parse_server_name(&raw);

    Ok(SteamLaunchOptions { raw, server_name })
}
