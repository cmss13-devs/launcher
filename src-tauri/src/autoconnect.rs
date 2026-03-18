#[cfg(feature = "steam")]
mod implementation {
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;
    use tauri::{AppHandle, Emitter, Manager};

    use crate::auth::TokenStorage;
    use crate::byond::connect_to_server_internal;
    use crate::relays::RelayState;
    use crate::servers::{Server, ServerState};
    use crate::settings::{load_settings, AuthMode};
    use crate::steam::{authenticate_with_steam, SteamState};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum AutoConnectStatus {
        Starting,
        WaitingForServers,
        WaitingForRelays,
        ServerNotFound,
        ServerUnavailable,
        AuthRequired,
        SteamLinkingRequired,
        Connecting,
        Connected,
        Error,
    }

    #[derive(Debug, Clone, Serialize)]
    pub struct AutoConnectEvent {
        pub status: AutoConnectStatus,
        pub server_name: String,
        pub message: Option<String>,
        pub linking_url: Option<String>,
    }

    fn emit_status(
        handle: &AppHandle,
        server_name: &str,
        status: AutoConnectStatus,
        message: Option<String>,
        linking_url: Option<String>,
    ) {
        let event = AutoConnectEvent {
            status,
            server_name: server_name.to_string(),
            message,
            linking_url,
        };
        let _ = handle.emit("autoconnect-status", &event);
    }

    fn find_server(servers: &[Server], server_name: &str) -> Option<Server> {
        let normalized_name = server_name.replace('+', " ").to_lowercase();
        servers
            .iter()
            .find(|s| s.name.to_lowercase() == normalized_name)
            .cloned()
    }

    fn parse_server_url(url: &str) -> Option<String> {
        url.split(':').nth(1).map(|s| s.to_string())
    }

    async fn get_steam_access_token(handle: &AppHandle) -> Result<Option<String>, String> {
        let steam_state = match handle.try_state::<Arc<SteamState>>() {
            Some(state) => state,
            None => return Err("Steam not available".to_string()),
        };

        let result = authenticate_with_steam(&steam_state, false).await?;

        if result.success {
            Ok(result.access_token)
        } else if result.requires_linking {
            Err(format!(
                "LINKING_REQUIRED:{}",
                result.linking_url.unwrap_or_default()
            ))
        } else {
            Err(result
                .error
                .unwrap_or_else(|| "Steam authentication failed".to_string()))
        }
    }

    async fn get_access_token_for_mode(
        handle: &AppHandle,
        auth_mode: AuthMode,
    ) -> Result<(Option<String>, Option<String>), String> {
        match auth_mode {
            AuthMode::CmSs13 => {
                let tokens = TokenStorage::get_tokens()?;
                match tokens {
                    Some(t) if !TokenStorage::is_expired() => {
                        let config = crate::config::get_config();
                        Ok((Some(config.variant.to_string()), Some(t.access_token)))
                    }
                    _ => Err("AUTH_REQUIRED".to_string()),
                }
            }
            AuthMode::Steam => match get_steam_access_token(handle).await {
                Ok(token) => Ok((Some("steam".to_string()), token)),
                Err(e) => Err(e),
            },
            AuthMode::Byond => Ok((Some("byond".to_string()), None)),
        }
    }

