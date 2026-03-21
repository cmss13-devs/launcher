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

/// Get user agent string for BYOND webviews.
/// BYOND requires a Windows user agent to show the .join_link element.
fn get_user_agent() -> String {
    let config = crate::config::get_config();
    let version = env!("CARGO_PKG_VERSION");
    format!("{}/{} (Windows)", config.product_name, version)
}

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

            function isCloudflareChallenge() {
                const title = document.title || '';
                return title.toLowerCase().includes('just a moment');
            }

            function checkLogin() {
                // Wait for Cloudflare challenge to complete
                if (isCloudflareChallenge()) {
                    console.log('[BYOND Login] Cloudflare challenge in progress, waiting...');
                    setTimeout(checkLogin, CHECK_INTERVAL);
                    return;
                }

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
            const CHECK_INTERVAL = 500;
            const MAX_RETRIES = 60; // 30 seconds max
            let retries = 0;

            function isCloudflareChallenge() {
                const title = document.title || '';
                return title.toLowerCase().includes('just a moment');
            }

            function extractWebId() {
                // Wait for Cloudflare challenge to complete
                if (isCloudflareChallenge()) {
                    console.log('[BYOND WebID] Cloudflare challenge in progress, waiting...');
                    if (retries++ < MAX_RETRIES) {
                        setTimeout(extractWebId, CHECK_INTERVAL);
                    } else {
                        console.log('[BYOND WebID] Timeout waiting for Cloudflare');
                        window.__TAURI_INTERNALS__.invoke('byond_webid_complete', { webId: null });
                    }
                    return;
                }

                const joinLink = document.querySelector('.join_link');
                if (!joinLink) {
                    // Element might not be loaded yet, retry a few times
                    if (retries++ < MAX_RETRIES) {
                        console.log('[BYOND WebID] No .join_link found, retrying...');
                        setTimeout(extractWebId, CHECK_INTERVAL);
                        return;
                    }
                    console.log('[BYOND WebID] No .join_link found after retries');
                    window.__TAURI_INTERNALS__.invoke('byond_webid_complete', { webId: null });
                    return;
                }

                const href = joinLink.getAttribute('href') || '';
                console.log('[BYOND WebID] Full href:', href);

                const match = href.match(/webid=([a-fA-F0-9]+)/);
                const webId = match ? match[1] : null;

                console.log('[BYOND WebID] Extracted webId:', webId);
                console.log('[BYOND WebID] webId length:', webId ? webId.length : 0);
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
    .user_agent(&get_user_agent())
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByondSessionCheck {
    pub logged_in: bool,
    pub username: Option<String>,
    pub web_id: Option<String>,
}

pub struct SessionCheckState {
    result_tx: Arc<Mutex<Option<oneshot::Sender<ByondSessionCheck>>>>,
}

impl SessionCheckState {
    pub fn new() -> Self {
        Self {
            result_tx: Arc::new(Mutex::new(None)),
        }
    }

    pub fn complete(&self, result: ByondSessionCheck) {
        let tx: Option<oneshot::Sender<ByondSessionCheck>> = self.result_tx.lock().take();
        if let Some(tx) = tx {
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
        "BYOND session check complete: web_id={:?}, username={:?}",
        web_id,
        username
    );

    let is_guest = web_id.as_ref().map(|id| id == "guest").unwrap_or(true);
    let logged_in = !is_guest && web_id.is_some();

    if logged_in {
        if let (Some(uname), Some(_wid)) = (&username, &web_id) {
            if let Some(session_state) = app.try_state::<ByondSessionState>() {
                session_state.set_session(uname.clone(), "cookie_session".to_string());
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

    let check_state = SessionCheckState::new();
    *check_state.result_tx.lock() = Some(tx);
    app.manage(check_state);

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
                    console.log('[BYOND Session] Cloudflare challenge in progress, waiting...');
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
                        console.log('[BYOND Session] No .join_link found, retrying...');
                        setTimeout(checkSession, CHECK_INTERVAL);
                        return;
                    }
                    window.__TAURI_INTERNALS__.invoke('byond_session_check_complete', { webId: null, username: null });
                    return;
                }

                const href = joinLink.getAttribute('href') || '';
                const match = href.match(/webid=([a-fA-F0-9]+|guest)/i);
                const webId = match ? match[1] : null;

                // Extract username from topbar
                const nameLink = document.querySelector('.topbar_name_link');
                let username = null;
                if (nameLink) {
                    const text = nameLink.textContent.trim();
                    username = text.split('\n')[0].trim();
                }

                console.log('[BYOND Session] webId:', webId, 'username:', username);
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

    let _window = WebviewWindowBuilder::new(
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
    .build()
    .map_err(|e| format!("Failed to create webview: {}", e))?;

    match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
        Ok(Ok(result)) => {
            tracing::info!("BYOND session check completed");
            Ok(result)
        }
        Ok(Err(_)) => {
            tracing::warn!("BYOND session check channel closed");
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
