use crate::error::{CommandError, CommandResult};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum AuthMode {
    #[default]
    #[serde(alias = "cm_ss13")]
    Oidc,
    Hub,
    Byond,
    Steam,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    #[serde(alias = "ntos")]
    Tgui,
    #[serde(alias = "default")]
    Crt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum RenderingPipeline {
    #[default]
    Dxvk,
    #[serde(rename = "wined3d")]
    Wined3d,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct AppSettings {
    pub auth_mode: AuthMode,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub notification_servers: HashSet<String>,
    #[serde(default)]
    pub age_verified: bool,
    #[serde(default)]
    pub locale: Option<String>,
    #[serde(default)]
    pub rendering_pipeline: RenderingPipeline,
}

impl Default for AppSettings {
    fn default() -> Self {
        let config = crate::config::get_config();
        let default_theme = match config.default_theme {
            "crt" => Theme::Crt,
            _ => Theme::Tgui,
        };

        let auth_mode = if config.urls.hub_api.is_some() {
            AuthMode::Hub
        } else {
            #[cfg(feature = "steam")]
            {
                AuthMode::Steam
            }
            #[cfg(not(feature = "steam"))]
            {
                if config.oidc.is_some() {
                    AuthMode::Oidc
                } else {
                    AuthMode::Byond
                }
            }
        };

        Self {
            auth_mode,
            theme: default_theme,
            notification_servers: HashSet::new(),
            age_verified: false,
            locale: None,
            rendering_pipeline: RenderingPipeline::default(),
        }
    }
}

fn get_settings_path(app: &AppHandle) -> CommandResult<PathBuf> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| CommandError::Io(format!("app data directory unavailable: {e}")))?;

    fs::create_dir_all(&app_data)?;

    Ok(app_data.join(SETTINGS_FILE))
}

pub fn load_settings(app: &AppHandle) -> CommandResult<AppSettings> {
    tracing::debug!("Loading settings");
    let path = get_settings_path(app)?;

    if !path.exists() {
        return Ok(AppSettings::default());
    }

    let contents = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Failed to read settings file, using defaults: {}", e);
            return Ok(AppSettings::default());
        }
    };

    if contents.trim().is_empty() {
        tracing::warn!("Settings file is empty, using defaults");
        return Ok(AppSettings::default());
    }

    match serde_json::from_str::<AppSettings>(&contents) {
        Ok(settings) => Ok(settings),
        Err(e) => {
            tracing::warn!("Failed to parse settings file, using defaults: {}", e);
            Ok(AppSettings::default())
        }
    }
}

pub fn save_settings(app: &AppHandle, settings: &AppSettings) -> CommandResult<()> {
    tracing::debug!("Saving settings");
    let path = get_settings_path(app)?;

    let contents = serde_json::to_string_pretty(settings)
        .map_err(|e| CommandError::Internal(format!("Failed to serialize settings: {e}")))?;

    fs::write(&path, contents)?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn get_settings(app: AppHandle) -> CommandResult<AppSettings> {
    load_settings(&app)
}

#[tauri::command]
#[specta::specta]
pub async fn set_auth_mode(app: AppHandle, mode: AuthMode) -> CommandResult<AppSettings> {
    let mut settings = load_settings(&app)?;
    settings.auth_mode = mode;
    save_settings(&app, &settings)?;
    Ok(settings)
}

#[tauri::command]
#[specta::specta]
pub async fn set_theme(app: AppHandle, theme: Theme) -> CommandResult<AppSettings> {
    let mut settings = load_settings(&app)?;
    settings.theme = theme;
    save_settings(&app, &settings)?;
    Ok(settings)
}

#[tauri::command]
#[specta::specta]
pub async fn set_age_verified(app: AppHandle) -> CommandResult<AppSettings> {
    let mut settings = load_settings(&app)?;
    settings.age_verified = true;
    save_settings(&app, &settings)?;
    Ok(settings)
}

#[tauri::command]
#[specta::specta]
pub async fn set_locale(app: AppHandle, locale: Option<String>) -> CommandResult<AppSettings> {
    let mut settings = load_settings(&app)?;
    settings.locale = locale;
    save_settings(&app, &settings)?;
    Ok(settings)
}

#[tauri::command]
#[specta::specta]
pub async fn toggle_server_notifications(
    app: AppHandle,
    server_name: String,
    enabled: bool,
) -> CommandResult<AppSettings> {
    let mut settings = load_settings(&app)?;
    if enabled {
        settings.notification_servers.insert(server_name);
    } else {
        settings.notification_servers.remove(&server_name);
    }
    save_settings(&app, &settings)?;

    Ok(settings)
}

#[tauri::command]
#[specta::specta]
pub async fn set_rendering_pipeline(
    app: AppHandle,
    pipeline: RenderingPipeline,
) -> CommandResult<AppSettings> {
    let mut settings = load_settings(&app)?;
    settings.rendering_pipeline = pipeline;
    save_settings(&app, &settings)?;
    Ok(settings)
}
