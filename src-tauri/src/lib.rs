mod auth;
mod autoconnect;
mod byond;
mod byond_login;
pub mod config;
mod control_server;
mod discord;
mod error;
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

#[allow(clippy::unreadable_literal)]
pub const DEFAULT_STEAM_ID: u32 = 4313790;
pub const DEFAULT_STEAM_NAME: &str = "production";

mod webview2;

use tauri::Manager;

use auth::{
    background_refresh_task, get_access_token, get_auth_state, get_hub_oauth_providers, hub_login,
    hub_oauth_login, logout, refresh_auth, start_login,
};
use byond::{
    check_byond_version, connect_to_server, connect_to_url, delete_byond_version,
    get_byond_username, install_byond_version, is_byond_pager_running, is_dev_mode,
    list_installed_byond_versions,
};
use byond_login::{
    byond_login_complete, byond_session_check_complete, cancel_byond_login,
    check_byond_web_session, clear_byond_session, get_byond_session_status, logout_byond_web,
    start_byond_login, ByondSessionState,
};
use relays::{get_relays, get_selected_relay, set_selected_relay};
use servers::get_servers;
use settings::{
    get_settings, set_age_verified, set_auth_mode, set_locale, set_rendering_pipeline, set_theme,
    toggle_server_notifications,
};

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
#[specta::specta]
fn get_platform() -> String {
    #[cfg(target_os = "windows")]
    return "windows".to_string();

    #[cfg(target_os = "macos")]
    return "macos".to_string();

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    return "unknown".to_string();
}

#[cfg(not(target_os = "linux"))]
#[derive(serde::Serialize, specta::Type)]
#[allow(clippy::struct_excessive_bools)]
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
#[specta::specta]
async fn check_wine_status() -> error::CommandResult<WineStatus> {
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
#[specta::specta]
async fn initialize_wine_prefix(
    _pipeline: settings::RenderingPipeline,
) -> error::CommandResult<()> {
    Err(error::CommandError::UnsupportedPlatform {
        feature: "wine".into(),
        platform: std::env::consts::OS.into(),
    })
}

#[cfg(not(target_os = "linux"))]
#[tauri::command]
#[specta::specta]
async fn reset_wine_prefix() -> error::CommandResult<()> {
    Err(error::CommandError::UnsupportedPlatform {
        feature: "wine".into(),
        platform: std::env::consts::OS.into(),
    })
}

#[cfg(feature = "steam")]
use auth::hub_steam_login;
#[cfg(feature = "steam")]
use steam::{
    cancel_steam_auth_ticket, get_steam_auth_ticket, get_steam_launch_options, get_steam_user_info,
    steam_authenticate,
};

#[tauri::command]
#[specta::specta]
fn greet(name: &str) -> String {
    format!("Hello, {name}! You've been greeted from Rust!")
}

#[tauri::command]
#[specta::specta]
fn get_control_server_port(control_server: tauri::State<'_, control_server::ControlServer>) -> u16 {
    control_server.port
}

#[tauri::command]
#[specta::specta]
fn kill_game(
    presence_manager: tauri::State<'_, std::sync::Arc<presence::PresenceManager>>,
) -> bool {
    presence_manager.kill_game_process()
}

#[tauri::command]
#[specta::specta]
fn open_url(url: String) -> error::CommandResult<()> {
    open_url::open(&url)
}

#[cfg(not(feature = "steam"))]
pub fn build_specta() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new().commands(tauri_specta::collect_commands![
        greet,
        check_byond_version,
        install_byond_version,
        connect_to_server,
        connect_to_url,
        is_dev_mode,
        list_installed_byond_versions,
        delete_byond_version,
        is_byond_pager_running,
        get_byond_username,
        start_login,
        hub_login,
        hub_oauth_login,
        get_hub_oauth_providers,
        logout,
        get_auth_state,
        refresh_auth,
        get_access_token,
        get_settings,
        set_age_verified,
        set_auth_mode,
        set_theme,
        set_locale,
        toggle_server_notifications,
        set_rendering_pipeline,
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
        start_byond_login,
        cancel_byond_login,
        byond_login_complete,
        get_byond_session_status,
        clear_byond_session,
        logout_byond_web,
        check_byond_web_session,
        byond_session_check_complete,
    ])
}

