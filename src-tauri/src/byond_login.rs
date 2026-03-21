//! BYOND website login via webview for web-based authentication.
//!
//! This module handles logging into BYOND's website through a webview,
//! storing the username and using persistent cookies to fetch the user's
//! web_id for automatic BYOND authentication.

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{
    webview::PageLoadEvent, AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder,
};
use tokio::sync::oneshot;

/// Get user agent string for BYOND webviews.
/// BYOND requires a Windows user agent to show the .join_link element.
fn get_user_agent() -> String {
    let config = crate::config::get_config();
    let version = env!("CARGO_PKG_VERSION");
    format!("{}/{} (Windows)", config.product_name, version)
}

/// In-memory storage for BYOND session username
#[derive(Debug, Clone, Default)]
pub struct ByondSession {
    pub username: Option<String>,
}

/// Thread-safe BYOND session state
pub struct ByondSessionState {
    session: Mutex<ByondSession>,
}

impl ByondSessionState {
    pub fn new() -> Self {
        Self {
            session: Mutex::new(ByondSession::default()),
        }
    }

    pub fn set_username(&self, username: String) {
        let mut session = self.session.lock();
        session.username = Some(username);
    }

    pub fn get_username(&self) -> Option<String> {
        self.session.lock().username.clone()
    }

    pub fn clear_session(&self) {
        let mut session = self.session.lock();
        *session = ByondSession::default();
    }
}

/// Result from BYOND login - just the username
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ByondLoginResult {
    pub username: Option<String>,
}

/// State for managing the login flow
pub struct ByondLoginState {
    result_tx: Mutex<Option<oneshot::Sender<Result<ByondLoginResult, String>>>>,
}

impl ByondLoginState {
    pub fn new() -> Self {
        Self {
            result_tx: Mutex::new(None),
        }
    }

    pub fn set_sender(&self, tx: oneshot::Sender<Result<ByondLoginResult, String>>) {
        *self.result_tx.lock() = Some(tx);
    }

    pub fn complete(&self, result: Result<ByondLoginResult, String>) {
        if let Some(tx) = self.result_tx.lock().take() {
            let _ = tx.send(result);
        }
    }
}

/// Called from the login webview's JS when login is complete
#[tauri::command]
pub fn byond_login_complete(app: AppHandle, username: Option<String>) {
    tracing::info!("BYOND login complete - username: {:?}", username);

    if let Some(ref name) = username {
        if let Some(session_state) = app.try_state::<ByondSessionState>() {
            session_state.set_username(name.clone());
            tracing::info!("BYOND session stored for user: {}", name);
        }
    }

    let _ = app.emit("byond-session-changed", username.clone());

    if let Some(state) = app.try_state::<ByondLoginState>() {
        state.complete(Ok(ByondLoginResult { username }));
    }

    if let Some(window) = app.get_webview_window("byond_login") {
        let _ = window.close();
    }
}

/// Get current BYOND session status
#[tauri::command]
pub fn get_byond_session_status(app: AppHandle) -> Option<String> {
    app.try_state::<ByondSessionState>()
        .and_then(|state| state.get_username())
}

/// Clear BYOND session
#[tauri::command]
pub fn clear_byond_session(app: AppHandle) {
    if let Some(state) = app.try_state::<ByondSessionState>() {
        state.clear_session();
        tracing::info!("BYOND session cleared");
    }
}

/// Log out from BYOND web session
#[tauri::command]
pub async fn logout_byond_web(app: AppHandle) -> Result<(), String> {
    tracing::info!("Logging out from BYOND web session");

    if let Some(state) = app.try_state::<ByondSessionState>() {
        state.clear_session();
    }

    // Delete the webview data folder to clear cookies
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("byond_webview");

    if data_dir.exists() {
        std::fs::remove_dir_all(&data_dir)
            .map_err(|e| format!("Failed to delete webview data: {}", e))?;
        tracing::info!("Deleted BYOND webview data at {:?}", data_dir);
    }

    let _ = app.emit("byond-session-changed", None::<String>);

    tracing::info!("BYOND web logout complete");
    Ok(())
}

