use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const PING_PORT: u16 = 4000;
const PING_COUNT: u32 = 10;
const PING_TIMEOUT: Duration = Duration::from_secs(5);
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relay {
    pub id: String,
    pub name: String,
    pub host: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayWithPing {
    #[serde(flatten)]
    pub relay: Relay,
    pub ping: Option<u32>,
    pub checking: bool,
}

pub struct RelayState {
    relays: RwLock<Vec<RelayWithPing>>,
    selected: RwLock<String>,
}

impl RelayState {
    pub fn new() -> Self {
        let relays = get_default_relays()
            .into_iter()
            .map(|r| RelayWithPing {
                relay: r,
                ping: None,
                checking: true,
            })
            .collect();

        Self {
            relays: RwLock::new(relays),
            selected: RwLock::new(String::new()),
        }
    }

    pub async fn get_relays(&self) -> Vec<RelayWithPing> {
        self.relays.read().await.clone()
    }

    pub async fn get_selected(&self) -> String {
        self.selected.read().await.clone()
    }

    pub async fn set_selected(&self, id: String) {
        *self.selected.write().await = id;
    }

    pub async fn get_selected_host(&self) -> Option<String> {
        let relays = self.relays.read().await;
        let selected = self.selected.read().await;
        relays
            .iter()
            .find(|r| r.relay.id == *selected)
            .map(|r| r.relay.host.clone())
    }

    #[allow(dead_code)]
    pub async fn all_relays_pinged(&self) -> bool {
        let relays = self.relays.read().await;
        relays.iter().all(|r| !r.checking)
    }

    async fn update_relay_ping(&self, id: &str, ping: Option<u32>) {
        let mut relays = self.relays.write().await;
        if let Some(relay) = relays.iter_mut().find(|r| r.relay.id == id) {
            relay.ping = ping;
            relay.checking = false;
        }
    }
}

#[cfg(feature = "cm_ss13")]
fn get_default_relays() -> Vec<Relay> {
    vec![
        Relay {
            id: "direct".to_string(),
            name: "Direct".to_string(),
            host: "direct.cm-ss13.com".to_string(),
        },
        Relay {
            id: "nyc".to_string(),
            name: "NYC".to_string(),
            host: "nyc.cm-ss13.com".to_string(),
        },
        Relay {
            id: "uk".to_string(),
            name: "UK".to_string(),
            host: "uk.cm-ss13.com".to_string(),
        },
        Relay {
            id: "eu-e".to_string(),
            name: "EU East".to_string(),
            host: "eu-e.cm-ss13.com".to_string(),
        },
        Relay {
            id: "eu-w".to_string(),
            name: "EU West".to_string(),
            host: "eu-w.cm-ss13.com".to_string(),
        },
        Relay {
            id: "aus".to_string(),
            name: "Australia".to_string(),
            host: "aus.cm-ss13.com".to_string(),
        },
        Relay {
            id: "us-e".to_string(),
            name: "US East".to_string(),
            host: "us-e.cm-ss13.com".to_string(),
        },
        Relay {
            id: "us-w".to_string(),
            name: "US West".to_string(),
            host: "us-w.cm-ss13.com".to_string(),
        },
        Relay {
            id: "asia-se".to_string(),
            name: "SE Asia".to_string(),
            host: "asia-se.cm-ss13.com".to_string(),
        },
    ]
}

#[cfg(not(feature = "cm_ss13"))]
fn get_default_relays() -> Vec<Relay> {
    vec![]
}

#[allow(clippy::cast_possible_truncation, clippy::arithmetic_side_effects)] // ping times in ms are small
async fn ping_relay(host: &str) -> Option<u32> {
    let url = format!("wss://{host}:{PING_PORT}");

    let connect_result = tokio::time::timeout(PING_TIMEOUT, connect_async(&url)).await;

    let (mut ws_stream, _) = match connect_result {
        Ok(Ok(conn)) => conn,
        Ok(Err(e)) => {
            tracing::debug!("WebSocket connection error for {}: {}", host, e);
            return None;
        }
        Err(_) => {
            tracing::debug!("WebSocket connection timeout for {}", host);
            return None;
        }
    };

    let mut ping_times = Vec::with_capacity(PING_COUNT as usize);

    for i in 1..=PING_COUNT {
        let start = Instant::now();
        let msg = i.to_string();

        if ws_stream.send(Message::Text(msg.clone())).await.is_err() {
            break;
        }

        let response = tokio::time::timeout(Duration::from_secs(2), ws_stream.next()).await;

        match response {
            Ok(Some(Ok(Message::Text(text)))) if text == msg => {
                ping_times.push(start.elapsed().as_millis() as u32);
            }
            _ => break,
        }
    }

    let _ = ws_stream.close(None).await;

    if ping_times.is_empty() {
        None
    } else {
        let avg = ping_times.iter().sum::<u32>() / ping_times.len() as u32;
        Some(avg)
    }
}

pub async fn init_relays(state: &Arc<RelayState>, handle: &AppHandle) {
    let relays = state.get_relays().await;

    let state_clone = Arc::clone(state);
    let handle_clone = handle.clone();

    let ping_futures: Vec<_> = relays
        .iter()
        .map(|r| {
            let id = r.relay.id.clone();
            let host = r.relay.host.clone();
            let state = Arc::clone(&state_clone);
            let handle = handle_clone.clone();

            async move {
                let ping = ping_relay(&host).await;
                state.update_relay_ping(&id, ping).await;

                if let Some(ping) = ping {
                    let current_selected = state.get_selected().await;
                    let relays = state.get_relays().await;

                    let current_ping = relays
                        .iter()
                        .find(|r| r.relay.id == current_selected)
                        .and_then(|r| r.ping);

                    let should_select =
                        current_selected.is_empty() || current_ping.is_none_or(|p| ping < p);

                    if should_select {
                        state.set_selected(id.clone()).await;
                        tracing::info!("Auto-selected relay: {} ({}ms)", id, ping);
                        let _ = handle.emit("relay-selected", &id);
                    }

                    let _ = handle.emit("relays-updated", &relays);
                } else {
                    let relays = state.get_relays().await;
                    let _ = handle.emit("relays-updated", &relays);
                }

                (id, ping)
            }
        })
        .collect();

    futures_util::future::join_all(ping_futures).await;
}

#[tauri::command]
pub async fn get_relays(
    state: tauri::State<'_, Arc<RelayState>>,
) -> Result<Vec<RelayWithPing>, ()> {
    Ok(state.get_relays().await)
}

#[tauri::command]
pub async fn get_selected_relay(state: tauri::State<'_, Arc<RelayState>>) -> Result<String, ()> {
    Ok(state.get_selected().await)
}

#[tauri::command]
pub async fn set_selected_relay(
    id: String,
    state: tauri::State<'_, Arc<RelayState>>,
    handle: AppHandle,
) -> Result<(), ()> {
    state.set_selected(id.clone()).await;
    let _ = handle.emit("relay-selected", &id);
    Ok(())
}
