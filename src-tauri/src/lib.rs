mod auth;
mod autoconnect;
mod byond;
pub mod config;
mod control_server;
mod discord;
#[cfg(target_os = "windows")]
mod job_object;
mod logging;
mod open_url;
mod presence;
mod relays;
mod servers;
mod settings;
mod singleplayer;
#[cfg(feature = "steam")]
mod steam;
#[cfg(target_os = "linux")]
mod wine;

pub const DEFAULT_STEAM_ID: u32 = 4313790;
pub const DEFAULT_STEAM_NAME: &str = "production";

mod webview2;

use tauri::Manager;

use auth::{
    background_refresh_task, get_access_token, get_auth_state, logout, refresh_auth, start_login,
};
use byond::{
    check_byond_version, connect_to_server, connect_to_url, delete_byond_version,
    install_byond_version, is_byond_pager_running, is_dev_mode, list_installed_byond_versions,
};
use relays::{get_relays, get_selected_relay, set_selected_relay};
use servers::get_servers;
use settings::{get_settings, set_auth_mode, set_fullscreen_overlay, set_theme, toggle_server_notifications};

use singleplayer::{
    delete_singleplayer, get_latest_singleplayer_release, get_singleplayer_status,
    install_singleplayer, launch_singleplayer,
};

use config::get_launcher_config;

#[cfg(target_os = "linux")]
use wine::{check_wine_status, initialize_wine_prefix, reset_wine_prefix, WineStatus};

#[cfg(target_os = "linux")]
pub use wine::get_platform;

#[cfg(not(target_os = "linux"))]
#[tauri::command]
fn get_platform() -> String {
    #[cfg(target_os = "windows")]
    return "windows".to_string();

    #[cfg(target_os = "macos")]
    return "macos".to_string();

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    return "unknown".to_string();
}

#[cfg(not(target_os = "linux"))]
#[derive(serde::Serialize)]
struct WineStatus {
    installed: bool,
    version: Option<String>,
    meets_minimum_version: bool,
    winetricks_installed: bool,
    prefix_initialized: bool,
    webview2_installed: bool,
    error: Option<String>,
}

#[cfg(not(target_os = "linux"))]
#[tauri::command]
async fn check_wine_status() -> Result<WineStatus, String> {
    Ok(WineStatus {
        installed: false,
        version: None,
        meets_minimum_version: false,
        winetricks_installed: false,
        prefix_initialized: false,
        webview2_installed: false,
        error: Some("Wine is only available on Linux".to_string()),
    })
}

#[cfg(not(target_os = "linux"))]
#[tauri::command]
async fn initialize_wine_prefix() -> Result<(), String> {
    Err("Wine is only available on Linux".to_string())
}

#[cfg(not(target_os = "linux"))]
#[tauri::command]
async fn reset_wine_prefix() -> Result<(), String> {
    Err("Wine is only available on Linux".to_string())
}

#[cfg(feature = "steam")]
use steam::{
    cancel_steam_auth_ticket, get_steam_auth_ticket, get_steam_launch_options, get_steam_user_info,
    steam_authenticate,
};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_control_server_port(control_server: tauri::State<'_, control_server::ControlServer>) -> u16 {
    control_server.port
}

#[tauri::command]
fn kill_game(
    presence_manager: tauri::State<'_, std::sync::Arc<presence::PresenceManager>>,
) -> bool {
    presence_manager.kill_game_process()
}

