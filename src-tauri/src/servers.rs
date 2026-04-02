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

trait ServerApi: Send + Sync {
    fn parse(&self, body: &str) -> Result<Vec<Server>, String>;
}
struct HubApi;

#[derive(Debug, Deserialize)]
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

impl ServerApi for HubApi {
    fn parse(&self, body: &str) -> Result<Vec<Server>, String> {
        let hub_servers: Vec<HubServer> =
            serde_json::from_str(body).map_err(|e| format!("Failed to parse response: {e}"))?;

        Ok(hub_servers.into_iter().map(Self::convert).collect())
    }
}

impl HubApi {
    fn convert(hub: HubServer) -> Server {
        let topic = hub.topic_status.as_ref();

        let data = topic.and_then(|ts| {
            ts.get("round_id")
                .and_then(serde_json::Value::as_i64)
                .map(|round_id| ServerData {
                    round_id,
                    mode: Self::get_str(ts, "mode").unwrap_or_default(),
                    map_name: Self::get_str(ts, "map_name")
                        .or_else(|| Self::get_str(ts, "map"))
                        .unwrap_or_default(),
                    round_duration: Self::get_f64(ts, "round_duration").unwrap_or(0.0),
                    gamestate: Self::get_i32(ts, "gamestate").unwrap_or(0),
                    players: Self::get_i32(ts, "players").unwrap_or(hub.players),
                    admins: Self::get_i32(ts, "admins"),
                    popcap: Self::get_i32(ts, "popcap"),
                    security_level: Self::get_str(ts, "security_level"),
                })
        });

        let address = topic
            .and_then(|ts| Self::get_str(ts, "public_address"))
            .unwrap_or_else(|| hub.address.clone());

        Server {
            name: hub.name,
            url: format!("byond://{address}"),
            status: if hub.online { "available" } else { "offline" }.to_string(),
            hub_status: hub.status.clone(),
            players: hub.players,
            data,
            is_18_plus: hub.status.contains("18+"),
            version: topic.and_then(|ts| Self::get_str(ts, "version")),
            recommended_byond_version: None,
            tags: Vec::new(),
        }
    }

    fn get_str(value: &Value, key: &str) -> Option<String> {
        value.get(key).and_then(|v| v.as_str()).map(String::from)
    }

    #[allow(clippy::cast_possible_truncation)] // server data fits in i32
    fn get_i32(value: &Value, key: &str) -> Option<i32> {
        value
            .get(key)
            .and_then(serde_json::Value::as_i64)
            .map(|v| v as i32)
    }

    fn get_f64(value: &Value, key: &str) -> Option<f64> {
        value.get(key).and_then(serde_json::Value::as_f64)
    }
}

struct CmApi;

#[derive(Debug, Deserialize)]
struct CmApiResponse {
    servers: Vec<CmServer>,
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
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

impl ServerApi for CmApi {
    fn parse(&self, body: &str) -> Result<Vec<Server>, String> {
        let response: CmApiResponse =
            serde_json::from_str(body).map_err(|e| format!("Failed to parse response: {e}"))?;

        Ok(response.servers.into_iter().map(Self::convert).collect())
    }
}

impl CmApi {
    fn convert(cm: CmServer) -> Server {
        let players = cm.data.as_ref().and_then(|d| d.players).unwrap_or(0);

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
            status: cm.status,
            hub_status: String::new(),
            players,
            data,
            is_18_plus: false,
            version: None,
            recommended_byond_version: cm.recommended_byond_version,
            tags: cm.tags.unwrap_or_default(),
        }
    }
}

fn get_api_adapter() -> Box<dyn ServerApi> {
    use crate::config::ServerApiType;
    match get_config().server_api {
        ServerApiType::HubApi => Box::new(HubApi),
        ServerApiType::CmApi => Box::new(CmApi),
    }
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
    let adapter = get_api_adapter();

    let response = reqwest::get(config.urls.server_api)
        .await
        .map_err(|e| format!("Failed to fetch servers: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {e}"))?;

    adapter.parse(&body)
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
                        notification_body = format!("Round #{current}");
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
