use crate::config::get_config;
use crate::settings::load_settings;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::RwLock;

fn get_server_api_url() -> &'static str {
    get_config().urls.server_api
}
const SERVER_FETCH_INTERVAL_SECS: u64 = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerData {
    pub round_id: i64,
    pub mode: String,
    pub map_name: String,
    pub round_duration: f64,
    pub gamestate: i32,
    pub players: i32,
    #[serde(default)]
    pub admins: Option<i32>,
    #[serde(default)]
    pub popcap: Option<i32>,
    #[serde(default)]
    pub security_level: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub name: String,
    pub url: String,
    pub status: String,
    #[serde(default)]
    pub hub_status: String,
    #[serde(default)]
    pub players: i32,
    #[serde(default)]
    pub data: Option<ServerData>,
    #[serde(default)]
    pub is_18_plus: bool,
    #[serde(default)]
    pub version: Option<String>,
    pub recommended_byond_version: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

// Hub API response structure (topic_status can have arbitrary fields)
#[derive(Debug, Clone, Deserialize)]
struct HubServer {
    address: String,
    name: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    topic_status: Option<Value>,
    online: bool,
    #[serde(default)]
    players: i32,
}

// CM-SS13 API response wrapper
#[derive(Debug, Clone, Deserialize)]
struct CmApiResponse {
    servers: Vec<CmServer>,
}

// CM-SS13 API server structure
#[derive(Debug, Clone, Deserialize)]
struct CmServer {
    name: String,
    url: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    recommended_byond_version: Option<String>,
    #[serde(default)]
    data: Option<CmServerData>,
    #[serde(default)]
    tags: Option<Vec<String>>,
}

// CM-SS13 API server data (nested under "data")
#[derive(Debug, Clone, Deserialize)]
struct CmServerData {
    #[serde(default)]
    round_id: Option<i64>,
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    map_name: Option<String>,
    #[serde(default)]
    round_duration: Option<f64>,
    #[serde(default)]
    gamestate: Option<i32>,
    #[serde(default)]
    players: Option<i32>,
    #[serde(default)]
    admins: Option<i32>,
    #[serde(default)]
    security_level: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerUpdateEvent {
    pub servers: Vec<Server>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerErrorEvent {
    pub error: String,
}

#[derive(Debug, Clone, Default)]
struct PreviousServerState {
    was_online: bool,
    round_id: Option<i64>,
}

#[derive(Debug, Default)]
pub struct ServerState {
    servers: RwLock<Vec<Server>>,
    previous_states: RwLock<HashMap<String, PreviousServerState>>,
}

impl ServerState {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn get_servers(&self) -> Vec<Server> {
        self.servers.read().await.clone()
    }
}

async fn fetch_servers_internal() -> Result<Vec<Server>, String> {
    let config = get_config();

    let response = reqwest::get(get_server_api_url())
        .await
        .map_err(|e| format!("Failed to fetch servers: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    // Use different parsing based on which API we're using
    if config.features.hub_server_list {
        // Hub API format
        let hub_servers: Vec<HubServer> = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse server response: {}", e))?;

        let servers = hub_servers
            .into_iter()
            .map(|hub| {
                let data = hub.topic_status.as_ref().and_then(|ts| {
                    ts.get("round_id").and_then(|v| v.as_i64()).map(|round_id| ServerData {
                        round_id,
                        mode: ts.get("mode").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        map_name: ts.get("map_name")
                            .and_then(|v| v.as_str())
                            .or_else(|| ts.get("map").and_then(|v| v.as_str()))
                            .unwrap_or("")
                            .to_string(),
                        round_duration: ts.get("round_duration").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        gamestate: ts.get("gamestate").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                        players: ts.get("players").and_then(|v| v.as_i64()).unwrap_or(hub.players as i64) as i32,
                        admins: ts.get("admins").and_then(|v| v.as_i64()).map(|v| v as i32),
                        popcap: ts.get("popcap").and_then(|v| v.as_i64()).map(|v| v as i32),
                        security_level: ts.get("security_level").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    })
                });

                let address = hub.topic_status.as_ref()
                    .and_then(|ts| ts.get("public_address").and_then(|v| v.as_str()))
                    .unwrap_or(&hub.address);

                let version = hub.topic_status.as_ref()
                    .and_then(|ts| ts.get("version").and_then(|v| v.as_str()))
                    .map(|s| s.to_string());

                let is_18_plus = hub.status.contains("18+");

                Server {
                    name: hub.name,
                    url: format!("byond://{}", address),
                    status: if hub.online { "available".to_string() } else { "offline".to_string() },
                    hub_status: hub.status,
                    players: hub.players,
                    data,
                    is_18_plus,
                    version,
                    recommended_byond_version: None,
                    tags: Vec::new(),
                }
            })
            .collect();

        Ok(servers)
    } else {
        // CM-SS13 API format (wrapped in {"servers": [...]})
        let cm_response: CmApiResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse server response: {}", e))?;

        let servers = cm_response.servers
            .into_iter()
            .map(|cm| {
                let players = cm.data.as_ref()
                    .and_then(|d| d.players)
                    .unwrap_or(0);

                let data = cm.data.as_ref().and_then(|d| {
                    d.round_id.map(|round_id| ServerData {
                        round_id,
                        mode: d.mode.clone().unwrap_or_default(),
                        map_name: d.map_name.clone().unwrap_or_default(),
                        round_duration: d.round_duration.unwrap_or(0.0),
                        gamestate: d.gamestate.unwrap_or(0),
                        players,
                        admins: d.admins,
                        popcap: None,
                        security_level: d.security_level.clone(),
                    })
                });

                Server {
                    name: cm.name,
                    url: format!("byond://{}", cm.url),
                    status: cm.status.clone(),
                    hub_status: String::new(),
                    players,
                    data,
                    is_18_plus: false,
                    version: None,
                    recommended_byond_version: cm.recommended_byond_version,
                    tags: cm.tags.unwrap_or_default(),
                }
            })
            .collect();

        Ok(servers)
    }
}

/// Fetch servers and populate the cache. Called during app setup.
pub async fn init_servers(state: &Arc<ServerState>) {
    match fetch_servers_internal().await {
        Ok(servers) => {
            let mut previous_states = state.previous_states.write().await;
            for server in &servers {
                let is_online = server.status == "available";
                let round_id = server.data.as_ref().map(|d| d.round_id);
                previous_states.insert(
                    server.name.clone(),
                    PreviousServerState {
                        was_online: is_online,
                        round_id,
                    },
                );
            }
            drop(previous_states);

            *state.servers.write().await = servers;
            tracing::info!("Initial server fetch complete");
        }
        Err(e) => {
            tracing::error!("Initial server fetch failed: {}", e);
        }
    }
}

#[tauri::command]
pub async fn get_servers(state: tauri::State<'_, Arc<ServerState>>) -> Result<Vec<Server>, String> {
    Ok(state.servers.read().await.clone())
}

pub async fn server_fetch_background_task(handle: AppHandle, state: Arc<ServerState>) {
    loop {
        tokio::time::sleep(Duration::from_secs(SERVER_FETCH_INTERVAL_SECS)).await;

        match fetch_servers_internal().await {
            Ok(servers) => {
                // Check for notification triggers before updating state
                check_and_send_notifications(&handle, &state, &servers).await;

                *state.servers.write().await = servers.clone();
                let _ = handle.emit("servers-updated", ServerUpdateEvent { servers });
            }
            Err(error) => {
                tracing::error!("Server fetch error: {}", error);
                let _ = handle.emit("servers-error", ServerErrorEvent { error });
            }
        }
    }
}

async fn check_and_send_notifications(
    handle: &AppHandle,
    state: &Arc<ServerState>,
    new_servers: &[Server],
) {
    let notification_servers = match load_settings(handle) {
        Ok(settings) => settings.notification_servers,
        Err(e) => {
            tracing::warn!("Failed to load settings for notifications: {}", e);
            return;
        }
    };

    if notification_servers.is_empty() {
        return;
    }

    let mut previous_states = state.previous_states.write().await;

    for server in new_servers {
        if !notification_servers.contains(&server.name) {
            continue;
        }

        let is_online = server.status == "available";
        let current_round_id = server.data.as_ref().map(|d| d.round_id);

        let prev = previous_states
            .entry(server.name.clone())
            .or_insert_with(|| PreviousServerState {
                was_online: is_online,
                round_id: current_round_id,
            });

        let mut should_notify = false;
        let mut notification_title = String::new();
        let mut notification_body = String::new();

        if is_online && !prev.was_online {
            should_notify = true;
            notification_title = format!("{} is now online", server.name);
            notification_body = "The server is available to join.".to_string();
        } else if is_online {
            if let (Some(current), Some(previous)) = (current_round_id, prev.round_id) {
                if current > previous {
                    should_notify = true;
                    notification_title = format!("{} has restarted", server.name);
                    if let Some(data) = &server.data {
                        notification_body = format!("Round #{} - {}", data.round_id, data.map_name);
                    } else {
                        notification_body = format!("Round #{}", current);
                    }
                }
            }
        }

        prev.was_online = is_online;
        prev.round_id = current_round_id;

        if should_notify {
            let mut builder = handle
                .notification()
                .builder()
                .title(&notification_title)
                .body(&notification_body);

            if let Ok(resource_path) = handle.path().resource_dir() {
                let icon_path = resource_path.join("icons").join("icon.png");
                if icon_path.exists() {
                    builder = builder.icon(icon_path.to_string_lossy().to_string());
                }
            }

            if let Err(e) = builder.show() {
                tracing::warn!("Failed to send notification: {}", e);
            } else {
                tracing::info!(
                    "Sent notification for {}: {}",
                    server.name,
                    notification_title
                );
            }
        }
    }
}