#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    open_url::open(&url)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _guard = logging::init_logging();

    #[cfg(target_os = "windows")]
    {
        // Initialize job object for child process lifecycle management.
        // This ensures spawned game processes are terminated when the launcher exits
        // (e.g., when user clicks "Stop" in Steam).
        if let Err(e) = job_object::init_job_object() {
            tracing::error!("Failed to initialize job object: {}", e);
        }

        if !webview2::check_webview2_installed() {
            webview2::show_webview2_error();
            let _ = open::that("https://go.microsoft.com/fwlink/p/?LinkId=2124703");
            std::process::exit(1);
        }
    }

    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init());

    #[cfg(not(feature = "steam"))]
    {
        builder = builder.invoke_handler(tauri::generate_handler![
            greet,
            check_byond_version,
            install_byond_version,
            connect_to_server,
            connect_to_url,
            is_dev_mode,
            list_installed_byond_versions,
            delete_byond_version,
            is_byond_pager_running,
            start_login,
            logout,
            get_auth_state,
            refresh_auth,
            get_access_token,
            get_settings,
            set_auth_mode,
            set_theme,
            set_fullscreen_overlay,
            toggle_server_notifications,
            get_control_server_port,
            kill_game,
            get_servers,
            get_relays,
            get_selected_relay,
            set_selected_relay,
            get_platform,
            check_wine_status,
            initialize_wine_prefix,
            reset_wine_prefix,
            open_url,
            get_singleplayer_status,
            get_latest_singleplayer_release,
            install_singleplayer,
            delete_singleplayer,
            launch_singleplayer,
            get_launcher_config,
        ]);
    }

    #[cfg(feature = "steam")]
    {
        builder = builder.invoke_handler(tauri::generate_handler![
            greet,
            check_byond_version,
            install_byond_version,
            connect_to_server,
            connect_to_url,
            is_dev_mode,
            list_installed_byond_versions,
            delete_byond_version,
            is_byond_pager_running,
            start_login,
            logout,
            get_auth_state,
            refresh_auth,
            get_access_token,
            get_settings,
            set_auth_mode,
            set_theme,
            set_fullscreen_overlay,
            toggle_server_notifications,
            get_control_server_port,
            kill_game,
            get_servers,
            get_relays,
            get_selected_relay,
            set_selected_relay,
            get_steam_user_info,
            get_steam_auth_ticket,
            cancel_steam_auth_ticket,
            steam_authenticate,
            get_steam_launch_options,
            get_platform,
            check_wine_status,
            initialize_wine_prefix,
            reset_wine_prefix,
            open_url,
            get_singleplayer_status,
            get_latest_singleplayer_release,
            install_singleplayer,
            delete_singleplayer,
            launch_singleplayer,
            get_launcher_config,
        ]);
    }

    let mut manager = presence::PresenceManager::new();
    #[allow(unused_mut)]
    let mut steam_poll_callback: Option<Box<dyn Fn() + Send + Sync>> = None;

    #[cfg(feature = "steam")]
    let steam_overlay_rx: Option<tokio::sync::broadcast::Receiver<bool>> = {
        use std::sync::Arc;

        use crate::steam::get_steam_app_id;

        if steamworks::restart_app_if_necessary(steamworks::AppId(get_steam_app_id())) {
            std::process::exit(1);
        }

        match steam::SteamState::init() {
            Ok(steam_state) => {
                let steam_state = Arc::new(steam_state);

                let steam_presence = steam::SteamPresence::new(steam_state.client().clone());
                manager.add_provider(Box::new(steam_presence));

                let steam_state_clone = Arc::clone(&steam_state);
                steam_poll_callback = Some(Box::new(move || steam_state_clone.run_callbacks()));

                let overlay_rx = steam_state.subscribe_overlay_events();

                builder = builder.manage(steam_state);
                Some(overlay_rx)
            }
            Err(e) => {
                tracing::error!("Failed to initialize Steam: {:?}", e);
                None
            }
        }
    };

    #[cfg(not(feature = "steam"))]
    let steam_overlay_rx: Option<tokio::sync::broadcast::Receiver<bool>> = None;

    {
        use std::sync::Arc;

        match tauri::async_runtime::block_on(discord::DiscordState::init()) {
            Ok(discord_state) => {
                let discord_state = Arc::new(discord_state);
                // Add provider immediately - updates are queued and sent once connected
                let discord_presence = discord::DiscordPresence::new(Arc::clone(&discord_state));
                manager.add_provider(Box::new(discord_presence));
                tracing::info!("Discord presence provider added (connecting in background)");
            }
            Err(e) => {
                tracing::error!("Failed to initialize Discord: {:?}", e);
            }
        }
    }

    let presence_manager = std::sync::Arc::new(manager);
    let server_state = std::sync::Arc::new(servers::ServerState::new());
    let relay_state = std::sync::Arc::new(relays::RelayState::new());

    builder = builder
        .manage(std::sync::Arc::clone(&presence_manager))
        .manage(std::sync::Arc::clone(&server_state))
        .manage(std::sync::Arc::clone(&relay_state));

    builder
        .setup(move |app| {
            let handle = app.handle().clone();

            presence::start_presence_background_task(
                std::sync::Arc::clone(&presence_manager),
                steam_poll_callback,
                handle.clone(),
            );

            match control_server::ControlServer::start(
                handle.clone(),
                std::sync::Arc::clone(&presence_manager),
            ) {
                Ok(server) => {
                    tracing::info!("Control server running on port {}", server.port);
                    app.manage(server);
                }
                Err(e) => {
                    tracing::error!("Failed to start control server: {}", e);
                }
            }

            let handle_for_auth = handle.clone();
            tauri::async_runtime::spawn(async move {
                background_refresh_task(handle_for_auth).await;
            });

            let server_state = app
                .state::<std::sync::Arc<servers::ServerState>>()
                .inner()
                .clone();

            let server_state_init = server_state.clone();
            tauri::async_runtime::block_on(async {
                servers::init_servers(&server_state_init).await;
            });

            let handle_for_server_task = handle.clone();
            tauri::async_runtime::spawn(async move {
                servers::server_fetch_background_task(handle_for_server_task, server_state).await;
            });

            let relay_state = app
                .state::<std::sync::Arc<relays::RelayState>>()
                .inner()
                .clone();

            let relay_state_init = relay_state.clone();
            let handle_for_relay_init = handle.clone();
            tauri::async_runtime::spawn(async move {
                relays::init_relays(&relay_state_init, &handle_for_relay_init).await;
            });

            autoconnect::check_and_start_autoconnect(handle.clone());

            if let Some(mut overlay_rx) = steam_overlay_rx {
                let handle_for_overlay = handle;
                tauri::async_runtime::spawn(async move {
                    loop {
                        match overlay_rx.recv().await {
                            Ok(active) => {
                                // Check if fullscreen overlay events are enabled
                                let should_broadcast = settings::load_settings(&handle_for_overlay)
                                    .map(|s| s.fullscreen_overlay)
                                    .unwrap_or(true);

                                if should_broadcast {
                                    if let Some(server) =
                                        handle_for_overlay.try_state::<control_server::ControlServer>()
                                    {
                                        server.broadcast_json(
                                            "steam_overlay",
                                            &serde_json::json!({ "active": active }),
                                        );
                                        tracing::debug!(
                                            "Broadcast steam_overlay event: active={}",
                                            active
                                        );
                                    }
                                } else {
                                    tracing::debug!(
                                        "Skipped steam_overlay event (disabled in settings): active={}",
                                        active
                                    );
                                }
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                tracing::warn!(
                                    "Overlay event receiver lagged, skipped {} events",
                                    n
                                );
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                tracing::debug!("Overlay event channel closed");
                                break;
                            }
                        }
                    }
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
