use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use tauri::{Emitter, Manager};
use tiny_http::{Response, Server};
use tokio::net::TcpListener as TokioTcpListener;
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message;
use url::Url;

use crate::presence::{ConnectionParams, PresenceManager};

#[allow(clippy::unwrap_used)] // static bytes never fail
fn cors_headers() -> Vec<tiny_http::Header> {
    vec![
        tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap(),
        tiny_http::Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, OPTIONS"[..])
            .unwrap(),
        tiny_http::Header::from_bytes(
            &b"Access-Control-Allow-Headers"[..],
            &b"Content-Type, Launcher-Key"[..],
        )
        .unwrap(),
    ]
}

#[allow(clippy::unwrap_used)] // static bytes never fail
fn json_response(status: u16, body: serde_json::Value) -> Response<std::io::Cursor<Vec<u8>>> {
    let mut response = Response::from_string(body.to_string())
        .with_header(
            tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
        )
        .with_status_code(status);

    for header in cors_headers() {
        response.add_header(header);
    }

    response
}

fn preflight_response() -> Response<std::io::Empty> {
    let mut response = Response::empty(204);
    for header in cors_headers() {
        response.add_header(header);
    }
    response
}

pub struct ControlServer {
    pub port: u16,
    #[allow(dead_code)]
    pub ws_port: u16,

    #[allow(dead_code)]
    pub game_connected: Arc<AtomicBool>,

    /// The current launcher key, rotated per server connection.
    /// Requests must include this in the `Launcher-Key` header.
    launcher_key: Arc<std::sync::Mutex<String>>,

    /// Broadcast channel for sending events to connected WebSocket clients
    #[allow(dead_code)]
    event_tx: broadcast::Sender<String>,
}

