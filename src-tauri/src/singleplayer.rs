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
use crate::error::{CommandError, CommandResult};
#[cfg(any(target_os = "windows", target_os = "linux"))]
use crate::presence::PresenceManager;

const SINGLEPLAYER_DIR: &str = "singleplayer";
const VERSION_FILE: &str = ".version";

fn get_singleplayer_config() -> CommandResult<(String, String)> {
    let config = crate::config::get_config();
    let repo = config
        .singleplayer
        .github_repo
        .ok_or_else(|| CommandError::NotConfigured {
            feature: "singleplayer".to_string(),
        })?;
    let asset =
        config
            .singleplayer
            .build_asset_name
            .ok_or_else(|| CommandError::NotConfigured {
                feature: "singleplayer".to_string(),
            })?;
    Ok((repo.to_string(), asset.to_string()))
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
pub struct SinglePlayerStatus {
    pub installed: bool,
    pub version: Option<String>,
    pub release_tag: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub name: String,
    pub published_at: String,
    pub download_url: Option<String>,
    #[specta(type = f64)]
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

fn get_singleplayer_base_dir() -> CommandResult<PathBuf> {
    let config = crate::config::get_config();
    let local_data = dirs::data_local_dir()
        .ok_or_else(|| CommandError::Io("local data directory unavailable".to_string()))?
        .join(config.app_identifier);

    Ok(local_data.join(SINGLEPLAYER_DIR))
}

fn get_version_file_path() -> CommandResult<PathBuf> {
    Ok(get_singleplayer_base_dir()?.join(VERSION_FILE))
}

fn read_installed_version() -> Option<String> {
    let version_path = get_version_file_path().ok()?;
    fs::read_to_string(version_path).ok()
}

fn write_installed_version(version: &str) -> CommandResult<()> {
    let version_path = get_version_file_path()?;
    fs::write(&version_path, version)?;
    Ok(())
}

/// Fetch the latest release info from GitHub
async fn fetch_latest_release() -> CommandResult<ReleaseInfo> {
    let (github_repo, build_asset_name) = get_singleplayer_config()?;

    let url = format!("https://api.github.com/repos/{github_repo}/releases/latest");

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("User-Agent", "CM-Launcher")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(CommandError::InvalidResponse(format!(
            "GitHub API returned HTTP {}",
            response.status()
        )));
    }

    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|e| CommandError::InvalidResponse(format!("Failed to parse release info: {e}")))?;

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
async fn download_file(url: &str) -> CommandResult<Vec<u8>> {
    tracing::info!("Downloading from {}", url);

    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("User-Agent", "CM-Launcher")
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(CommandError::InvalidResponse(format!(
            "Download failed with HTTP {}",
            response.status()
        )));
    }

    let bytes = response.bytes().await?;

    Ok(bytes.to_vec())
}

