//! Centralized configuration module for launcher variants.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct LauncherConfig {
    pub variant: &'static str,
    pub product_name: &'static str,
    pub default_theme: &'static str,
    pub app_identifier: &'static str,
    pub discord_app_id: i64,
    pub default_byond_version: Option<&'static str>,
    pub features: LauncherFeatures,
    pub urls: LauncherUrls,
    pub strings: LauncherStrings,
    pub singleplayer: SingleplayerConfig,
    pub oidc: Option<OidcConfig>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LauncherFeatures {
    pub social_links: bool,
    pub relay_selector: bool,
    pub hub_server_list: bool,
    pub cm_auth: bool,
    pub singleplayer: bool,
    pub server_search: bool,
    pub server_filters: bool,
    pub show_offline_servers: bool,
    pub server_stats: bool,
    pub auto_launch_byond: bool,
    pub connection_timeout_fallback: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SingleplayerConfig {
    pub github_repo: Option<&'static str>,
    pub build_asset_name: Option<&'static str>,
    pub dmb_name: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct OidcConfig {
    pub client_id: &'static str,
    pub auth_url: &'static str,
    pub token_url: &'static str,
    pub userinfo_url: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct LauncherUrls {
    pub server_api: &'static str,
    pub auth_base: Option<&'static str>,
    pub steam_auth: Option<&'static str>,
    pub byond_hash_api: Option<&'static str>,
    pub help_url: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct LauncherStrings {
    pub auth_provider_name: &'static str,
    pub login_prompt: &'static str,
    pub discord_game_name: &'static str,
}

#[cfg(feature = "cm_ss13")]
pub fn get_config() -> LauncherConfig {
    LauncherConfig {
        variant: "cm_ss13",
        product_name: "CM-SS13 Launcher",
        default_theme: "crt",
        app_identifier: "com.cm-ss13.launcher",
        discord_app_id: 1383904378154651768,
        default_byond_version: None,
        features: LauncherFeatures {
            social_links: true,
            relay_selector: true,
            hub_server_list: false,
            cm_auth: true,
            singleplayer: true,
            server_search: false,
            server_filters: false,
            show_offline_servers: true,
            server_stats: false,
            auto_launch_byond: false,
            connection_timeout_fallback: false,
        },
        urls: LauncherUrls {
            server_api: "https://db.cm-ss13.com/api/Round",
            auth_base: Some("https://login.cm-ss13.com"),
            steam_auth: Some("https://db.cm-ss13.com/api/Steam/Authenticate"),
            byond_hash_api: Some("https://db.cm-ss13.com/api/ByondHash"),
            help_url: "https://github.com/cmss13-devs/cm-launcher/issues",
        },
        strings: LauncherStrings {
            auth_provider_name: "CM-SS13",
            login_prompt: "Please log in with your CM-SS13 account to continue.",
            discord_game_name: "Colonial Marines",
        },
        singleplayer: SingleplayerConfig {
            github_repo: Some("cmss13-devs/cmss13"),
            build_asset_name: Some("colonialmarines-build.tar.zst"),
            dmb_name: Some("colonialmarines.dmb"),
        },
        oidc: Some(OidcConfig {
            client_id: "6hm46av41Q5fb47CU8en8B9zZzDsIsKw3BRhSlyo",
            auth_url: "https://login.cm-ss13.com/application/o/authorize/",
            token_url: "https://login.cm-ss13.com/application/o/token/",
            userinfo_url: "https://login.cm-ss13.com/application/o/userinfo/",
        }),
    }
}

#[cfg(not(feature = "cm_ss13"))]
pub fn get_config() -> LauncherConfig {
    LauncherConfig {
        variant: "ss13",
        product_name: "SS13 Launcher",
        default_theme: "tgui",
        app_identifier: "com.ss13.launcher",
        discord_app_id: 1483901387086761994,
        default_byond_version: Some("516.1667"),
        features: LauncherFeatures {
            social_links: false,
            relay_selector: false,
            hub_server_list: true,
            cm_auth: false,
            singleplayer: false,
            server_search: true,
            server_filters: true,
            show_offline_servers: false,
            server_stats: true,
            auto_launch_byond: true,
            connection_timeout_fallback: true,
        },
        urls: LauncherUrls {
            server_api: "https://hub.cm-ss13.com/servers",
            auth_base: None,
            steam_auth: None,
            byond_hash_api: None,
            help_url: "https://github.com/user/ss13-launcher/issues",
        },
        strings: LauncherStrings {
            auth_provider_name: "SS13",
            login_prompt: "Please log in to continue.",
            discord_game_name: "Space Station 13",
        },
        singleplayer: SingleplayerConfig {
            github_repo: None,
            build_asset_name: None,
            dmb_name: None,
        },
        oidc: None,
    }
}

#[tauri::command]
pub fn get_launcher_config() -> LauncherConfig {
    get_config()
}