fn generate_launcher_key() -> String {
    use rand::distributions::Alphanumeric;
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

impl ControlServer {
    pub fn start(
        app_handle: tauri::AppHandle,
        presence_manager: Arc<PresenceManager>,
    ) -> crate::error::CommandResult<Self> {
        use crate::error::CommandError;

        tracing::info!("Starting control server on 127.0.0.1:0");

        let server = Server::http("127.0.0.1:0").map_err(|e| {
            tracing::error!(
                "Failed to start control server: {} (error type: {:?})",
                e,
                std::any::type_name_of_val(&e)
            );
            tracing::error!(
                "This may be caused by: firewall blocking the connection, \
                antivirus software, or network configuration issues. \
                On Windows, check Windows Firewall settings and any third-party security software."
            );
            CommandError::Io(format!(
                "Failed to start control server: {e}. \
                Please check your firewall and antivirus settings."
            ))
        })?;

        let addr = server.server_addr().to_ip().ok_or_else(|| {
            tracing::error!("Failed to get control server address after binding");
            CommandError::Internal("control server address unavailable after bind".into())
        })?;

        let port = addr.port();
        tracing::info!(
            "Control server started successfully on {}:{} (listening for game connections)",
            addr.ip(),
            port
        );

        let ws_listener = TcpListener::bind("127.0.0.1:0")?;
        let ws_port = ws_listener.local_addr()?.port();
        tracing::info!(
            "WebSocket server started on 127.0.0.1:{} (for launcher events)",
            ws_port
        );

        let (event_tx, _) = broadcast::channel::<String>(32);
        let event_tx_clone = event_tx.clone();

        let game_connected = Arc::new(AtomicBool::new(false));
        let game_connected_clone = Arc::clone(&game_connected);

        let launcher_key = Arc::new(std::sync::Mutex::new(generate_launcher_key()));
        let launcher_key_clone = Arc::clone(&launcher_key);

        thread::spawn(move || {
            Self::run_server(
                server,
                app_handle,
                presence_manager,
                game_connected_clone,
                launcher_key_clone,
            );
        });

        let event_tx_ws = event_tx;
        std::thread::spawn(move || {
            #[allow(clippy::expect_used)] // Runtime creation is required for the server
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime for WebSocket server");

            rt.block_on(async move {
                Self::run_websocket_server(ws_listener, event_tx_ws).await;
            });
        });

        Ok(Self {
            port,
            ws_port,
            game_connected,
            launcher_key,
            event_tx: event_tx_clone,
        })
    }

    /// Generate a new launcher key for a new server connection.
    /// Returns the new key to be passed to the game.
    pub fn rotate_key(&self) -> String {
        let new_key = generate_launcher_key();
        *self.launcher_key.lock().unwrap() = new_key.clone();
        new_key
    }

    /// Get the current launcher key.
    pub fn get_key(&self) -> String {
        self.launcher_key.lock().unwrap().clone()
    }

    /// Send an event to all connected WebSocket clients
    #[allow(dead_code)]
    pub fn broadcast_event(&self, event: &str) {
        if self.event_tx.receiver_count() > 0 {
            if let Err(e) = self.event_tx.send(event.to_string()) {
                tracing::warn!("Failed to broadcast event: {}", e);
            }
        }
    }

    /// Send a JSON event to all connected WebSocket clients
    #[allow(dead_code)]
    pub fn broadcast_json<T: serde::Serialize>(&self, event_type: &str, data: &T) {
        let json = serde_json::json!({
            "type": event_type,
            "data": data,
        });
        self.broadcast_event(&json.to_string());
    }

    async fn run_websocket_server(listener: TcpListener, event_tx: broadcast::Sender<String>) {
        listener.set_nonblocking(true).ok();
        #[allow(clippy::expect_used)] // Listener conversion required for async server
        let listener =
            TokioTcpListener::from_std(listener).expect("Failed to convert TcpListener to tokio");

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    tracing::info!("New WebSocket connection from {}", addr);
                    let event_rx = event_tx.subscribe();

                    tokio::spawn(async move {
                        Self::handle_websocket_connection(stream, event_rx).await;
                    });
                }
                Err(e) => {
                    tracing::error!("Failed to accept WebSocket connection: {}", e);
                }
            }
        }
    }

    async fn handle_websocket_connection(
        stream: tokio::net::TcpStream,
        mut event_rx: broadcast::Receiver<String>,
    ) {
        let ws_stream = match tokio_tungstenite::accept_async(stream).await {
            Ok(ws) => ws,
            Err(e) => {
                tracing::error!("WebSocket handshake failed: {}", e);
                return;
            }
        };

        let (mut write, mut read) = ws_stream.split();

        let config = crate::config::get_config();
        let welcome = serde_json::json!({
            "type": "connected",
            "data": { "message": format!("Connected to {}", config.product_name) }
        });
        if let Err(e) = write.send(Message::Text(welcome.to_string())).await {
            tracing::error!("Failed to send welcome message: {}", e);
            return;
        }

        loop {
            tokio::select! {
                event = event_rx.recv() => {
                    match event {
                        Ok(msg) => {
                            if let Err(e) = write.send(Message::Text(msg)).await {
                                tracing::debug!("WebSocket send error (client disconnected): {}", e);
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("WebSocket client lagged, skipped {} messages", n);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            break;
                        }
                    }
                }
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Ping(data))) => {
                            if let Err(e) = write.send(Message::Pong(data)).await {
                                tracing::debug!("Failed to send pong: {}", e);
                                break;
                            }
                        }
                        Some(Ok(Message::Close(_))) | None => {
                            tracing::info!("WebSocket client disconnected");
                            break;
                        }
                        Some(Ok(Message::Text(text))) => {
                            tracing::debug!("Received WebSocket message: {}", text);
                        }
                        Some(Err(e)) => {
                            tracing::debug!("WebSocket error: {}", e);
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn reset_connected_flag(&self) {
        self.game_connected.store(false, Ordering::SeqCst);
    }

    fn run_server(
        server: Server,
        app_handle: tauri::AppHandle,
        presence_manager: Arc<PresenceManager>,
        game_connected: Arc<AtomicBool>,
        launcher_key: Arc<std::sync::Mutex<String>>,
    ) {
        for request in server.incoming_requests() {
            if request.method() == &tiny_http::Method::Options {
                request.respond(preflight_response()).ok();
                continue;
            }

            // Validate the Launcher-Key header
            let provided_key = request
                .headers()
                .iter()
                .find(|h| h.field.as_str() == "Launcher-Key" || h.field.as_str() == "launcher-key")
                .map(|h| h.value.as_str().to_owned());

            let expected_key = launcher_key.lock().unwrap().clone();
            match provided_key {
                Some(ref key) if key == &expected_key => {}
                _ => {
                    tracing::warn!(
                        "Control server request rejected: invalid or missing Launcher-Key"
                    );
                    let response = json_response(403, serde_json::json!({"error": "Forbidden"}));
                    request.respond(response).ok();
                    continue;
                }
            }

            let full_url = format!("http://127.0.0.1{}", request.url());
            let url = match Url::parse(&full_url) {
                Ok(url) => url,
                Err(e) => {
                    tracing::error!("Failed to parse control server URL: {}", e);
                    let response = json_response(400, serde_json::json!({"error": e.to_string()}));
                    request.respond(response).ok();
                    continue;
                }
            };

            tracing::debug!("Control server received request: {}", url.path());

            if !game_connected.swap(true, Ordering::SeqCst) {
                tracing::info!("Game connected to control server");
                if let Some(session) = presence_manager.get_game_session() {
                    app_handle.emit("game-connected", &session.server_name).ok();
                }
            }

            match url.path() {
                "/restart" => {
                    Self::handle_restart(request, &url, &app_handle, &presence_manager);
                }
                "/get-url" => {
                    Self::handle_get_url(request, &app_handle, &presence_manager);
                }
                "/status" => {
                    Self::handle_status(request, &presence_manager);
                }
                "/auth-ticket" => {
                    Self::handle_auth_ticket(request, &app_handle, &presence_manager);
                }
                _ => {
                    let response = json_response(404, serde_json::json!({"error": "Not found"}));
                    request.respond(response).ok();
                }
            }
        }
    }

    fn handle_restart(
        request: tiny_http::Request,
        url: &Url,
        app_handle: &tauri::AppHandle,
        presence_manager: &Arc<PresenceManager>,
    ) {
        let reason = url
            .query_pairs()
            .find(|(key, _)| key == "reason")
            .map(|(_, value)| value.into_owned());

        tracing::info!("Restart command received with reason: {:?}", reason);

        let connection_params = presence_manager.get_last_connection_params();

        if connection_params.is_none() {
            let response = json_response(
                400,
                serde_json::json!({"error": "No previous connection to restart"}),
            );
            request.respond(response).ok();
            return;
        }

        let Some(params) = connection_params else {
            return;
        };

        if presence_manager.kill_game_process() {
            tracing::info!("Killed existing game process");
        }

        app_handle
            .emit(
                "game-restarting",
                serde_json::json!({
                    "server_name": params.server_name,
                    "reason": reason,
                }),
            )
            .ok();

        let app_handle = app_handle.clone();
        let server_name = params.server_name.clone();

        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            let fresh_params = match refresh_auth_token(&app_handle, params).await {
                Ok(params) => params,
                Err(e) => {
                    tracing::error!("Failed to refresh auth token: {}", e);
                    return;
                }
            };

            let result = crate::byond::connect(
                app_handle,
                crate::byond::ConnectionRequest {
                    version: fresh_params.version,
                    host: fresh_params.host,
                    port: fresh_params.port,
                    access_type: fresh_params.access_type,
                    access_token: fresh_params.access_token,
                    server_name: fresh_params.server_name,
                    map_name: fresh_params.map_name,
                    source: Some("control_server_restart".to_string()),
                    server_id: fresh_params.server_id,
                    players: None,
                },
            )
            .await;

            match result {
                Ok(_) => tracing::info!("Successfully restarted connection to {}", server_name),
                Err(e) => tracing::error!("Failed to restart connection: {}", e),
            }
        });

        let response = json_response(200, serde_json::json!({"status": "restarting"}));
        request.respond(response).ok();
    }

    fn handle_get_url(
        request: tiny_http::Request,
        app_handle: &tauri::AppHandle,
        presence_manager: &Arc<PresenceManager>,
    ) {
        tracing::info!("Get URL request received");

        let Some(params) = presence_manager.get_last_connection_params() else {
            let response = json_response(
                400,
                serde_json::json!({"error": "No previous connection available"}),
            );
            request.respond(response).ok();
            return;
        };

        let result: Result<String, String> = tauri::async_runtime::block_on(async {
            let fresh_params = refresh_auth_token(app_handle, params).await?;

            let control_port = app_handle
                .try_state::<ControlServer>()
                .map(|s| s.port.to_string());
            let launcher_key = app_handle
                .try_state::<ControlServer>()
                .map(|s| s.rotate_key());
            let websocket_port = app_handle
                .try_state::<ControlServer>()
                .map(|s| s.ws_port.to_string());

            let url = crate::byond::build_connect_url(
                &fresh_params.host,
                &fresh_params.port,
                fresh_params.access_type.as_deref(),
                fresh_params.access_token.as_deref(),
                control_port.as_deref(),
                launcher_key.as_deref(),
                websocket_port.as_deref(),
            );

            Ok(url)
        });

        match result {
            Ok(url) => {
                let response = json_response(200, serde_json::json!({"url": url}));
                request.respond(response).ok();
            }
            Err(e) => {
                let response = json_response(500, serde_json::json!({"error": e}));
                request.respond(response).ok();
            }
        }
    }

    fn handle_status(request: tiny_http::Request, presence_manager: &Arc<PresenceManager>) {
        let is_running = presence_manager.check_game_running();
        let session = presence_manager.get_game_session();
        let hwid = generate_hwid();

        let response = json_response(
            200,
            serde_json::json!({
                "running": is_running,
                "server_name": session.as_ref().map(|s| &s.server_name),
                "hwid": hwid,
            }),
        );
        request.respond(response).ok();
    }

    fn handle_auth_ticket(
        request: tiny_http::Request,
        _app_handle: &tauri::AppHandle,
        presence_manager: &Arc<PresenceManager>,
    ) {
        tracing::info!("Auth ticket request received");

        let Some(params) = presence_manager.get_last_connection_params() else {
            let response = json_response(
                400,
                serde_json::json!({"error": "No previous connection available"}),
            );
            request.respond(response).ok();
            return;
        };

        let result: Result<String, String> = tauri::async_runtime::block_on(async {
            let session_token = match crate::auth::TokenStorage::get_tokens() {
                Ok(Some(tokens)) if !crate::auth::TokenStorage::is_expired() => tokens.access_token,
                Ok(_) => return Err("Hub authentication expired or not available".to_string()),
                Err(e) => return Err(format!("Failed to read auth tokens: {e}")),
            };

            let server_id = params.server_id.as_deref().ok_or("Server has no hub ID")?;

            let hwid = generate_hwid();

            crate::auth::hub_client::HubClient::join(&session_token, server_id, hwid.as_deref())
                .await
                .map_err(|e| format!("Failed to get auth ticket: {e}"))
        });

        match result {
            Ok(ticket) => {
                let response = json_response(200, serde_json::json!({"auth_ticket": ticket}));
                request.respond(response).ok();
            }
            Err(e) => {
                tracing::error!("Auth ticket request failed: {e}");
                let response = json_response(500, serde_json::json!({"error": e}));
                request.respond(response).ok();
            }
        }
    }
}

