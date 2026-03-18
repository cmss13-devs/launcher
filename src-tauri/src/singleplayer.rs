//! Single player mode support.
//!
//! This module handles downloading and extracting the latest game build
//! from GitHub releases. Only available when singleplayer is enabled in config.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;
#[cfg(any(target_os = "windows", target_os = "linux"))]
use std::sync::Arc;
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
use tauri::AppHandle;
#[cfg(any(target_os = "windows", target_os = "linux"))]
use tauri::{AppHandle, Emitter, Manager};

#[cfg(any(target_os = "windows", target_os = "linux"))]
use crate::byond::get_byond_base_dir;
use crate::byond::install_byond_version;
#[cfg(any(target_os = "windows", target_os = "linux"))]
use crate::presence::PresenceManager;

const SINGLEPLAYER_DIR: &str = "singleplayer";
const VERSION_FILE: &str = ".version";

fn get_singleplayer_config() -> Result<(String, String), String> {
    let config = crate::config::get_config();
    let repo = config
        .singleplayer
        .github_repo
        .ok_or("Singleplayer is not available for this launcher variant")?;
    let asset = config
        .singleplayer
        .build_asset_name
        .ok_or("Singleplayer is not available for this launcher variant")?;
    Ok((repo.to_string(), asset.to_string()))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SinglePlayerStatus {
    pub installed: bool,
    pub version: Option<String>,
    pub release_tag: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub name: String,
    pub published_at: String,
    pub download_url: Option<String>,
    pub size: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: String,
    published_at: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

fn get_singleplayer_base_dir() -> Result<PathBuf, String> {
    let config = crate::config::get_config();
    let local_data = dirs::data_local_dir()
        .ok_or("Failed to get local data directory")?
        .join(config.app_identifier);

    Ok(local_data.join(SINGLEPLAYER_DIR))
}

fn get_version_file_path() -> Result<PathBuf, String> {
    Ok(get_singleplayer_base_dir()?.join(VERSION_FILE))
}

fn read_installed_version() -> Option<String> {
    let version_path = get_version_file_path().ok()?;
    fs::read_to_string(version_path).ok()
}

fn write_installed_version(version: &str) -> Result<(), String> {
    let version_path = get_version_file_path()?;
    fs::write(&version_path, version).map_err(|e| format!("Failed to write version file: {}", e))
}

/// Fetch the latest release info from GitHub
async fn fetch_latest_release() -> Result<ReleaseInfo, String> {
    let (github_repo, build_asset_name) = get_singleplayer_config()?;

    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        github_repo
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("User-Agent", "CM-Launcher")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch release info: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("GitHub API returned HTTP {}", response.status()));
    }

    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse release info: {}", e))?;

    let build_asset = release.assets.iter().find(|a| a.name == build_asset_name);

    Ok(ReleaseInfo {
        tag_name: release.tag_name,
        name: release.name,
        published_at: release.published_at,
        download_url: build_asset.map(|a| a.browser_download_url.clone()),
        size: build_asset.map(|a| a.size),
    })
}

/// Download a file from a URL
async fn download_file(url: &str) -> Result<Vec<u8>, String> {
    tracing::info!("Downloading from {}", url);

    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("User-Agent", "CM-Launcher")
        .send()
        .await
        .map_err(|e| format!("Download request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with HTTP {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read download: {}", e))?;

    Ok(bytes.to_vec())
}

/// Extract a tar.zst archive to a directory
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
fn extract_tar_zst(data: &[u8], dest: &PathBuf) -> Result<(), String> {
    tracing::info!("Extracting archive to {:?}", dest);

    fs::create_dir_all(dest).map_err(|e| format!("Failed to create directory: {}", e))?;

    let cursor = io::Cursor::new(data);
    let zstd_decoder = zstd::stream::Decoder::new(cursor)
        .map_err(|e| format!("Failed to create zstd decoder: {}", e))?;

    let mut archive = tar::Archive::new(zstd_decoder);
    archive.set_preserve_permissions(true);

    for entry in archive
        .entries()
        .map_err(|e| format!("Failed to read archive entries: {}", e))?
    {
        let mut entry = entry.map_err(|e| format!("Failed to read archive entry: {}", e))?;

        let path = entry
            .path()
            .map_err(|e| format!("Failed to get entry path: {}", e))?;

        if path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            tracing::warn!("Skipping entry with path traversal: {:?}", path);
            continue;
        }

        let outpath = dest.join(&path);

        if entry.header().entry_type().is_dir() {
            fs::create_dir_all(&outpath)
                .map_err(|e| format!("Failed to create directory {:?}: {}", outpath, e))?;
        } else {
            if let Some(parent) = outpath.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("Failed to create parent directory: {}", e))?;
                }
            }

            let mut outfile = fs::File::create(&outpath)
                .map_err(|e| format!("Failed to create file {:?}: {}", outpath, e))?;

            io::copy(&mut entry, &mut outfile)
                .map_err(|e| format!("Failed to extract file {:?}: {}", outpath, e))?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(mode) = entry.header().mode() {
                    fs::set_permissions(&outpath, fs::Permissions::from_mode(mode)).ok();
                }
            }
        }
    }

    tracing::info!("Archive extracted successfully");
    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn extract_tar_zst(_data: &[u8], _dest: &PathBuf) -> Result<(), String> {
    Err("Single player extraction is not supported on this platform".to_string())
}