/// Extract a tar.zst archive to a directory
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
fn extract_tar_zst(data: &[u8], dest: &PathBuf) -> CommandResult<()> {
    tracing::info!("Extracting archive to {:?}", dest);

    fs::create_dir_all(dest)?;

    let cursor = io::Cursor::new(data);
    let zstd_decoder = zstd::stream::Decoder::new(cursor)?;

    let mut archive = tar::Archive::new(zstd_decoder);
    archive.set_preserve_permissions(true);

    for entry in archive.entries()? {
        let mut entry = entry?;

        let path = entry.path()?;

        if path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            tracing::warn!("Skipping entry with path traversal: {:?}", path);
            continue;
        }

        let outpath = dest.join(&path);

        if entry.header().entry_type().is_dir() {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }

            let mut outfile = fs::File::create(&outpath)?;

            io::copy(&mut entry, &mut outfile)?;

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
fn extract_tar_zst(_data: &[u8], _dest: &PathBuf) -> CommandResult<()> {
    Err(CommandError::UnsupportedPlatform {
        feature: "singleplayer extraction".into(),
        platform: std::env::consts::OS.into(),
    })
}

/// Check the current single player installation status
#[tauri::command]
#[specta::specta]
pub async fn get_singleplayer_status(_app: AppHandle) -> CommandResult<SinglePlayerStatus> {
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
#[specta::specta]
pub async fn get_latest_singleplayer_release(_app: AppHandle) -> CommandResult<ReleaseInfo> {
    fetch_latest_release().await
}

/// Install or update the single player game files
#[tauri::command]
#[specta::specta]
pub async fn install_singleplayer(_app: AppHandle) -> CommandResult<SinglePlayerStatus> {
    tracing::info!("Starting single player installation");

    let (_, build_asset_name) = get_singleplayer_config()?;
    let release = fetch_latest_release().await?;

    let download_url = release.download_url.ok_or_else(|| {
        CommandError::NotFound(format!(
            "Release {} does not contain {}",
            release.tag_name, build_asset_name
        ))
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
        fs::remove_dir_all(&base_dir)?;
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
#[specta::specta]
pub async fn delete_singleplayer(_app: AppHandle) -> CommandResult<bool> {
    let base_dir = get_singleplayer_base_dir()?;

    if base_dir.exists() {
        tracing::info!("Deleting single player installation at {:?}", base_dir);
        fs::remove_dir_all(&base_dir)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Parse the BYOND version from dependencies.sh
fn get_byond_version_from_dependencies() -> CommandResult<String> {
    let base_dir = get_singleplayer_base_dir()?;
    let deps_path = base_dir.join("dependencies.sh");

    if !deps_path.exists() {
        return Err(CommandError::NotFound(
            "dependencies.sh in singleplayer installation".to_string(),
        ));
    }

    let contents = fs::read_to_string(&deps_path)?;

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
        (Some(maj), Some(min)) => Ok(format!("{maj}.{min}")),
        _ => Err(CommandError::InvalidResponse(
            "Could not parse BYOND version from dependencies.sh".to_string(),
        )),
    }
}

/// Find the .dmb file in the singleplayer directory
fn find_dmb_file() -> CommandResult<PathBuf> {
    let base_dir = get_singleplayer_base_dir()?;

    if !base_dir.exists() {
        return Err(CommandError::NotFound(
            "singleplayer installation".to_string(),
        ));
    }

    // Try configured DMB name first
    if let Some(dmb_name) = crate::config::get_config().singleplayer.dmb_name {
        let dmb_path = base_dir.join(dmb_name);
        if dmb_path.exists() {
            return Ok(dmb_path);
        }
    }

    // Fall back to searching for any .dmb file
    for entry in fs::read_dir(&base_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "dmb") {
            return Ok(path);
        }
    }

    Err(CommandError::NotFound(
        ".dmb file in singleplayer installation".to_string(),
    ))
}

/// Launch the single player game
#[tauri::command]
#[specta::specta]
pub async fn launch_singleplayer(app: AppHandle) -> CommandResult<()> {
    let byond_version = get_byond_version_from_dependencies()?;
    tracing::info!("Launching singleplayer with BYOND {}", byond_version);

    let version_info = install_byond_version(app.clone(), byond_version.clone()).await?;

    if !version_info.installed {
        return Err(CommandError::Internal(format!(
            "Failed to install BYOND version {byond_version}"
        )));
    }

    let dreamseeker_path = version_info
        .path
        .ok_or_else(|| CommandError::NotFound("dreamseeker executable".into()))?;

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
            .spawn()?;

        app.emit("game-connected", "Sandbox").ok();
        if let Some(manager) = app.try_state::<Arc<PresenceManager>>() {
            manager.start_game_session("Sandbox".to_string(), None, 1, child);
        }
    }

    #[cfg(target_os = "linux")]
    {
        use crate::wine;

        let status = wine::check_prefix_status(&app).await;
        if !status.prefix_initialized || !status.webview2_installed {
            return Err(CommandError::NotConfigured {
                feature: "wine_prefix".into(),
            });
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
        .map_err(|e| CommandError::Io(format!("Failed to launch DreamSeeker via Wine: {e}")))?;

        app.emit("game-connected", "Sandbox").ok();
        if let Some(manager) = app.try_state::<Arc<PresenceManager>>() {
            manager.start_game_session("Sandbox".to_string(), None, 1, child);
        }
    }

    #[cfg(target_os = "macos")]
    {
        let _ = (dreamseeker_path, dmb_path);
        Err(CommandError::UnsupportedPlatform {
            feature: "byond".into(),
            platform: "macos".into(),
        })
    }

    #[cfg(not(target_os = "macos"))]
    Ok(())
}