pub fn generate_hwid() -> Option<String> {
    use sha2::{Digest, Sha256};
    use sysinfo::{Motherboard, Product, System};

    let mut hasher = Sha256::new();
    let mut has_data = false;

    if let Some(uuid) = Product::uuid() {
        hasher.update(uuid.as_bytes());
        has_data = true;
    }
    if let Some(serial) = Product::serial_number() {
        hasher.update(serial.as_bytes());
        has_data = true;
    }

    if let Some(mb) = Motherboard::new() {
        if let Some(serial) = mb.serial_number() {
            hasher.update(serial.as_bytes());
            has_data = true;
        }
        if let Some(name) = mb.name() {
            hasher.update(name.as_bytes());
        }
        if let Some(vendor) = mb.vendor_name() {
            hasher.update(vendor.as_bytes());
        }
    }

    let sys = System::new();
    if let Some(cpu) = sys.cpus().first() {
        hasher.update(cpu.vendor_id().as_bytes());
        hasher.update(cpu.brand().as_bytes());
        has_data = true;
    }

    let config = crate::config::get_config();
    hasher.update(format!("{}-hwid-v1", config.variant).as_bytes());

    if has_data {
        use base64::Engine;
        Some(base64::engine::general_purpose::STANDARD.encode(hasher.finalize()))
    } else {
        None
    }
}