#[cfg(feature = "steam")]
pub fn build_specta() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new().commands(tauri_specta::collect_commands![
        greet,
        check_byond_version,
        install_byond_version,
        connect_to_server,
        connect_to_url,
        is_dev_mode,
        list_installed_byond_versions,
        delete_byond_version,
        is_byond_pager_running,
        get_byond_username,
        start_login,
        hub_login,
        hub_oauth_login,
        hub_steam_login,
        get_hub_oauth_providers,
        logout,
        get_auth_state,
        refresh_auth,
        get_access_token,
        get_settings,
        set_age_verified,
        set_auth_mode,
        set_theme,
        set_locale,
        toggle_server_notifications,
        set_rendering_pipeline,
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
        start_byond_login,
        cancel_byond_login,
        byond_login_complete,
        get_byond_session_status,
        clear_byond_session,
        logout_byond_web,
        check_byond_web_session,
        byond_session_check_complete,
    ])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _guard = logging::init_logging();

    #[cfg(target_os = "windows")]
    {
        if let Err(e) = job_object::init_job_object() {
            tracing::error!("Failed to initialize job object: {}", e);
        }

        if !webview2::check_webview2_installed() {
            webview2::show_webview2_error();
            let _ = open::that("https://go.microsoft.com/fwlink/p/?LinkId=2124703");
            std::process::exit(1);
        }
    }

    let specta_builder = build_specta();

    #[cfg(debug_assertions)]
    #[allow(clippy::expect_used)]
    specta_builder
        .export(
            specta_typescript::Typescript::default(),
            "../src/bindings.ts",
        )
        .expect("Failed to export typescript bindings");

    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(specta_builder.invoke_handler());

    // Only include updater for non-CM builds (CM uses Steam for updates)
    #[cfg(not(feature = "cm_ss13"))]
    {
        builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    }

    let mut manager = presence::PresenceManager::new();
    #[allow(unused_mut)]
    let mut steam_poll_callback: Option<Box<dyn Fn() + Send + Sync>> = None;

    #[cfg(feature = "steam")]
    {
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

                builder = builder.manage(steam_state);
            }
            Err(e) => {
                tracing::error!("Failed to initialize Steam: {:?}", e);
            }
        }
    };

    {
        use std::sync::Arc;

        let discord_state = Arc::new(discord::DiscordState::init());
        let discord_presence = discord::DiscordPresence::new(Arc::clone(&discord_state));
        manager.add_provider(Box::new(discord_presence));
        tracing::info!("Discord presence provider added (connecting in background)");
    }

    let presence_manager = std::sync::Arc::new(manager);
    let server_state = std::sync::Arc::new(servers::ServerState::new());
    let relay_state = std::sync::Arc::new(relays::RelayState::new());

    let byond_session_state = ByondSessionState::new();

    builder = builder
        .manage(std::sync::Arc::clone(&presence_manager))
        .manage(std::sync::Arc::clone(&server_state))
        .manage(std::sync::Arc::clone(&relay_state))
        .manage(byond_session_state);

    #[allow(clippy::expect_used)] // Main entry point - no recovery possible
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

            let server_state =
                std::sync::Arc::clone(app.state::<std::sync::Arc<servers::ServerState>>().inner());

            let server_state_init = std::sync::Arc::clone(&server_state);
            tauri::async_runtime::block_on(async {
                servers::init_servers(&server_state_init).await;
            });

            let handle_for_server_task = handle.clone();
            tauri::async_runtime::spawn(async move {
                servers::server_fetch_background_task(handle_for_server_task, server_state).await;
            });

            let relay_state =
                std::sync::Arc::clone(app.state::<std::sync::Arc<relays::RelayState>>().inner());

            let relay_state_init = relay_state;
            let handle_for_relay_init = handle.clone();
            tauri::async_runtime::spawn(async move {
                relays::init_relays(&relay_state_init, &handle_for_relay_init).await;
            });

            byond::cleanup_old_versions(&handle);

            autoconnect::check_and_start_autoconnect(handle.clone());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::build_specta;

    #[test]
    fn export_bindings() {
        assert!(
            cfg!(feature = "steam"),
            "export_bindings must be run with --features steam to generate complete bindings"
        );

        build_specta()
            .export(
                specta_typescript::Typescript::default()
                    .bigint(specta_typescript::BigIntExportBehavior::Number)
                    .header("// @ts-nocheck\n"),
                "../src/bindings.ts",
            )
            .expect("Failed to export typescript bindings");
    }
}
