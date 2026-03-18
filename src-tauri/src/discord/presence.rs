//! Discord Rich Presence integration using discord-sdk

use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "steam")]
use discord_sdk::activity::Button;
use discord_sdk::{
    activity::{ActivityBuilder, Assets},
    registration::{Application, LaunchCommand},
    wheel::{UserState, Wheel},
    Discord, Subscriptions,
};
use tokio::sync::{mpsc, watch};

#[cfg(feature = "steam")]
use crate::steam::get_steam_app_id;
use crate::{
    presence::{PresenceProvider, PresenceState},
    DEFAULT_STEAM_ID,
};

#[cfg(feature = "steam")]
fn steam_launch_url() -> String {
    use crate::steam::get_steam_app_id;

    format!("steam://run/{}", get_steam_app_id())
}

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

pub struct DiscordState {
    update_tx: mpsc::UnboundedSender<PresenceState>,
    connected_rx: watch::Receiver<bool>,
}

impl DiscordState {
    pub async fn init() -> Result<Self, discord_sdk::Error> {
        #[allow(unused_assignments, unused_mut)]
        let mut app_id: Option<u32> = None;

        #[cfg(feature = "steam")]
        {
            app_id = Some(get_steam_app_id());
        }

        let config = crate::config::get_config();
        if let Err(e) = discord_sdk::registration::register_app(Application {
            id: config.discord_app_id,
            name: Some(config.product_name.to_string()),
            command: LaunchCommand::Steam(app_id.unwrap_or(DEFAULT_STEAM_ID)),
        }) {
            tracing::warn!("Failed to register Discord app: {:?}", e);
        }

        let (update_tx, update_rx) = mpsc::unbounded_channel();
        let (connected_tx, connected_rx) = watch::channel(false);

        tokio::spawn(Self::run_discord_task(update_rx, connected_tx));

        Ok(Self {
            update_tx,
            connected_rx,
        })
    }

    #[allow(dead_code)]
    pub fn is_connected(&self) -> bool {
        *self.connected_rx.borrow()
    }

    /// Background task that maintains the Discord connection and processes presence updates
    async fn run_discord_task(
        mut update_rx: mpsc::UnboundedReceiver<PresenceState>,
        connected_tx: watch::Sender<bool>,
    ) {
        let (wheel, handler) = Wheel::new(Box::new(|err| {
            tracing::warn!("Discord error: {:?}", err);
        }));

        let mut user_spoke = wheel.user();

        let config = crate::config::get_config();
        let discord = match Discord::new(
            config.discord_app_id,
            Subscriptions::ACTIVITY,
            Box::new(handler),
        ) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("Discord not available: {:?}", e);
                return;
            }
        };

        tracing::info!("Discord connecting...");

        let user = match tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
            if user_spoke.0.changed().await.is_err() {
                Err("Discord connection closed".to_string())
            } else {
                match &*user_spoke.0.borrow() {
                    UserState::Connected(user) => Ok(user.clone()),
                    UserState::Disconnected(err) => Err(format!("Discord disconnected: {:?}", err)),
                }
            }
        })
        .await
        {
            Ok(Ok(user)) => user,
            Ok(Err(e)) => {
                tracing::warn!("{}", e);
                return;
            }
            Err(_) => {
                tracing::warn!("Discord handshake timed out");
                return;
            }
        };

        tracing::info!(
            "Discord Rich Presence connected as {}#{}",
            user.username,
            user.discriminator.unwrap_or(0)
        );

        if connected_tx.send(true).is_err() {
            tracing::warn!("Failed to signal Discord connection status");
        }

        while let Some(state) = update_rx.recv().await {
            let result = match &state {
                PresenceState::InLauncher => {
                    let config = crate::config::get_config();
                    #[allow(unused_mut)]
                    let mut activity = ActivityBuilder::new()
                        .state("In Launcher")
                        .assets(Assets::default().large("logo", Some(config.product_name)));

                    #[cfg(feature = "steam")]
                    {
                        activity = activity.button(Button {
                            label: "Play".to_string(),
                            url: steam_launch_url(),
                        });
                    }

                    discord.update_activity(activity).await
                }
                PresenceState::Playing {
                    server_name,
                    player_count,
                    map_name,
                } => {
                    let details = match map_name {
                        Some(map) => format!("{} players on {}", player_count, map),
                        None => format!("{} players online", player_count),
                    };

                    let game_name = crate::config::get_config().strings.discord_game_name;
                    #[allow(unused_mut)]
                    let mut activity = ActivityBuilder::new()
                        .state(format!("Playing on {}", server_name))
                        .details(details)
                        .assets(Assets::default().large("logo", Some(game_name)));

                    // Only add join button when Steam feature is enabled (provides valid URL)
                    #[cfg(feature = "steam")]
                    {
                        let encoded_server =
                            url::form_urlencoded::byte_serialize(server_name.as_bytes())
                                .collect::<String>();
                        let join_url = format!("{}//{}", steam_launch_url(), encoded_server);
                        activity = activity.button(Button {
                            label: "Join Game".to_string(),
                            url: join_url,
                        });
                    }

                    #[cfg(feature = "discord_invites")]
                    {
                        use discord_sdk::activity::Secrets;
                        use serde_json::json;

                        activity = activity
                            .party(
                                server_name,
                                std::num::NonZeroU32::new(*player_count),
                                std::num::NonZeroU32::new(300),
                                discord_sdk::activity::PartyPrivacy::Public,
                            )
                            .secrets(Secrets {
                                r#match: None,
                                join: Some(
                                    json!({"server_name": &server_name, "type": "join"})
                                        .to_string(),
                                ),
                                spectate: Some(
                                    json!({"server_name": &server_name, "type": "spectate"})
                                        .to_string(),
                                ),
                            })
                    }

                    discord.update_activity(activity).await
                }
                PresenceState::Disconnected => discord.clear_activity().await,
            };

            if let Err(e) = result {
                tracing::warn!("Failed to update Discord activity: {:?}", e);
            } else {
                tracing::debug!("Discord activity updated successfully");
            }
        }

        discord.disconnect().await;
        tracing::info!("Discord Rich Presence disconnected");
    }

    pub fn send_update(&self, state: PresenceState) {
        if self.update_tx.send(state.clone()).is_err() {
            tracing::warn!(
                "Failed to send Discord presence update (channel closed): {:?}",
                state
            );
        } else {
            tracing::debug!("Queued Discord presence update: {:?}", state);
        }
    }
}

pub struct DiscordPresence {
    state: Arc<DiscordState>,
}

impl DiscordPresence {
    pub fn new(state: Arc<DiscordState>) -> Self {
        Self { state }
    }
}

impl PresenceProvider for DiscordPresence {
    fn name(&self) -> &'static str {
        "Discord"
    }

    fn update_presence(&self, state: &PresenceState) {
        self.state.send_update(state.clone());
    }

    fn clear_presence(&self) {
        self.state.send_update(PresenceState::Disconnected);
    }
}