/// Open BYOND login window and wait for user to authenticate
#[tauri::command]
pub async fn start_byond_login(app: AppHandle) -> Result<ByondLoginResult, String> {
    // Check if login window already exists
    if app.get_webview_window("byond_login").is_some() {
        return Err("Login already in progress".to_string());
    }

    tracing::info!("Starting BYOND web login flow");

    let (tx, rx) = oneshot::channel();

    // Set up state to receive the result (get existing or create new)
    if let Some(state) = app.try_state::<ByondLoginState>() {
        state.set_sender(tx);
    } else {
        let login_state = ByondLoginState::new();
        login_state.set_sender(tx);
        app.manage(login_state);
    }

    let init_script = r#"
        if (window.location.hostname === 'secure.byond.com' || window.location.hostname === 'www.byond.com' || window.location.hostname === 'byond.com') {
            const CHECK_INTERVAL = 500;

            function extractUsername() {
                const nameLink = document.querySelector('.topbar_name_link');
                if (nameLink) {
                    const text = nameLink.textContent.trim();
                    return text.split('\n')[0].trim();
                }
                return null;
            }

            function isCloudflareChallenge() {
                const title = document.title || '';
                return title.toLowerCase().includes('just a moment');
            }

            function checkLogin() {
                if (isCloudflareChallenge()) {
                    setTimeout(checkLogin, CHECK_INTERVAL);
                    return;
                }

                const username = extractUsername();
                const path = window.location.pathname.toLowerCase();
                const onLoginPage = path.includes('login');

                if (!onLoginPage && username) {
                    window.__TAURI_INTERNALS__.invoke('byond_login_complete', { username });
                    return;
                }

                setTimeout(checkLogin, CHECK_INTERVAL);
            }

            if (document.readyState === 'complete' || document.readyState === 'interactive') {
                setTimeout(checkLogin, 1000);
            } else {
                window.addEventListener('DOMContentLoaded', () => setTimeout(checkLogin, 1000));
            }
        }
    "#;

    let app_for_close = app.clone();
    let app_for_nav = app.clone();

    // Persistent data directory for cookies
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("byond_webview");

    let window = WebviewWindowBuilder::new(
        &app,
        "byond_login",
        WebviewUrl::External("https://secure.byond.com/login.cgi".parse().unwrap()),
    )
    .title("BYOND Login")
    .inner_size(990.0, 480.0)
    .center()
    .user_agent(&get_user_agent())
    .data_directory(data_dir)
    .initialization_script(init_script)
    .on_page_load(move |_webview, payload| {
        if payload.event() == PageLoadEvent::Finished {
            tracing::debug!("BYOND login page loaded: {}", payload.url());
        }
    })
    .on_navigation(move |url| {
        let path = url.path().to_lowercase();
        if !path.contains("login") {
            tracing::debug!("BYOND login: navigating away from login page to {}", url);
            if let Some(window) = app_for_nav.get_webview_window("byond_login") {
                let _ = window.hide();
            }
        }
        true
    })
    .build()
    .map_err(|e| format!("Failed to create login window: {}", e))?;

    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { .. } = event {
            tracing::info!("BYOND login window closed by user");
            if let Some(state) = app_for_close.try_state::<ByondLoginState>() {
                state.complete(Err("Login cancelled".to_string()));
            }
        }
    });

    // Wait for result with 5 minute timeout
    match tokio::time::timeout(std::time::Duration::from_secs(300), rx).await {
        Ok(Ok(result)) => {
            tracing::info!("BYOND login flow completed successfully",);
            result
        }
        Ok(Err(_)) => {
            tracing::warn!("BYOND login channel closed unexpectedly");
            Err("Login channel closed".to_string())
        }
        Err(_) => {
            tracing::warn!("BYOND login timed out after 5 minutes");
            if let Some(w) = app.get_webview_window("byond_login") {
                let _ = w.close();
            }
            Err("Login timed out".to_string())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByondSessionCheck {
    pub logged_in: bool,
    pub username: Option<String>,
    pub web_id: Option<String>,
}

pub struct SessionCheckState {
    result_tx: Mutex<Option<oneshot::Sender<ByondSessionCheck>>>,
}

impl SessionCheckState {
    pub fn new() -> Self {
        Self {
            result_tx: Mutex::new(None),
        }
    }

    pub fn set_sender(&self, tx: oneshot::Sender<ByondSessionCheck>) {
        *self.result_tx.lock() = Some(tx);
    }

    pub fn complete(&self, result: ByondSessionCheck) {
        if let Some(tx) = self.result_tx.lock().take() {
            let _ = tx.send(result);
        }
    }
}

/// Called from JS when session check is complete
#[tauri::command]
pub fn byond_session_check_complete(
    app: AppHandle,
    web_id: Option<String>,
    username: Option<String>,
) {
    tracing::info!(
        "BYOND session check complete: has_web_id={}, username={:?}",
        web_id.is_some(),
        username
    );

    let is_guest = web_id.as_ref().map(|id| id == "guest").unwrap_or(true);
    let logged_in = !is_guest && web_id.is_some();

    if logged_in {
        if let Some(uname) = &username {
            if let Some(session_state) = app.try_state::<ByondSessionState>() {
                session_state.set_username(uname.clone());
            }
        }
    }

    if let Some(state) = app.try_state::<SessionCheckState>() {
        state.complete(ByondSessionCheck {
            logged_in,
            username: if logged_in { username } else { None },
            web_id: if logged_in { web_id } else { None },
        });
    }

    if let Some(window) = app.get_webview_window("byond_session_check") {
        let _ = window.close();
    }
}

#[tauri::command]
pub async fn check_byond_web_session(app: AppHandle) -> Result<ByondSessionCheck, String> {
    if app.get_webview_window("byond_session_check").is_some() {
        return Err("Session check already in progress".to_string());
    }

    tracing::info!("Checking BYOND web session");

    let (tx, rx) = oneshot::channel();

    if let Some(state) = app.try_state::<SessionCheckState>() {
        state.set_sender(tx);
    } else {
        let check_state = SessionCheckState::new();
        check_state.set_sender(tx);
        app.manage(check_state);
    }

    let init_script = r#"
        if (window.location.hostname === 'www.byond.com' || window.location.hostname === 'byond.com') {
            const CHECK_INTERVAL = 500;
            const MAX_RETRIES = 60;
            let retries = 0;

            function isCloudflareChallenge() {
                const title = document.title || '';
                return title.toLowerCase().includes('just a moment');
            }

            function checkSession() {
                if (isCloudflareChallenge()) {
                    if (retries++ < MAX_RETRIES) {
                        setTimeout(checkSession, CHECK_INTERVAL);
                    } else {
                        window.__TAURI_INTERNALS__.invoke('byond_session_check_complete', { webId: null, username: null });
                    }
                    return;
                }

                const joinLink = document.querySelector('.join_link');
                if (!joinLink) {
                    if (retries++ < MAX_RETRIES) {
                        setTimeout(checkSession, CHECK_INTERVAL);
                        return;
                    }
                    window.__TAURI_INTERNALS__.invoke('byond_session_check_complete', { webId: null, username: null });
                    return;
                }

                const href = joinLink.getAttribute('href') || '';
                const match = href.match(/webid=([a-fA-F0-9]+|guest)/i);
                const webId = match ? match[1] : null;

                const nameLink = document.querySelector('.topbar_name_link');
                let username = null;
                if (nameLink) {
                    const text = nameLink.textContent.trim();
                    username = text.split('\n')[0].trim();
                }

                window.__TAURI_INTERNALS__.invoke('byond_session_check_complete', { webId, username });
            }

            if (document.readyState === 'complete') {
                checkSession();
            } else {
                window.addEventListener('load', checkSession);
            }
        }
    "#;

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("byond_webview");

    let url = "https://www.byond.com/games/Exadv1.SpaceStation13";

    tracing::debug!("Creating session check webview (visible=false)");
    tracing::debug!("Data directory: {:?}", data_dir);

    let app_for_events = app.clone();

    let window = WebviewWindowBuilder::new(
        &app,
        "byond_session_check",
        WebviewUrl::External(url.parse().unwrap()),
    )
    .title("Checking BYOND Session...")
    .inner_size(400.0, 300.0)
    .visible(false)
    .user_agent(&get_user_agent())
    .data_directory(data_dir)
    .initialization_script(init_script)
    .on_page_load(move |_webview, payload| {
        tracing::debug!(
            "Session check page load event: {:?} - {}",
            payload.event(),
            payload.url()
        );
        if payload.event() == PageLoadEvent::Finished {
            tracing::info!("Session check page finished loading: {}", payload.url());
        }
    })
    .build()
    .map_err(|e| format!("Failed to create webview: {}", e))?;

    tracing::info!("Session check webview created successfully");

    // Monitor window events
    window.on_window_event(move |event| match event {
        tauri::WindowEvent::CloseRequested { .. } => {
            tracing::warn!("Session check window close requested");
        }
        tauri::WindowEvent::Destroyed => {
            tracing::warn!("Session check window destroyed");

            if let Some(state) = app_for_events.try_state::<SessionCheckState>() {
                state.complete(ByondSessionCheck {
                    logged_in: false,
                    username: None,
                    web_id: None,
                });
            }
        }
        _ => {}
    });

    tracing::debug!("Waiting for session check result (30s timeout)");

    match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
        Ok(Ok(result)) => {
            tracing::info!("BYOND session check completed");
            Ok(result)
        }
        Ok(Err(_)) => {
            let window_exists = app.get_webview_window("byond_session_check").is_some();
            tracing::warn!(
                "BYOND session check channel closed (window still exists: {})",
                window_exists
            );
            if let Some(w) = app.get_webview_window("byond_session_check") {
                let _ = w.close();
            }
            Err("Session check channel closed".to_string())
        }
        Err(_) => {
            tracing::warn!("BYOND session check timed out");
            if let Some(w) = app.get_webview_window("byond_session_check") {
                let _ = w.close();
            }
            Err("Session check timed out".to_string())
        }
    }
}