#[allow(clippy::unused_async)] // Uses await when steam feature is enabled
async fn refresh_auth_token(
    #[allow(unused_variables)] app_handle: &tauri::AppHandle,
    mut params: ConnectionParams,
) -> Result<ConnectionParams, String> {
    match params.access_type.as_deref() {
        Some("steam") => {
            #[cfg(feature = "steam")]
            {
                tracing::info!("Refreshing Steam authentication token");
                let steam_state = app_handle
                    .try_state::<Arc<crate::steam::SteamState>>()
                    .ok_or("Steam state not available")?;

                let auth_result = crate::steam::authenticate_with_steam(&steam_state, false)
                    .await
                    .map_err(|e| e.to_string())?;

                if !auth_result.success {
                    return Err(auth_result
                        .error
                        .unwrap_or_else(|| "Steam authentication failed".to_string()));
                }

                params.access_token = auth_result.access_token;
                Ok(params)
            }

            #[cfg(not(feature = "steam"))]
            {
                Err("Steam feature not enabled".to_string())
            }
        }
        Some(access_type) if access_type == crate::config::get_config().variant => {
            let config = crate::config::get_config();
            tracing::info!(
                "Fetching current {} access token",
                config.strings.auth_provider_name
            );
            match crate::auth::TokenStorage::get_tokens().map_err(|e| e.to_string())? {
                Some(tokens) if !crate::auth::TokenStorage::is_expired() => {
                    params.access_token = Some(tokens.access_token);
                    Ok(params)
                }
                _ => Err(format!(
                    "{} authentication expired or not available",
                    config.strings.auth_provider_name
                )),
            }
        }
        _ => Ok(params),
    }
}