/// Check the current single player installation status
#[tauri::command]
pub async fn get_singleplayer_status(_app: AppHandle) -> Result<SinglePlayerStatus, String> {
    let base_dir = get_singleplayer_base_dir()?;

    if !base_dir.exists() {
        return Ok(SinglePlayerStatus {
            installed: false,
            version: None,
            release_tag: None,
            path: None,
        });
    }

    let version = read_installed_version();
    let installed = version.is_some();

    Ok(SinglePlayerStatus {
        installed,
        version: version.clone(),
        release_tag: version,
        path: if installed {
            Some(base_dir.to_string_lossy().to_string())
        } else {
            None
        },
    })
}

/// Get the latest available release info from GitHub
#[tauri::command]
pub async fn get_latest_singleplayer_release(_app: AppHandle) -> Result<ReleaseInfo, String> {
    fetch_latest_release().await
}

/// Install or update the single player game files
#[tauri::command]
pub async fn install_singleplayer(_app: AppHandle) -> Result<SinglePlayerStatus, String> {
    tracing::info!("Starting single player installation");

    let (_, build_asset_name) = get_singleplayer_config()?;
    let release = fetch_latest_release().await?;

    let download_url = release.download_url.ok_or_else(|| {
        format!(
            "Release {} does not contain {}",
            release.tag_name, build_asset_name
        )
    })?;

    if let Some(installed_version) = read_installed_version() {
        if installed_version == release.tag_name {
            tracing::info!(
                "Single player version {} already installed",
                release.tag_name
            );
            let base_dir = get_singleplayer_base_dir()?;
            return Ok(SinglePlayerStatus {
                installed: true,
                version: Some(installed_version.clone()),
                release_tag: Some(installed_version),
                path: Some(base_dir.to_string_lossy().to_string()),
            });
        }
    }

    let base_dir = get_singleplayer_base_dir()?;

    if base_dir.exists() {
        tracing::info!("Removing existing installation at {:?}", base_dir);
        fs::remove_dir_all(&base_dir)
            .map_err(|e| format!("Failed to remove existing installation: {}", e))?;
    }

    tracing::info!("Downloading single player build {}", release.tag_name);
    let data = download_file(&download_url).await?;

    tracing::info!("Extracting single player build");
    extract_tar_zst(&data, &base_dir)?;

    write_installed_version(&release.tag_name)?;

    tracing::info!("Single player {} installed successfully", release.tag_name);

    Ok(SinglePlayerStatus {
        installed: true,
        version: Some(release.tag_name.clone()),
        release_tag: Some(release.tag_name),
        path: Some(base_dir.to_string_lossy().to_string()),
    })
}

