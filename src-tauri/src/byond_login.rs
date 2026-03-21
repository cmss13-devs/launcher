//! BYOND website login via webview for cookie-based authentication.
//!
//! This module handles logging into BYOND's website through a webview,
//! capturing the auth cookies which can then be used to fetch the user's
//! web_id for automatic BYOND authentication.

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{webview::PageLoadEvent, AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::oneshot;

/// In-memory storage for BYOND session (not persisted)
#[derive(Debug, Clone, Default)]
pub struct ByondSession {
    pub username: Option<String>,
    pub sbyondcert: Option<String>,
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

    pub fn set_session(&self, username: String, sbyondcert: String) {
        let mut session = self.session.lock();
        session.username = Some(username);
        session.sbyondcert = Some(sbyondcert);
    }

    pub fn get_session(&self) -> ByondSession {
        self.session.lock().clone()
    }

    pub fn clear_session(&self) {
        let mut session = self.session.lock();
        *session = ByondSession::default();
    }

    pub fn is_logged_in(&self) -> bool {
        let session = self.session.lock();
        session.username.is_some() && session.sbyondcert.is_some()
    }
}

/// Cookies and user info extracted from BYOND login
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ByondCookies {
    pub auth: Option<String>,
    pub byondcert: Option<String>,
    pub sbyondcert: Option<String>,
    pub username: Option<String>,
}

impl ByondCookies {
    pub fn is_valid(&self) -> bool {
        self.auth.is_some() || self.byondcert.is_some()
    }
}

/// State for managing the login flow
pub struct ByondLoginState {
    result_tx: Arc<Mutex<Option<oneshot::Sender<Result<ByondCookies, String>>>>>,
}

impl ByondLoginState {
    pub fn new() -> Self {
        Self {
            result_tx: Arc::new(Mutex::new(None)),
        }
    }

    pub fn complete(&self, result: Result<ByondCookies, String>) {
        let tx: Option<oneshot::Sender<Result<ByondCookies, String>>> =
            self.result_tx.lock().take();
        if let Some(tx) = tx {
            let _ = tx.send(result);
        }
    }
}

/// Called from the login webview's JS when cookies are extracted
#[tauri::command]
pub fn byond_login_complete(app: AppHandle, cookies: ByondCookies) {
    tracing::info!(
        "BYOND login complete - username: {:?}, auth: {}, byondcert: {}, sbyondcert: {}",
        cookies.username,
        cookies.auth.is_some(),
        cookies.byondcert.is_some(),
        cookies.sbyondcert.is_some()
    );

    if let Some(cert) = &cookies.sbyondcert {
        tracing::debug!("sbyondcert cookie: {}...", &cert[..cert.len().min(20)]);
    }

    // Store session in memory
    if let (Some(username), Some(sbyondcert)) = (&cookies.username, &cookies.sbyondcert) {
        if let Some(session_state) = app.try_state::<ByondSessionState>() {
            session_state.set_session(username.clone(), sbyondcert.clone());
            tracing::info!("BYOND session stored for user: {}", username);
        }
    }

    if let Some(state) = app.try_state::<ByondLoginState>() {
        state.complete(Ok(cookies));
    }

    if let Some(window) = app.get_webview_window("byond_login") {
        let _ = window.close();
    }
}

/// Get current BYOND session status
#[tauri::command]
pub fn get_byond_session_status(app: AppHandle) -> Option<String> {
    app.try_state::<ByondSessionState>()
        .and_then(|state| state.get_session().username)
}

/// Clear BYOND session
#[tauri::command]
pub fn clear_byond_session(app: AppHandle) {
    if let Some(state) = app.try_state::<ByondSessionState>() {
        state.clear_session();
        tracing::info!("BYOND session cleared");
    }
}

/// Open BYOND login window and wait for user to authenticate
#[tauri::command]
pub async fn start_byond_login(app: AppHandle) -> Result<ByondCookies, String> {
    // Check if login window already exists
    if app.get_webview_window("byond_login").is_some() {
        return Err("Login already in progress".to_string());
    }

    tracing::info!("Starting BYOND web login flow");

    let (tx, rx) = oneshot::channel();

    // Set up state to receive the result
    let login_state = ByondLoginState::new();
    *login_state.result_tx.lock() = Some(tx);
    app.manage(login_state);

    let init_script = r#"
        if (window.location.hostname === 'secure.byond.com' || window.location.hostname === 'www.byond.com' || window.location.hostname === 'byond.com') {
            const CHECK_INTERVAL = 500;

            function extractCookies() {
                const cookies = {};
                document.cookie.split(';').forEach(c => {
                    const parts = c.trim().split('=');
                    const name = parts[0];
                    const value = parts.slice(1).join('=');
                    if (name) cookies[name] = value || '';
                });

                const nameLink = document.querySelector('.topbar_name_link');
                let username = null;
                if (nameLink) {
                    const text = nameLink.textContent.trim();
                    username = text.split('\n')[0].trim();
                }

                return {
                    auth: cookies['auth'] || null,
                    byondcert: cookies['byondcert'] || null,
                    sbyondcert: cookies['sbyondcert'] || null,
                    username: username
                };
            }

            function checkLogin() {
                const cookies = extractCookies();
                const path = window.location.pathname.toLowerCase();
                const onLoginPage = path.includes('login');
                const hasAuth = cookies.auth || cookies.byondcert;

                console.log('[BYOND Login] Checking...', { path, onLoginPage, hasAuth, cookies });

                if (!onLoginPage && hasAuth) {
                    console.log('[BYOND Login] Success! Sending cookies to Tauri');
                    window.__TAURI_INTERNALS__.invoke('byond_login_complete', { cookies });
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
    .inner_size(985.0, 475.0)
    .center()
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

pub struct WebIdFetchState {
    result_tx: Arc<Mutex<Option<oneshot::Sender<Result<String, String>>>>>,
}

impl WebIdFetchState {
    pub fn new() -> Self {
        Self {
            result_tx: Arc::new(Mutex::new(None)),
        }
    }

    pub fn complete(&self, result: Result<String, String>) {
        let tx: Option<oneshot::Sender<Result<String, String>>> = self.result_tx.lock().take();
        if let Some(tx) = tx {
            let _ = tx.send(result);
        }
    }
}

#[tauri::command]
pub fn byond_webid_complete(app: AppHandle, web_id: Option<String>) {
    tracing::info!("BYOND web_id fetch complete: {:?}", web_id);

    if let Some(state) = app.try_state::<WebIdFetchState>() {
        match web_id {
            Some(id) => state.complete(Ok(id)),
            None => state.complete(Err("web_id not found on page".to_string())),
        }
    }

    if let Some(window) = app.get_webview_window("byond_webid_fetch") {
        let _ = window.close();
    }
}

#[tauri::command]
pub async fn fetch_byond_web_id(app: AppHandle) -> Result<String, String> {
    let session = app
        .try_state::<ByondSessionState>()
        .map(|s| s.get_session())
        .unwrap_or_default();

    if session.sbyondcert.is_none() {
        return Err("Not logged in to BYOND. Please login first.".to_string());
    }

    if app.get_webview_window("byond_webid_fetch").is_some() {
        return Err("Web ID fetch already in progress".to_string());
    }

    tracing::info!("Fetching BYOND web_id");

    let (tx, rx) = oneshot::channel();

    let fetch_state = WebIdFetchState::new();
    *fetch_state.result_tx.lock() = Some(tx);
    app.manage(fetch_state);

    let init_script = r#"
        if (window.location.hostname === 'www.byond.com' || window.location.hostname === 'byond.com') {
            function extractWebId() {
                const joinLink = document.querySelector('.join_link');
                if (!joinLink) {
                    console.log('[BYOND WebID] No .join_link found');
                    window.__TAURI_INTERNALS__.invoke('byond_webid_complete', { webId: null });
                    return;
                }

                const href = joinLink.getAttribute('href') || '';

                const match = href.match(/webid=([a-f0-9]+)/i);
                const webId = match ? match[1] : null;

                console.log('[BYOND WebID] href:', href);
                console.log('[BYOND WebID] Extracted:', webId);
                window.__TAURI_INTERNALS__.invoke('byond_webid_complete', { webId });
            }

            if (document.readyState === 'complete') {
                extractWebId();
            } else {
                window.addEventListener('load', extractWebId);
            }
        }
    "#;

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("byond_webview");

    let url = "https://www.byond.com/games/Exadv1.SpaceStation13";

    let _window = WebviewWindowBuilder::new(
        &app,
        "byond_webid_fetch",
        WebviewUrl::External(url.parse().unwrap()),
    )
    .title("Fetching BYOND Web ID...")
    .inner_size(400.0, 300.0)
    .visible(false) // Hidden window
    .data_directory(data_dir)
    .initialization_script(init_script)
    .build()
    .map_err(|e| format!("Failed to create webview: {}", e))?;

    match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
        Ok(Ok(result)) => {
            tracing::info!("BYOND web_id fetch completed");
            result
        }
        Ok(Err(_)) => {
            tracing::warn!("BYOND web_id fetch channel closed");
            if let Some(w) = app.get_webview_window("byond_webid_fetch") {
                let _ = w.close();
            }
            Err("Fetch channel closed".to_string())
        }
        Err(_) => {
            tracing::warn!("BYOND web_id fetch timed out");
            if let Some(w) = app.get_webview_window("byond_webid_fetch") {
                let _ = w.close();
            }
            Err("Fetch timed out".to_string())
        }
    }
}
