use crate::config::get_config;
use crate::error::{CommandError, CommandResult};
use crate::settings::load_settings;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::RwLock;

const SERVER_FETCH_INTERVAL_SECS: u64 = 20;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ServerData {
    #[specta(type = f64)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, specta::Type)]
pub struct EngineRequirements {
    #[serde(default)]
    pub min_version: Option<String>,
    #[serde(default)]
    pub max_version: Option<String>,
    #[serde(default)]
    pub blacklisted_versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct Server {
    pub id: Option<String>,
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
    #[serde(default)]
    pub engine: Option<EngineRequirements>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub auth_methods: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub links: Vec<ServerLink>,
    #[serde(default)]
    pub verified_domain: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ServerLink {
    pub link: String,
    #[serde(rename = "type")]
    pub link_type: String,
}

trait ServerApi: Send + Sync {
    fn parse(&self, body: &str) -> CommandResult<Vec<Server>>;
}
struct HubApi;

#[derive(Debug, Deserialize)]
struct HubServer {
    id: String,
    address: String,
    #[serde(default)]
    status: Option<HubServerStatus>,
    #[serde(default)]
    auth_methods: Vec<String>,
    #[serde(default)]
    verified_domain: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HubEngineInfo {
    #[serde(default)]
    min_version: Option<String>,
    #[serde(default)]
    max_version: Option<String>,
    #[serde(default)]
    blacklisted_versions: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct HubServerStatus {
    pop: i32,
    display_name: String,
    #[serde(default)]
    pop_cap: Option<i32>,
    #[serde(default)]
    region: Option<String>,
    #[serde(default)]
    server_tags: Option<Vec<String>>,
    #[serde(default)]
    engine: Option<HubEngineInfo>,
    #[serde(default)]
    round: Option<HubRoundInfo>,
    #[serde(default)]
    connection_address: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    links: Option<Vec<ServerLink>>,
}

#[derive(Debug, Deserialize)]
struct HubRoundInfo {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    gamemode: Option<String>,
    #[serde(default)]
    map_name: Option<String>,
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    security_level: Option<String>,
    #[serde(default)]
    state: Option<String>,
}

impl ServerApi for HubApi {
    fn parse(&self, body: &str) -> CommandResult<Vec<Server>> {
        let hub_servers: Vec<HubServer> = serde_json::from_str(body).map_err(|e| {
            CommandError::InvalidResponse(format!("Failed to parse hub server list: {e}"))
        })?;

        Ok(hub_servers.into_iter().map(Self::convert).collect())
    }
}

impl HubApi {
    fn convert(hub: HubServer) -> Server {
        let (name, players, data, engine, tags, is_18_plus, description, links, connection_address) =
            if let Some(ref s) = hub.status {
                let round = s.round.as_ref();

                let data = round.and_then(|r| {
                    r.id.as_deref()
                        .and_then(|id| id.parse::<i64>().ok())
                        .map(|round_id| ServerData {
                            round_id,
                            mode: r.gamemode.clone().unwrap_or_default(),
                            map_name: r.map_name.clone().unwrap_or_default(),
                            round_duration: r.duration.unwrap_or(0.0),
                            gamestate: 0,
                            players: s.pop,
                            admins: None,
                            popcap: s.pop_cap,
                            security_level: r.security_level.clone(),
                        })
                });

                let engine = s.engine.as_ref().map(|e| EngineRequirements {
                    min_version: e.min_version.clone(),
                    max_version: e.max_version.clone(),
                    blacklisted_versions: e.blacklisted_versions.clone().unwrap_or_default(),
                });

                let tags = s.server_tags.clone().unwrap_or_default();
                let is_18_plus = tags.iter().any(|t| t == "18+");

                (
                    s.display_name.clone(),
                    s.pop,
                    data,
                    engine,
                    tags,
                    is_18_plus,
                    s.description.clone(),
                    s.links.clone().unwrap_or_default(),
                    s.connection_address.clone(),
                )
            } else {
                (
                    hub.address.clone(),
                    0,
                    None,
                    None,
                    Vec::new(),
                    false,
                    None,
                    Vec::new(),
                    None,
                )
            };

        let address = connection_address.unwrap_or(hub.address);

        Server {
            id: Some(hub.id),
            name,
            url: format!("byond://{address}"),
            status: "available".to_string(),
            hub_status: String::new(),
            players,
            data,
            is_18_plus,
            version: None,
            engine,
            tags,
            auth_methods: hub.auth_methods,
            description,
            links,
            verified_domain: hub.verified_domain,
        }
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
    fn parse(&self, body: &str) -> CommandResult<Vec<Server>> {
        let response: CmApiResponse = serde_json::from_str(body).map_err(|e| {
            CommandError::InvalidResponse(format!("Failed to parse cm server list: {e}"))
        })?;

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

        let engine = cm.recommended_byond_version.map(|v| EngineRequirements {
            min_version: Some(v.clone()),
            max_version: Some(v),
            blacklisted_versions: Vec::new(),
        });

        Server {
            id: None,
            name: cm.name,
            url: format!("byond://{}", cm.url),
            status: cm.status,
            hub_status: String::new(),
            players,
            data,
            is_18_plus: false,
            version: None,
            engine,
            tags: cm.tags.unwrap_or_default(),
            auth_methods: Vec::new(),
            description: None,
            links: Vec::new(),
            verified_domain: None,
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

async fn fetch_servers_internal() -> CommandResult<Vec<Server>> {
    let config = get_config();
    let adapter = get_api_adapter();

    let response = reqwest::get(config.urls.server_api).await?;

    if !response.status().is_success() {
        return Err(CommandError::InvalidResponse(format!(
            "Server list HTTP error: {}",
            response.status()
        )));
    }

    let body = response.text().await?;

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
#[specta::specta]
pub async fn get_servers(state: tauri::State<'_, Arc<ServerState>>) -> CommandResult<Vec<Server>> {
    Ok(state.servers.read().await.clone())
}

pub async fn server_fetch_background_task(handle: AppHandle, state: Arc<ServerState>) {
    loop {
        tokio::time::sleep(Duration::from_secs(SERVER_FETCH_INTERVAL_SECS)).await;

        match fetch_servers_internal().await {
            Ok(servers) => {
                check_and_send_notifications(&handle, &state, &servers).await;

                *state.servers.write().await = servers.clone();
                let _ = handle.emit("servers-updated", ServerUpdateEvent { servers });
            }
            Err(error) => {
                tracing::error!("Server fetch error: {}", error);
                let _ = handle.emit(
                    "servers-error",
                    ServerErrorEvent {
                        error: error.to_string(),
                    },
                );
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