/// Delete the single player installation
#[tauri::command]
pub async fn delete_singleplayer(_app: AppHandle) -> Result<bool, String> {
    let base_dir = get_singleplayer_base_dir()?;

    if base_dir.exists() {
        tracing::info!("Deleting single player installation at {:?}", base_dir);
        fs::remove_dir_all(&base_dir)
            .map_err(|e| format!("Failed to delete single player installation: {}", e))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Parse the BYOND version from dependencies.sh
fn get_byond_version_from_dependencies() -> Result<String, String> {
    let base_dir = get_singleplayer_base_dir()?;
    let deps_path = base_dir.join("dependencies.sh");

    if !deps_path.exists() {
        return Err("dependencies.sh not found in singleplayer installation".to_string());
    }

    let contents = fs::read_to_string(&deps_path)
        .map_err(|e| format!("Failed to read dependencies.sh: {}", e))?;

    let mut major: Option<&str> = None;
    let mut minor: Option<&str> = None;

    for line in contents.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("export BYOND_MAJOR=") {
            major = Some(value);
        } else if let Some(value) = line.strip_prefix("export BYOND_MINOR=") {
            minor = Some(value);
        }
    }

    match (major, minor) {
        (Some(maj), Some(min)) => Ok(format!("{}.{}", maj, min)),
        _ => Err("Could not parse BYOND version from dependencies.sh".to_string()),
    }
}

/// Find the .dmb file in the singleplayer directory
fn find_dmb_file() -> Result<PathBuf, String> {
    let base_dir = get_singleplayer_base_dir()?;

    if !base_dir.exists() {
        return Err("Single player not installed".to_string());
    }

    // Try configured DMB name first
    if let Some(dmb_name) = crate::config::get_config().singleplayer.dmb_name {
        let dmb_path = base_dir.join(dmb_name);
        if dmb_path.exists() {
            return Ok(dmb_path);
        }
    }

    // Fall back to searching for any .dmb file
    for entry in fs::read_dir(&base_dir)
        .map_err(|e| format!("Failed to read singleplayer directory: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        if path.extension().map(|e| e == "dmb").unwrap_or(false) {
            return Ok(path);
        }
    }

    Err("No .dmb file found in singleplayer installation".to_string())
}

/// Launch the single player game
#[tauri::command]
pub async fn launch_singleplayer(app: AppHandle) -> Result<(), String> {
    let byond_version = get_byond_version_from_dependencies()?;
    tracing::info!("Launching singleplayer with BYOND {}", byond_version);

    let version_info = install_byond_version(app.clone(), byond_version.clone()).await?;

    if !version_info.installed {
        return Err(format!("Failed to install BYOND version {}", byond_version));
    }

    let dreamseeker_path = version_info.path.ok_or("DreamSeeker path not found")?;

    let dmb_path = find_dmb_file()?;

    tracing::info!(
        "Launching DreamSeeker: {} -trusted {}",
        dreamseeker_path,
        dmb_path.display()
    );

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        // Set a unique WebView2 user data folder to avoid conflicts with the system BYOND pager.
        // When the BYOND pager is running, it locks the default WebView2 user data directory,
        // preventing our DreamSeeker from using WebView2. Using a separate folder resolves this.
        let webview2_data_dir = get_byond_base_dir(&app)?.join("webview2_data");

        let child = Command::new(&dreamseeker_path)
            .arg("-trusted")
            .arg(&dmb_path)
            .env("WEBVIEW2_USER_DATA_FOLDER", &webview2_data_dir)
            .spawn()
            .map_err(|e| format!("Failed to launch DreamSeeker: {}", e))?;

        app.emit("game-connected", "Sandbox").ok();
        if let Some(manager) = app.try_state::<Arc<PresenceManager>>() {
            manager.start_game_session("Sandbox".to_string(), None, child);
        }
    }

    #[cfg(target_os = "linux")]
    {
        use crate::wine;

        let status = wine::check_prefix_status(&app).await;
        if !status.prefix_initialized || !status.webview2_installed {
            return Err(
                "Wine environment not fully configured. Please complete setup first.".to_string(),
            );
        }

        // Convert Linux path to Wine path (Z:\...)
        let dmb_path_str = dmb_path.to_str().unwrap_or("");
        let wine_dmb_path = format!("Z:{}", dmb_path_str.replace('/', "\\"));

        let webview2_data_dir = get_byond_base_dir(&app)?.join("webview2_data");

        let child = wine::launch_with_wine(
            &app,
            std::path::Path::new(&dreamseeker_path),
            &["-trusted", &wine_dmb_path],
            &[(
                "WEBVIEW2_USER_DATA_FOLDER",
                webview2_data_dir.to_str().unwrap(),
            )],
        )
        .map_err(|e| format!("Failed to launch DreamSeeker via Wine: {}", e))?;

        app.emit("game-connected", "Sandbox").ok();
        if let Some(manager) = app.try_state::<Arc<PresenceManager>>() {
            manager.start_game_session("Sandbox".to_string(), None, child);
        }
    }

    #[cfg(target_os = "macos")]
    {
        let _ = (dreamseeker_path, dmb_path);
        Err("BYOND is not supported on macOS".to_string())
    }

    #[cfg(not(target_os = "macos"))]
    Ok(())
}
