use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io;
#[cfg(target_os = "linux")]
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Manager};

use crate::auth::TokenStorage;
use crate::error::{CommandError, CommandResult};
use crate::relays::RelayState;
use crate::servers::ServerState;
use crate::settings::{load_settings, AuthMode};

#[cfg(any(target_os = "windows", target_os = "linux"))]
use crate::byond_login::{check_byond_web_session, start_byond_login};
#[cfg(any(target_os = "windows", target_os = "linux"))]
use tauri::Emitter;

#[cfg(target_os = "windows")]
use std::process::Command;

#[cfg(target_os = "linux")]
use crate::wine;

#[cfg(feature = "steam")]
use crate::steam::{authenticate_with_steam, SteamState};

static CONNECTING: AtomicBool = AtomicBool::new(false);

pub struct ConnectionRequest {
    pub version: String,
    pub host: String,
    pub port: String,
    pub access_type: Option<String>,
    pub access_token: Option<String>,
    pub server_name: String,
    pub map_name: Option<String>,
    pub source: Option<String>,
    pub server_id: Option<String>,
    pub players: Option<i32>,
}

const VERSIONS_FILE: &str = "byond_versions.json";

const ALLOWED_BIN_FILES: &[&str] = &[
    "dreamseeker.exe",
    "byond.exe",
    "byondcore.dll",
    "byondext.dll",
    "byondwin.dll",
    "WebView2Loader.dll",
    "fmodex.dll",
];

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ByondVersionEntry {
    pub installed_at: String,
    pub last_used: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ByondVersionStore {
    pub versions: HashMap<String, ByondVersionEntry>,
}

fn load_version_store(app: &AppHandle) -> CommandResult<ByondVersionStore> {
    let path = get_byond_base_dir(app)?.join(VERSIONS_FILE);
    if !path.exists() {
        return Ok(ByondVersionStore::default());
    }
    let contents = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Failed to read version store, using defaults: {}", e);
            return Ok(ByondVersionStore::default());
        }
    };
    match serde_json::from_str(&contents) {
        Ok(store) => Ok(store),
        Err(e) => {
            tracing::warn!("Failed to parse version store, using defaults: {}", e);
            Ok(ByondVersionStore::default())
        }
    }
}

fn save_version_store(app: &AppHandle, store: &ByondVersionStore) -> CommandResult<()> {
    let base = get_byond_base_dir(app)?;
    fs::create_dir_all(&base)?;
    let path = base.join(VERSIONS_FILE);
    let contents = serde_json::to_string_pretty(store)
        .map_err(|e| CommandError::Internal(format!("Failed to serialize version store: {e}")))?;
    fs::write(&path, contents)?;
    Ok(())
}

fn record_version_installed(app: &AppHandle, version: &str) -> CommandResult<()> {
    let mut store = load_version_store(app)?;
    store.versions.insert(
        version.to_string(),
        ByondVersionEntry {
            installed_at: chrono::Utc::now().to_rfc3339(),
            last_used: None,
        },
    );
    save_version_store(app, &store)
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn record_version_used(app: &AppHandle, version: &str) -> CommandResult<()> {
    let mut store = load_version_store(app)?;
    if let Some(entry) = store.versions.get_mut(version) {
        entry.last_used = Some(chrono::Utc::now().to_rfc3339());
    } else {
        store.versions.insert(
            version.to_string(),
            ByondVersionEntry {
                installed_at: chrono::Utc::now().to_rfc3339(),
                last_used: Some(chrono::Utc::now().to_rfc3339()),
            },
        );
    }
    save_version_store(app, &store)
}

fn remove_version_from_store(app: &AppHandle, version: &str) -> CommandResult<()> {
    let mut store = load_version_store(app)?;
    store.versions.remove(version);
    save_version_store(app, &store)
}

/// Remove old BYOND versions that are not in the 10 most recently used
/// and were last used more than 30 days ago.
pub fn cleanup_old_versions(app: &AppHandle) {
    let store = match load_version_store(app) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Failed to load version store for cleanup: {}", e);
            return;
        }
    };

    if store.versions.is_empty() {
        return;
    }

    let now = chrono::Utc::now();
    let cutoff = now - chrono::Duration::days(30);

    let mut sorted: Vec<(&String, &ByondVersionEntry)> = store.versions.iter().collect();
    sorted.sort_by(|a, b| {
        let a_time = a.1.last_used.as_deref().unwrap_or("");
        let b_time = b.1.last_used.as_deref().unwrap_or("");
        b_time.cmp(a_time)
    });

    let to_check = sorted.into_iter().skip(10);

    let mut versions_to_remove = Vec::new();
    for (version, entry) in to_check {
        let is_old = match &entry.last_used {
            Some(ts) => chrono::DateTime::parse_from_rfc3339(ts)
                .map(|t| t < cutoff)
                .unwrap_or(true),
            None => {
                // Never used — check installed_at instead
                chrono::DateTime::parse_from_rfc3339(&entry.installed_at)
                    .map(|t| t < cutoff)
                    .unwrap_or(true)
            }
        };
        if is_old {
            versions_to_remove.push(version.clone());
        }
    }

    for version in &versions_to_remove {
        match get_byond_version_dir(app, version) {
            Ok(dir) => {
                if dir.exists() {
                    if let Err(e) = fs::remove_dir_all(&dir) {
                        tracing::warn!("Failed to remove old BYOND version {}: {}", version, e);
                        continue;
                    }
                }
                tracing::info!("Cleaned up old BYOND version: {}", version);
            }
            Err(e) => {
                tracing::warn!("Failed to get path for BYOND version {}: {}", version, e);
                continue;
            }
        }
        if let Err(e) = remove_version_from_store(app, version) {
            tracing::warn!("Failed to remove {} from version store: {}", version, e);
        }
    }

    if !versions_to_remove.is_empty() {
        tracing::info!(
            "Cleaned up {} old BYOND version(s)",
            versions_to_remove.len()
        );
    }
}