    pub async fn perform_autoconnect(handle: AppHandle, server_name: String) {
        tracing::info!("Starting auto-connect to: {}", server_name);
        emit_status(
            &handle,
            &server_name,
            AutoConnectStatus::Starting,
            None,
            None,
        );

        let server_state = match handle.try_state::<Arc<ServerState>>() {
            Some(state) => state.inner().clone(),
            None => {
                tracing::error!("ServerState not available");
                emit_status(
                    &handle,
                    &server_name,
                    AutoConnectStatus::Error,
                    Some("Server state not available".to_string()),
                    None,
                );
                return;
            }
        };

        let servers = server_state.get_servers().await;
        if servers.is_empty() {
            tracing::warn!("No servers available yet");
            emit_status(
                &handle,
                &server_name,
                AutoConnectStatus::WaitingForServers,
                None,
                None,
            );
            emit_status(
                &handle,
                &server_name,
                AutoConnectStatus::Error,
                Some("No servers available".to_string()),
                None,
            );
            return;
        }

        let server = match find_server(&servers, &server_name) {
            Some(s) => s,
            None => {
                tracing::error!("Server not found: {}", server_name);
                emit_status(
                    &handle,
                    &server_name,
                    AutoConnectStatus::ServerNotFound,
                    Some(format!("Server \"{}\" not found", server_name)),
                    None,
                );
                return;
            }
        };

        if server.status != "available" {
            tracing::error!(
                "Server not available: {} (status: {})",
                server_name,
                server.status
            );
            emit_status(
                &handle,
                &server_name,
                AutoConnectStatus::ServerUnavailable,
                Some(format!(
                    "Server \"{}\" is currently unavailable",
                    server_name
                )),
                None,
            );
            return;
        }

        let port = match parse_server_url(&server.url) {
            Some(p) => p,
            None => {
                tracing::error!("Could not parse server URL: {}", server.url);
                emit_status(
                    &handle,
                    &server_name,
                    AutoConnectStatus::Error,
                    Some("Invalid server configuration".to_string()),
                    None,
                );
                return;
            }
        };

        let version = match &server.recommended_byond_version {
            Some(v) => v.clone(),
            None => {
                // Fall back to launcher's default BYOND version if configured
                match crate::config::get_config().default_byond_version {
                    Some(v) => v.to_string(),
                    None => {
                        tracing::error!("No BYOND version specified for server");
                        emit_status(
                            &handle,
                            &server_name,
                            AutoConnectStatus::Error,
                            Some("No BYOND version specified".to_string()),
                            None,
                        );
                        return;
                    }
                }
            }
        };

        let settings = match load_settings(&handle) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to load settings: {}", e);
                emit_status(
                    &handle,
                    &server_name,
                    AutoConnectStatus::Error,
                    Some("Failed to load settings".to_string()),
                    None,
                );
                return;
            }
        };

        let (access_type, access_token) =
            match get_access_token_for_mode(&handle, settings.auth_mode).await {
                Ok((t, tok)) => (t, tok),
                Err(e) if e == "AUTH_REQUIRED" => {
                    let config = crate::config::get_config();
                    tracing::info!("{} auth required", config.strings.auth_provider_name);
                    emit_status(
                        &handle,
                        &server_name,
                        AutoConnectStatus::AuthRequired,
                        Some("Please log in to continue".to_string()),
                        None,
                    );
                    return;
                }
                Err(e) if e.starts_with("LINKING_REQUIRED:") => {
                    let linking_url = e.strip_prefix("LINKING_REQUIRED:").map(|s| s.to_string());
                    tracing::info!("Steam linking required");
                    emit_status(
                        &handle,
                        &server_name,
                        AutoConnectStatus::SteamLinkingRequired,
                        Some("Steam account linking required".to_string()),
                        linking_url,
                    );
                    return;
                }
                Err(e) => {
                    tracing::error!("Auth error: {}", e);
                    emit_status(
                        &handle,
                        &server_name,
                        AutoConnectStatus::Error,
                        Some(e),
                        None,
                    );
                    return;
                }
            };

        let relay_state = match handle.try_state::<Arc<RelayState>>() {
            Some(state) => state.inner().clone(),
            None => {
                tracing::error!("RelayState not available");
                emit_status(
                    &handle,
                    &server_name,
                    AutoConnectStatus::Error,
                    Some("Relay state not available".to_string()),
                    None,
                );
                return;
            }
        };

        emit_status(
            &handle,
            &server_name,
            AutoConnectStatus::WaitingForRelays,
            None,
            None,
        );

        let mut attempts = 0;
        while !relay_state.all_relays_pinged().await {
            attempts += 1;
            if attempts >= 60 {
                tracing::error!("Timed out waiting for relays to be pinged");
                emit_status(
                    &handle,
                    &server_name,
                    AutoConnectStatus::Error,
                    Some("Timed out waiting for relays".to_string()),
                    None,
                );
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        let relay_host = match relay_state.get_selected_host().await {
            Some(host) => host,
            None => {
                tracing::error!("No relay selected after pinging");
                emit_status(
                    &handle,
                    &server_name,
                    AutoConnectStatus::Error,
                    Some("No relay available".to_string()),
                    None,
                );
                return;
            }
        };

        tracing::info!("Connecting to {} via {}", server_name, relay_host);
        emit_status(
            &handle,
            &server_name,
            AutoConnectStatus::Connecting,
            None,
            None,
        );

        let map_name = server.data.as_ref().map(|d| d.map_name.clone());

        match connect_to_server_internal(
            handle.clone(),
            version,
            relay_host,
            port,
            access_type,
            access_token,
            server_name.clone(),
            map_name,
            Some("autoconnect".to_string()),
        )
        .await
        {
            Ok(result) if result.success => {
                tracing::info!("Connection initiated successfully");
                emit_status(
                    &handle,
                    &server_name,
                    AutoConnectStatus::Connected,
                    None,
                    None,
                );
            }
            Ok(result) => {
                tracing::error!("Connection failed: {}", result.message);
                emit_status(
                    &handle,
                    &server_name,
                    AutoConnectStatus::Error,
                    Some(result.message),
                    None,
                );
            }
            Err(e) => {
                tracing::error!("Connection error: {}", e);
                emit_status(
                    &handle,
                    &server_name,
                    AutoConnectStatus::Error,
                    Some(e),
                    None,
                );
            }
        }
    }

    pub fn check_and_start_autoconnect(handle: AppHandle) {
        let steam_state = match handle.try_state::<Arc<SteamState>>() {
            Some(state) => state,
            None => {
                tracing::debug!("Steam not available, skipping auto-connect check");
                return;
            }
        };

        let launch_command = steam_state.get_launch_command_line();
        if launch_command.is_empty() {
            tracing::debug!("No Steam launch options");
            return;
        }

        let server_name = launch_command.trim().to_string();
        if server_name.is_empty() {
            return;
        }

        tracing::info!("Steam launch option detected: {}", server_name);

        // Spawn the auto-connect task
        tauri::async_runtime::spawn(async move {
            perform_autoconnect(handle, server_name).await;
        });
    }
}

#[cfg(feature = "steam")]
pub use implementation::check_and_start_autoconnect;

#[cfg(not(feature = "steam"))]
pub fn check_and_start_autoconnect(_handle: tauri::AppHandle) {
    tracing::debug!("Steam not compiled in, auto-connect disabled");
}