/// Trim a BYOND installation to only the files needed at runtime.
fn trim_byond_install(version_dir: &std::path::Path) -> CommandResult<()> {
    let byond_dir = version_dir.join("byond");
    if !byond_dir.exists() {
        return Ok(());
    }

    let entries = fs::read_dir(&byond_dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        if name == "bin" {
            continue;
        }
        if path.is_dir() {
            fs::remove_dir_all(&path).ok();
        } else {
            fs::remove_file(&path).ok();
        }
    }

    let bin_dir = byond_dir.join("bin");
    if bin_dir.exists() {
        let bin_entries = fs::read_dir(&bin_dir)?;
        for entry in bin_entries {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy().to_lowercase();
            let allowed = ALLOWED_BIN_FILES
                .iter()
                .any(|f| f.to_lowercase() == name_str);
            if !allowed {
                if path.is_dir() {
                    fs::remove_dir_all(&path).ok();
                } else {
                    fs::remove_file(&path).ok();
                }
            }
        }
    }

    Ok(())
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
pub struct ByondVersionInfo {
    pub version: String,
    pub installed: bool,
    pub path: Option<String>,
    pub last_used: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
pub struct AuthError {
    pub code: String,
    pub message: String,
    pub linking_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
pub struct ConnectionResult {
    pub success: bool,
    pub message: String,
    pub auth_error: Option<AuthError>,
}

use crate::servers::EngineRequirements;

fn parse_byond_version(v: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() != 2 {
        return None;
    }
    let major = parts.first()?.parse::<u32>().ok()?;
    let minor = parts.get(1)?.parse::<u32>().ok()?;
    Some((major, minor))
}

fn version_cmp(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    let a = parse_byond_version(a)?;
    let b = parse_byond_version(b)?;
    Some(a.cmp(&b))
}

/// Select the best BYOND version to use given engine constraints.
/// Returns the version string to use (may need to be installed).
pub fn select_byond_version(
    engine: Option<&EngineRequirements>,
    app: &AppHandle,
) -> CommandResult<String> {
    let config = crate::config::get_config();
    let default_version = config.default_byond_version.map(str::to_string);

    let no_default = || CommandError::NotConfigured {
        feature: "default_byond_version".to_string(),
    };

    let Some(engine) = engine else {
        return default_version.ok_or_else(no_default);
    };

    if engine.min_version.is_none()
        && engine.max_version.is_none()
        && engine.blacklisted_versions.is_empty()
    {
        return default_version.ok_or_else(no_default);
    }

    let store = load_version_store(app)?;
    let installed: Vec<&String> = store.versions.keys().collect();

    // Filter installed versions by constraints
    let mut valid: Vec<&String> = installed
        .into_iter()
        .filter(|v| {
            if engine.blacklisted_versions.contains(v) {
                return false;
            }
            if let Some(ref min) = engine.min_version {
                if version_cmp(v, min) == Some(std::cmp::Ordering::Less) {
                    return false;
                }
            }
            if let Some(ref max) = engine.max_version {
                if version_cmp(v, max) == Some(std::cmp::Ordering::Greater) {
                    return false;
                }
            }
            true
        })
        .collect();

    // Sort by version descending, pick highest
    valid.sort_by(|a, b| version_cmp(b, a).unwrap_or(std::cmp::Ordering::Equal));

    if let Some(best) = valid.first() {
        return Ok((*best).clone());
    }

    // No valid installed version — determine what to download
    if let Some(ref max) = engine.max_version {
        // If max is set (whether or not min is set), download max
        if !engine.blacklisted_versions.contains(max) {
            return Ok(max.clone());
        }
    }
    if let Some(ref min) = engine.min_version {
        if !engine.blacklisted_versions.contains(min) {
            return Ok(min.clone());
        }
    }

    // All constraint versions are blacklisted, fall back to default
    default_version.ok_or_else(|| {
        CommandError::NotFound(
            "no BYOND version satisfies engine constraints (all candidates are blacklisted)"
                .to_string(),
        )
    })
}

/// Build a BYOND connection URL with optional auth and launcher ports.
pub fn build_connect_url(
    host: &str,
    port: &str,
    access_type: Option<&str>,
    access_token: Option<&str>,
    launcher_port: Option<&str>,
    launcher_key: Option<&str>,
    websocket_port: Option<&str>,
) -> String {
    let mut query_params = Vec::new();
    if let (Some(access_type), Some(token)) = (access_type, access_token) {
        query_params.push(format!("{access_type}={token}"));
    }

    if let Some(port) = launcher_port {
        query_params.push(format!("launcher_port={port}"));
    }

    if let Some(key) = launcher_key {
        query_params.push(format!("launcher_key={key}"));
    }

    if let Some(port) = websocket_port {
        query_params.push(format!("websocket_port={port}"));
    }

    if query_params.is_empty() {
        format!("byond://{host}:{port}")
    } else {
        format!("byond://{}:{}?{}", host, port, query_params.join("&"))
    }
}

pub fn get_byond_base_dir(_app: &AppHandle) -> CommandResult<PathBuf> {
    let config = crate::config::get_config();
    let local_data = dirs::data_local_dir()
        .ok_or_else(|| CommandError::Io("local data directory unavailable".to_string()))?
        .join(config.app_identifier);

    Ok(local_data.join("byond"))
}

fn get_byond_version_dir(app: &AppHandle, version: &str) -> CommandResult<PathBuf> {
    let base = get_byond_base_dir(app)?;
    Ok(base.join(version))
}

#[cfg(target_os = "windows")]
fn get_dreamseeker_path(app: &AppHandle, version: &str) -> CommandResult<PathBuf> {
    let version_dir = get_byond_version_dir(app, version)?;
    Ok(version_dir
        .join("byond")
        .join("bin")
        .join("dreamseeker.exe"))
}

#[cfg(target_os = "linux")]
fn get_dreamseeker_path(app: &AppHandle, version: &str) -> CommandResult<PathBuf> {
    let version_dir = get_byond_version_dir(app, version)?;
    Ok(version_dir
        .join("byond")
        .join("bin")
        .join("dreamseeker.exe"))
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn get_dreamseeker_path(_app: &AppHandle, _version: &str) -> CommandResult<PathBuf> {
    Err(CommandError::UnsupportedPlatform {
        feature: "byond".into(),
        platform: std::env::consts::OS.into(),
    })
}

#[cfg(target_os = "windows")]
fn get_byond_pager_path(app: &AppHandle, version: &str) -> CommandResult<PathBuf> {
    let version_dir = get_byond_version_dir(app, version)?;
    Ok(version_dir.join("byond").join("bin").join("byond.exe"))
}

#[tauri::command]
#[specta::specta]
pub async fn check_byond_version(
    app: AppHandle,
    version: String,
) -> CommandResult<ByondVersionInfo> {
    tracing::debug!("Checking BYOND version: {}", version);
    let dreamseeker_path = get_dreamseeker_path(&app, &version)?;
    let installed = dreamseeker_path.exists();

    let last_used = if installed {
        load_version_store(&app)
            .ok()
            .and_then(|s| s.versions.get(&version).and_then(|e| e.last_used.clone()))
    } else {
        None
    };

    Ok(ByondVersionInfo {
        version,
        installed,
        path: if installed {
            Some(dreamseeker_path.to_string_lossy().to_string())
        } else {
            None
        },
        last_used,
    })
}

#[allow(clippy::indexing_slicing)] // length checked above
fn get_byond_download_urls(version: &str) -> CommandResult<(String, String)> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 2 {
        return Err(CommandError::InvalidInput(format!(
            "Invalid BYOND version format: {version}"
        )));
    }

    let major = parts[0];

    let primary = format!("https://www.byond.com/download/build/{major}/{version}_byond.zip");
    let fallback = format!("https://byond-builds.dm-lang.org/{major}/{version}_byond.zip");

    Ok((primary, fallback))
}

async fn try_download(url: &str) -> CommandResult<Vec<u8>> {
    let response = reqwest::get(url).await?;

    if !response.status().is_success() {
        return Err(CommandError::InvalidResponse(format!(
            "HTTP {}",
            response.status()
        )));
    }

    let bytes = response.bytes().await?;

    Ok(bytes.to_vec())
}

#[derive(Debug, Deserialize)]
struct ByondHashResponse {
    sha256: Option<String>,
}

async fn fetch_expected_hash(version: &str) -> CommandResult<Option<String>> {
    let config = crate::config::get_config();

    // If no BYOND hash API is configured for this variant, skip verification
    let Some(base_url) = config.urls.byond_hash_api else {
        tracing::debug!("No BYOND hash API configured for this variant");
        return Ok(None);
    };

    let url = format!("{base_url}?byond_ver={version}");

    let response = reqwest::get(&url).await?;

    if !response.status().is_success() {
        tracing::warn!(
            "Hash API returned HTTP {} for version {}",
            response.status(),
            version
        );
        return Ok(None);
    }

    let hash_response: ByondHashResponse = response.json().await.map_err(|e| {
        CommandError::InvalidResponse(format!("Failed to parse hash response: {e}"))
    })?;

    Ok(hash_response.sha256)
}

fn verify_sha256(data: &[u8], expected_hex: &str) -> CommandResult<()> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let actual_hex = hex::encode(result);

    if actual_hex.eq_ignore_ascii_case(expected_hex) {
        Ok(())
    } else {
        Err(CommandError::InvalidResponse(format!(
            "SHA-256 mismatch: expected {expected_hex}, got {actual_hex}"
        )))
    }
}

#[tauri::command]
#[specta::specta]
pub async fn install_byond_version(
    app: AppHandle,
    version: String,
) -> CommandResult<ByondVersionInfo> {
    let existing = check_byond_version(app.clone(), version.clone()).await?;
    if existing.installed {
        tracing::debug!("BYOND version {} already installed", version);
        return Ok(existing);
    }

    tracing::info!("Installing BYOND version: {}", version);
    let (primary_url, fallback_url) = get_byond_download_urls(&version)?;
    let version_dir = get_byond_version_dir(&app, &version)?;

    fs::create_dir_all(&version_dir)?;

    let zip_path = version_dir.join("byond.zip");

    let bytes = match try_download(&primary_url).await {
        Ok(b) => b,
        Err(primary_err) => {
            tracing::warn!(
                "Primary BYOND download failed ({}), trying fallback URL",
                primary_err
            );
            try_download(&fallback_url).await.map_err(|fallback_err| {
                CommandError::Network(format!(
                    "Failed to download BYOND {version}: primary={primary_err}, fallback={fallback_err}"
                ))
            })?
        }
    };

    // Verify download integrity using SHA-256 hash from API
    match fetch_expected_hash(&version).await {
        Ok(Some(expected_hash)) => {
            verify_sha256(&bytes, &expected_hash).map_err(|e| {
                tracing::error!("BYOND {} integrity check failed: {}", version, e);
                e
            })?;
            tracing::info!("BYOND {} SHA-256 verified successfully", version);
        }
        Ok(None) => {
            tracing::warn!(
                "No SHA-256 hash available for BYOND {}, skipping verification",
                version
            );
        }
        Err(e) => {
            tracing::warn!(
                "Failed to fetch hash for BYOND {}: {}, skipping verification",
                version,
                e
            );
        }
    }

    fs::write(&zip_path, &bytes)?;

    let file = fs::File::open(&zip_path)?;

    let mut archive = zip::ZipArchive::new(file).map_err(|e| {
        CommandError::InvalidResponse(format!("Downloaded BYOND archive is not a valid zip: {e}"))
    })?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| {
            CommandError::InvalidResponse(format!("Corrupt entry in BYOND zip: {e}"))
        })?;

        let outpath = match file.enclosed_name() {
            Some(path) => version_dir.join(path),
            None => continue,
        };

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            let mut outfile = fs::File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode)).ok();
            }
        }
    }

    fs::remove_file(&zip_path).ok();

    // On Linux, run BYOND's bundled DirectX installer via Wine
    #[cfg(target_os = "linux")]
    {
        let dx_installer = version_dir
            .join("byond")
            .join("directx")
            .join("DXSETUP.exe");

        if dx_installer.exists() {
            tracing::info!("Running BYOND's bundled DirectX installer via Wine");
            match wine::launch_with_wine(&app, &dx_installer, &["/silent"], &[]) {
                Ok(mut child) => {
                    // Wait for installer to complete (with timeout)
                    let timeout = tokio::time::Duration::from_secs(60);
                    let start = std::time::Instant::now();
                    loop {
                        match child.try_wait() {
                            Ok(Some(_)) => {
                                tracing::info!("BYOND DirectX installer completed");
                                break;
                            }
                            Ok(None) => {
                                if start.elapsed() > timeout {
                                    tracing::warn!("BYOND DirectX installer timed out");
                                    let _ = child.kill();
                                    break;
                                }
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                            }
                            Err(e) => {
                                tracing::warn!("Error waiting for DirectX installer: {}", e);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to run BYOND DirectX installer: {}", e);
                }
            }
        }
    }

    trim_byond_install(&version_dir)?;
    record_version_installed(&app, &version)?;

    tracing::info!("BYOND version {} installed successfully", version);

    check_byond_version(app, version).await
}

/// Internal function for connecting with explicit auth params.
pub async fn connect(app: AppHandle, req: ConnectionRequest) -> CommandResult<ConnectionResult> {
    let source_str = req.source.as_deref().unwrap_or("unknown");

    if CONNECTING.swap(true, Ordering::SeqCst) {
        tracing::warn!(
            "[connect_to_server] BLOCKED duplicate connection attempt, source={} server={}",
            source_str,
            req.server_name
        );
        return Ok(ConnectionResult {
            success: false,
            message: "Connection already in progress".to_string(),
            auth_error: None,
        });
    }

    tracing::info!(
        "[connect_to_server] source={} server={} version={}",
        source_str,
        req.server_name,
        req.version
    );

    let result = connect_impl(app, req).await;

    CONNECTING.store(false, Ordering::SeqCst);
    result
}

#[allow(clippy::unused_async)] // Uses await when steam feature is enabled
async fn get_auth_for_connection(
    app: &AppHandle,
    auth_methods: &[String],
) -> Result<(Option<String>, Option<String>), AuthError> {
    let supports_hub = auth_methods.iter().any(|m| m == "hub");
    let supports_byond = auth_methods.iter().any(|m| m == "byond");

    let settings = load_settings(app).map_err(|e| AuthError {
        code: "settings_error".to_string(),
        message: e.to_string(),
        linking_url: None,
    })?;

    let effective_mode = match settings.auth_mode {
        AuthMode::Oidc => AuthMode::Oidc,
        AuthMode::Steam => AuthMode::Steam,
        AuthMode::Hub if supports_hub => AuthMode::Hub,
        AuthMode::Hub | AuthMode::Byond if supports_byond => AuthMode::Byond,
        _ if supports_hub => AuthMode::Hub,
        _ => AuthMode::Byond,
    };

    match effective_mode {
        AuthMode::Oidc | AuthMode::Hub => {
            let tokens = TokenStorage::get_tokens().map_err(|e| AuthError {
                code: "token_error".to_string(),
                message: e.to_string(),
                linking_url: None,
            })?;

            match tokens {
                Some(t) if !TokenStorage::is_expired() => {
                    let config = crate::config::get_config();
                    Ok((Some(config.variant.to_string()), Some(t.access_token)))
                }
                _ => {
                    let config = crate::config::get_config();
                    Err(AuthError {
                        code: "auth_required".to_string(),
                        message: config.strings.login_prompt.to_string(),
                        linking_url: None,
                    })
                }
            }
        }
        AuthMode::Steam => {
            #[cfg(feature = "steam")]
            {
                let steam_state = app
                    .try_state::<Arc<SteamState>>()
                    .ok_or_else(|| AuthError {
                        code: "steam_unavailable".to_string(),
                        message: "Steam is not available".to_string(),
                        linking_url: None,
                    })?;

                let result = authenticate_with_steam(&steam_state, false)
                    .await
                    .map_err(|e| AuthError {
                        code: "steam_error".to_string(),
                        message: e.to_string(),
                        linking_url: None,
                    })?;

                if result.success {
                    Ok((Some("steam".to_string()), result.access_token))
                } else if result.requires_linking {
                    Err(AuthError {
                        code: "steam_linking_required".to_string(),
                        message: "Steam account linking required".to_string(),
                        linking_url: result.linking_url,
                    })
                } else {
                    Err(AuthError {
                        code: "steam_auth_failed".to_string(),
                        message: result
                            .error
                            .unwrap_or_else(|| "Steam authentication failed".to_string()),
                        linking_url: None,
                    })
                }
            }

            #[cfg(not(feature = "steam"))]
            {
                Err(AuthError {
                    code: "steam_unavailable".to_string(),
                    message: "Steam support not compiled".to_string(),
                    linking_url: None,
                })
            }
        }
        AuthMode::Byond => {
            let config = crate::config::get_config();
            if !config.features.auto_launch_byond && !check_byond_pager_running() {
                return Err(AuthError {
                    code: "byond_auth_required".to_string(),
                    message: "Please log in to BYOND before connecting.".to_string(),
                    linking_url: None,
                });
            }
            Ok((Some("byond".to_string()), None))
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn connect_to_server(
    app: AppHandle,
    server_name: String,
    source: Option<String>,
) -> CommandResult<ConnectionResult> {
    let source_str = source.as_deref().unwrap_or("unknown");

    let server_state = app
        .try_state::<Arc<ServerState>>()
        .ok_or_else(|| CommandError::Internal("server state not available".into()))?;
    let servers = server_state.get_servers().await;
    let server = servers
        .iter()
        .find(|s| s.name == server_name)
        .ok_or_else(|| CommandError::NotFound(format!("server '{server_name}'")))?
        .clone();

    let version = select_byond_version(server.engine.as_ref(), &app)?;

    let config = crate::config::get_config();

    // Parse host and port from server URL (format: byond://host:port)
    let address = server.url.strip_prefix("byond://").unwrap_or(&server.url);

    let (host, port) = if config.features.relay_selector {
        // CM mode: use relay for host, extract port from server URL
        let port = address
            .split(':')
            .nth(1)
            .ok_or_else(|| {
                CommandError::InvalidInput(format!("Invalid server URL format: {}", server.url))
            })?
            .to_string();

        let relay_state = app
            .try_state::<Arc<RelayState>>()
            .ok_or_else(|| CommandError::Internal("relay state not available".into()))?;
        let host = relay_state
            .get_selected_host()
            .await
            .ok_or_else(|| CommandError::NotFound("no relay selected".into()))?;

        (host, port)
    } else {
        // SS13 mode: use host:port directly from server URL
        let parts: Vec<&str> = address.split(':').collect();
        if parts.len() != 2 {
            return Err(CommandError::InvalidInput(format!(
                "Invalid server URL format: {}",
                server.url
            )));
        }
        #[allow(clippy::indexing_slicing)] // length checked above
        (parts[0].to_string(), parts[1].to_string())
    };

    let (access_type, access_token) =
        match get_auth_for_connection(&app, &server.auth_methods).await {
            Ok((t, tok)) => (t, tok),
            Err(auth_error) => {
                return Ok(ConnectionResult {
                    success: false,
                    message: auth_error.message.clone(),
                    auth_error: Some(auth_error),
                });
            }
        };

    // For hub auth, exchange the session token for a short-lived auth ticket
    let hub_active = access_type.as_deref() != Some("byond")
        && access_type.as_deref() != Some("steam")
        && server.auth_methods.iter().any(|m| m == "hub");
    let (access_type, access_token) = if hub_active {
        if let Some(session_token) = &access_token {
            let server_id = server
                .id
                .as_deref()
                .ok_or_else(|| CommandError::InvalidInput("server has no hub ID".into()))?;
            let hwid = crate::control_server::generate_hwid();
            match crate::auth::hub_client::HubClient::join(
                session_token,
                server_id,
                hwid.as_deref(),
            )
            .await
            {
                Ok(ticket) => (Some("auth_ticket".to_string()), Some(ticket)),
                Err(e) => {
                    return Ok(ConnectionResult {
                        success: false,
                        message: format!("Failed to get auth ticket: {e}"),
                        auth_error: Some(AuthError {
                            code: "ticket_error".to_string(),
                            message: format!("Failed to get auth ticket: {e}"),
                            linking_url: None,
                        }),
                    });
                }
            }
        } else {
            (access_type, access_token)
        }
    } else {
        (access_type, access_token)
    };

    let map_name = server.data.map(|d| d.map_name);

    tracing::info!(
        "[connect_to_server] source={} server={} version={} host={}",
        source_str,
        server_name,
        version,
        host
    );

    connect(
        app,
        ConnectionRequest {
            version,
            host,
            port,
            access_type,
            access_token,
            server_name,
            map_name,
            source,
            server_id: server.id,
            players: Some(server.players),
        },
    )
    .await
}

async fn connect_impl(app: AppHandle, req: ConnectionRequest) -> CommandResult<ConnectionResult> {
    let ConnectionRequest {
        version,
        host,
        port,
        access_type,
        access_token,
        server_name,
        map_name,
        source,
        server_id,
        players,
    } = req;

    let version_info = install_byond_version(app.clone(), version.clone()).await?;

    if !version_info.installed {
        let msg = format!("Failed to install BYOND version {version}");
        tracing::error!("{}", msg);
        return Err(CommandError::Internal(msg));
    }

    let dreamseeker_path = version_info
        .path
        .ok_or_else(|| CommandError::NotFound("dreamseeker executable".into()))?;

    #[cfg(target_os = "linux")]
    {
        let status = wine::check_prefix_status(&app).await;
        if !status.prefix_initialized || !status.webview2_installed {
            return Err(CommandError::NotConfigured {
                feature: "wine_prefix".into(),
            });
        }
    }

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        use std::sync::Arc;

        use crate::control_server::ControlServer;
        use crate::presence::{ConnectionParams, PresenceManager};

        let config = crate::config::get_config();

        if let Some(control_server) = app.try_state::<ControlServer>() {
            control_server.reset_connected_flag();
        }

        let control_port = app.try_state::<ControlServer>().map(|s| s.port.to_string());
        let launcher_key = app.try_state::<ControlServer>().map(|s| s.rotate_key());
        let websocket_port = app
            .try_state::<ControlServer>()
            .map(|s| s.ws_port.to_string());

        let webview2_data_dir = get_byond_base_dir(&app)?.join("webview2_data");

        let is_byond_auth = access_type.as_deref() == Some("byond");
        let pager_running = check_byond_pager_running();

        let mut session_check = if is_byond_auth {
            check_byond_web_session(app.clone()).await.ok()
        } else {
            None
        };

        let using_webid = match &session_check {
            Some(session) if session.logged_in => {
                tracing::info!("User logged in via web (web_id present), using web authentication");
                true
            }
            _ if !pager_running && is_byond_auth => {
                tracing::info!("Not logged in to BYOND and pager not running, opening login flow");
                let login_result = start_byond_login(app.clone()).await;
                if login_result.is_err() {
                    return Err(CommandError::Cancelled {
                        operation: "byond_login".into(),
                    });
                }
                session_check = check_byond_web_session(app.clone()).await.ok();
                true
            }
            _ => {
                if is_byond_auth {
                    tracing::info!("Using BYOND pager for authentication");
                }
                false
            }
        };

        if source.as_deref() != Some("control_server_restart") {
            app.emit("game-connecting", &server_name).ok();
        }

        if using_webid {
            let session = if session_check.as_ref().map(|s| s.logged_in).unwrap_or(false) {
                session_check.unwrap()
            } else {
                check_byond_web_session(app.clone()).await?
            };
            let web_id = session.web_id.ok_or(CommandError::NotAuthenticated)?;
            if !session.logged_in {
                return Err(CommandError::NotAuthenticated);
            }
            tracing::info!("Got web_id, launching byond.exe with web authentication");

            let mut existing_pids = get_dreamseeker_pids();

            let mut query_params = Vec::new();
            if let Some(lp) = &control_port {
                query_params.push(format!("launcher_port={}", lp));
            }
            if let Some(lk) = &launcher_key {
                query_params.push(format!("launcher_key={}", lk));
            }
            if let Some(wp) = &websocket_port {
                query_params.push(format!("websocket_port={}", wp));
            }

            let connect_url = if query_params.is_empty() {
                format!("byond://{}:{}##webid={}", host, port, web_id)
            } else {
                format!(
                    "byond://{}:{}?{}##webid={}",
                    host,
                    port,
                    query_params.join("&"),
                    web_id
                )
            };

            #[cfg(target_os = "windows")]
            let mut pager_child = {
                let byond_pager_path = get_byond_pager_path(&app, &version)?;
                Command::new(&byond_pager_path)
                    .arg(&connect_url)
                    .env("WEBVIEW2_USER_DATA_FOLDER", &webview2_data_dir)
                    .spawn()?
            };

            #[cfg(target_os = "linux")]
            let mut pager_child = {
                let version_dir = get_byond_version_dir(&app, &version)?;
                let exe_path = version_dir.join("byond").join("bin").join("byond.exe");
                wine::launch_with_wine(
                    &app,
                    &exe_path,
                    &[&connect_url],
                    &[(
                        "WEBVIEW2_USER_DATA_FOLDER",
                        webview2_data_dir.to_str().unwrap(),
                    )],
                )
                .map_err(|e| CommandError::Io(format!("Failed to launch BYOND via Wine: {e}")))?
            };

            existing_pids.insert(pager_child.id());

            let dreamseeker_pid = wait_for_new_dreamseeker(existing_pids, 30).await;

            if dreamseeker_pid.is_some() {
                tracing::info!("Waiting 5s for dreamseeker to authenticate before killing pager");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                tracing::info!("Killing byond.exe pager");
                let _ = pager_child.kill();
            }

            if let Some(manager) = app.try_state::<Arc<PresenceManager>>() {
                manager.set_last_connection_params(ConnectionParams {
                    version: version.clone(),
                    host: host.clone(),
                    port: port.clone(),
                    access_type,
                    access_token,
                    server_name: server_name.clone(),
                    map_name: map_name.clone(),
                    server_id: server_id.clone(),
                    launcher_key: launcher_key.clone(),
                });

                if let Some(pid) = dreamseeker_pid {
                    manager.start_game_session_by_pid(server_name.clone(), map_name.clone(), players.unwrap_or(0) as u32, pid);
                } else {
                    tracing::warn!(
                        "Could not find dreamseeker.exe, presence tracking may not work"
                    );
                }
            }
        } else {
            let connect_url = build_connect_url(
                &host,
                &port,
                access_type.as_deref(),
                access_token.as_deref(),
                control_port.as_deref(),
                launcher_key.as_deref(),
                websocket_port.as_deref(),
            );

            #[cfg(target_os = "windows")]
            let child = Command::new(&dreamseeker_path)
                .arg(&connect_url)
                .env("WEBVIEW2_USER_DATA_FOLDER", &webview2_data_dir)
                .spawn()?;

            #[cfg(target_os = "linux")]
            let child = wine::launch_with_wine(
                &app,
                Path::new(&dreamseeker_path),
                &[&connect_url],
                &[(
                    "WEBVIEW2_USER_DATA_FOLDER",
                    webview2_data_dir.to_str().unwrap(),
                )],
            )
            .map_err(|e| CommandError::Io(format!("Failed to launch BYOND via Wine: {e}")))?;

            if let Some(manager) = app.try_state::<Arc<PresenceManager>>() {
                manager.set_last_connection_params(ConnectionParams {
                    version: version.clone(),
                    host: host.clone(),
                    port: port.clone(),
                    access_type,
                    access_token,
                    server_name: server_name.clone(),
                    map_name: map_name.clone(),
                    server_id: server_id.clone(),
                    launcher_key: launcher_key.clone(),
                });

                manager.start_game_session(server_name.clone(), map_name.clone(), players.unwrap_or(0) as u32, child);
            }
        }

        if config.features.connection_timeout_fallback {
            if let Some(manager) = app.try_state::<Arc<PresenceManager>>() {
                let app_clone = app.clone();
                let server_name_clone = server_name.clone();
                let manager_clone = Arc::clone(&manager);
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                    if manager_clone.get_game_session().is_some() {
                        app_clone.emit("game-connected", &server_name_clone).ok();
                    }
                });
            }
        }

        // Record last-used timestamp
        if let Err(e) = record_version_used(&app, &version) {
            tracing::warn!("Failed to record BYOND version usage: {}", e);
        }

        #[cfg(target_os = "windows")]
        let message = format!("Connecting to {} with BYOND {}", host, version);
        #[cfg(target_os = "linux")]
        let message = format!("Connecting to {} with BYOND {} (via Wine)", host, version);

        return Ok(ConnectionResult {
            success: true,
            message,
            auth_error: None,
        });
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let _ = (
            dreamseeker_path,
            host,
            port,
            server_name,
            access_type,
            access_token,
            source,
            map_name,
        );
        Err(CommandError::UnsupportedPlatform {
            feature: "byond".into(),
            platform: std::env::consts::OS.into(),
        })
    }
}

#[tauri::command]
#[specta::specta]
pub async fn list_installed_byond_versions(app: AppHandle) -> CommandResult<Vec<ByondVersionInfo>> {
    let base_dir = get_byond_base_dir(&app)?;

    if !base_dir.exists() {
        return Ok(vec![]);
    }

    let mut store = load_version_store(&app)?;
    let mut versions = Vec::new();
    let mut store_changed = false;

    let entries = fs::read_dir(&base_dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(version_name) = path.file_name().and_then(|n| n.to_str()) {
                let info = check_byond_version(app.clone(), version_name.to_string()).await?;
                if info.installed {
                    versions.push(info);
                }
            }
        }
    }

    let installed_versions: Vec<String> = versions.iter().map(|v| v.version.clone()).collect();
    let stale_keys: Vec<String> = store
        .versions
        .keys()
        .filter(|k| !installed_versions.contains(k))
        .cloned()
        .collect();
    for key in stale_keys {
        store.versions.remove(&key);
        store_changed = true;
    }

    if store_changed {
        save_version_store(&app, &store)?;
    }

    Ok(versions)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_byond_version(app: AppHandle, version: String) -> CommandResult<bool> {
    let version_dir = get_byond_version_dir(&app, &version)?;

    if version_dir.exists() {
        tracing::info!("Deleting BYOND version: {}", version);
        fs::remove_dir_all(&version_dir)?;
        remove_version_from_store(&app, &version)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn check_byond_pager_running() -> bool {
    #[cfg(target_os = "windows")]
    {
        use sysinfo::System;
        let s = System::new_all();
        s.processes().values().any(|p| {
            p.name()
                .to_str()
                .map(|name| name.eq_ignore_ascii_case("byond.exe"))
                .unwrap_or(false)
        })
    }

    #[cfg(target_os = "linux")]
    {
        use sysinfo::System;

        let s = System::new_all();
        s.processes().values().any(|p| {
            p.cmd().iter().any(|arg| {
                arg.to_str()
                    .map(|a| a.to_lowercase().ends_with("byond.exe"))
                    .unwrap_or(false)
            })
        })
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        false
    }
}

/// Get PIDs of all running dreamseeker.exe processes
#[allow(dead_code)]
fn get_dreamseeker_pids() -> std::collections::HashSet<u32> {
    use std::collections::HashSet;
    use sysinfo::System;

    let s = System::new_all();

    #[cfg(target_os = "windows")]
    {
        s.processes()
            .iter()
            .filter(|(_, p)| {
                p.name()
                    .to_str()
                    .map(|name| name.eq_ignore_ascii_case("dreamseeker.exe"))
                    .unwrap_or(false)
            })
            .map(|(pid, _)| pid.as_u32())
            .collect()
    }

    #[cfg(target_os = "linux")]
    {
        s.processes()
            .iter()
            .filter(|(_, p)| {
                p.name()
                    .to_str()
                    .map(|n| n.eq_ignore_ascii_case("dreamseeker.exe"))
                    .unwrap_or(false)
            })
            .map(|(pid, _)| pid.as_u32())
            .collect()
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let _ = s; // Suppress unused variable warning
        HashSet::new()
    }
}

/// Poll for a new dreamseeker.exe process that wasn't in the original set.
/// Returns the PID if found within timeout, None otherwise.
#[allow(dead_code)]
async fn wait_for_new_dreamseeker(
    existing_pids: std::collections::HashSet<u32>,
    timeout_secs: u64,
) -> Option<u32> {
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(timeout_secs);

    loop {
        if start.elapsed() > timeout {
            tracing::warn!("Timeout waiting for dreamseeker.exe to spawn");
            return None;
        }

        let current_pids = get_dreamseeker_pids();
        let new_pids: Vec<u32> = current_pids.difference(&existing_pids).copied().collect();

        if let Some(&pid) = new_pids.first() {
            tracing::info!("Found new dreamseeker.exe with PID {}", pid);
            return Some(pid);
        }

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn find_dreamseeker_pid_by_key(launcher_key: &str) -> Option<u32> {
    use sysinfo::System;

    let s = System::new_all();
    s.processes()
        .iter()
        .find(|(_, p)| {
            p.cmd().iter().any(|arg| {
                arg.to_str()
                    .map(|a| a.contains(launcher_key))
                    .unwrap_or(false)
            }) && p.cmd().iter().any(|arg| {
                arg.to_str()
                    .map(|a| a.to_lowercase().contains("dreamseeker.exe"))
                    .unwrap_or(false)
            })
        })
        .map(|(pid, _)| pid.as_u32())
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn kill_dreamseeker_by_key(launcher_key: &str) -> bool {
    use sysinfo::System;

    let s = System::new_all();
    let mut killed = false;
    for (pid, proc) in s.processes() {
        let cmd_matches = proc.cmd().iter().any(|arg| {
            arg.to_str()
                .map(|a| a.contains(launcher_key))
                .unwrap_or(false)
        });
        if cmd_matches {
            tracing::info!("Killing process {} (name={:?})", pid.as_u32(), proc.name());
            proc.kill();
            killed = true;
        }
    }
    killed
}

#[tauri::command]
#[specta::specta]
pub async fn is_byond_pager_running() -> CommandResult<bool> {
    Ok(check_byond_pager_running())
}

/// Get the logged-in BYOND username from Documents/BYOND/key.txt
#[tauri::command]
#[specta::specta]
pub async fn get_byond_username() -> CommandResult<Option<String>> {
    #[cfg(target_os = "windows")]
    {
        let documents = dirs::document_dir()
            .ok_or_else(|| CommandError::Io("Could not find Documents directory".into()))?;
        let key_path = documents.join("BYOND").join("key.txt");

        if !key_path.exists() {
            return Ok(None);
        }

        let contents = fs::read_to_string(&key_path)?;

        // Look for "BEGIN KEY <username>" but not "BEGIN KEY Guest"
        for line in contents.lines() {
            let line = line.trim();
            if let Some(username) = line.strip_prefix("BEGIN KEY ") {
                if !username.eq_ignore_ascii_case("Guest") {
                    return Ok(Some(username.to_string()));
                }
            }
        }

        Ok(None)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(None)
    }
}

#[tauri::command]
#[specta::specta]
pub fn is_dev_mode() -> bool {
    cfg!(feature = "dev")
}

#[tauri::command]
#[specta::specta]
pub async fn connect_to_url(
    app: AppHandle,
    url: String,
    version: String,
    source: Option<String>,
) -> CommandResult<ConnectionResult> {
    #[cfg(not(feature = "dev"))]
    {
        let _ = (app, url, version, source);
        Err(CommandError::NotConfigured {
            feature: "dev_mode".into(),
        })
    }

    #[cfg(feature = "dev")]
    {
        let url = url.strip_prefix("byond://").unwrap_or(&url).to_string();

        let Some((host, port)) = url.split_once(':') else {
            return Err(CommandError::InvalidInput(
                "Invalid URL format. Expected 'host:port'".into(),
            ));
        };
        let host = host.to_string();
        let port = port.to_string();

        let (access_type, access_token) = match get_auth_for_connection(&app).await {
            Ok((t, tok)) => (t, tok),
            Err(auth_error) => {
                return Ok(ConnectionResult {
                    success: false,
                    message: auth_error.message.clone(),
                    auth_error: Some(auth_error),
                });
            }
        };

        tracing::info!(
            "[connect_to_url] dev mode connection to {}:{} version={}",
            host,
            port,
            version
        );

        connect(
            app,
            ConnectionRequest {
                version,
                host: host.to_string(),
                port: port.to_string(),
                access_type,
                access_token,
                server_name: format!("Dev Server ({url})"),
                map_name: None,
                source,
                server_id: None,
                players: None,
            },
        )
        .await
        .map_err(CommandError::Internal)
    }
}
