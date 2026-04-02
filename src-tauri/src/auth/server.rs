#![allow(clippy::expect_used)] // static header bytes never fail

use std::time::Duration;
use tiny_http::{Response, Server};
use url::Url;

const CALLBACK_TIMEOUT_SECS: u64 = 300; // 5 minutes

const SUCCESS_HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
    <title>Login Successful</title>
    <style>
        body {
            background: #1a1919;
            color: #00eb4e;
            font-family: monospace;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            margin: 0;
        }
        .container {
            text-align: center;
            border: 2px solid #00eb4e;
            padding: 40px 60px;
            box-shadow: 0 0 20px #00eb4e44;
        }
        h1 {
            margin: 0 0 16px;
            text-transform: uppercase;
            letter-spacing: 2px;
        }
        p {
            margin: 0;
            opacity: 0.7;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>Authentication Successful</h1>
        <p>You can close this window and return to the launcher.</p>
    </div>
</body>
</html>"#;

const ERROR_HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
    <title>Login Failed</title>
    <style>
        body {
            background: #1a1919;
            color: #ff4444;
            font-family: monospace;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            margin: 0;
        }
        .container {
            text-align: center;
            border: 2px solid #ff4444;
            padding: 40px 60px;
            box-shadow: 0 0 20px #ff444444;
        }
        h1 {
            margin: 0 0 16px;
            text-transform: uppercase;
            letter-spacing: 2px;
        }
        p {
            margin: 0;
            opacity: 0.7;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>Authentication Failed</h1>
        <p>{{ERROR}}</p>
    </div>
</body>
</html>"#;

pub struct CallbackResult {
    pub code: String,
    #[allow(dead_code)]
    pub state: String,
}

pub struct CallbackServer {
    server: Server,
    expected_state: Option<String>,
    pub port: u16,
}

impl CallbackServer {
    pub fn start_without_state() -> Result<Self, String> {
        tracing::info!("Starting OAuth callback server on 127.0.0.1:0");

        let server = Server::http("127.0.0.1:0").map_err(|e| {
            tracing::error!(
                "Failed to start OAuth callback server: {} (error type: {:?})",
                e,
                std::any::type_name_of_val(&e)
            );
            tracing::error!(
                "This may be caused by: firewall blocking the connection, \
                antivirus software, or network configuration issues"
            );
            format!(
                "Failed to start callback server: {e}. \
                Please check your firewall and antivirus settings."
            )
        })?;

        let addr = server.server_addr().to_ip().ok_or_else(|| {
            tracing::error!("Failed to get server address after binding");
            "Failed to get server address".to_string()
        })?;

        let port = addr.port();
        tracing::info!(
            "OAuth callback server started successfully on {}:{}",
            addr.ip(),
            port
        );

        Ok(Self {
            server,
            expected_state: None,
            port,
        })
    }

    pub fn set_expected_state(&mut self, state: String) {
        self.expected_state = Some(state);
    }

    pub fn redirect_uri(&self) -> String {
        format!("http://127.0.0.1:{}/callback", self.port)
    }

    pub fn wait_for_callback(self) -> Result<CallbackResult, String> {
        tracing::info!(
            "Waiting for OAuth callback on port {} (timeout: {}s)",
            self.port,
            CALLBACK_TIMEOUT_SECS
        );

        loop {
            let request = match self
                .server
                .recv_timeout(Duration::from_secs(CALLBACK_TIMEOUT_SECS))
            {
                Ok(Some(req)) => {
                    tracing::debug!("Received request: {} {}", req.method(), req.url());
                    req
                }
                Ok(None) => continue,
                Err(e) => {
                    tracing::error!(
                        "OAuth callback server timed out after {}s: {:?}",
                        CALLBACK_TIMEOUT_SECS,
                        e
                    );
                    return Err("Callback server timed out waiting for authentication".to_string());
                }
            };
            let full_url = format!("http://127.0.0.1{}", request.url());
            let url =
                Url::parse(&full_url).map_err(|e| format!("Failed to parse callback URL: {e}"))?;

            tracing::debug!("Callback server received request: {}", url.path());

            if url.path() != "/callback" {
                let response = Response::from_string("Not Found").with_status_code(404);
                request.respond(response).ok();
                continue;
            }

            let params: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();

            if let Some(error) = params.get("error") {
                let error_desc = params
                    .get("error_description")
                    .map_or("Unknown error", std::string::String::as_str);

                let html = ERROR_HTML.replace("{{ERROR}}", error_desc);
                let response = Response::from_string(html)
                    .with_header(
                        tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..])
                            .expect("static header"),
                    )
                    .with_status_code(400);
                request.respond(response).ok();

                return Err(format!("OAuth error: {error} - {error_desc}"));
            }

            let code = params
                .get("code")
                .ok_or("Missing authorization code in callback")?
                .clone();

            let state = params
                .get("state")
                .ok_or("Missing state in callback")?
                .clone();

            if let Some(ref expected) = self.expected_state {
                if &state != expected {
                    let html = ERROR_HTML.replace(
                        "{{ERROR}}",
                        "Invalid state parameter - possible CSRF attack",
                    );
                    let response = Response::from_string(html)
                        .with_header(
                            tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..])
                                .expect("static header"),
                        )
                        .with_status_code(400);
                    request.respond(response).ok();

                    return Err("State mismatch - possible CSRF attack".to_string());
                }
            }

            let response = Response::from_string(SUCCESS_HTML).with_header(
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..])
                    .expect("static header"),
            );
            request.respond(response).ok();

            return Ok(CallbackResult { code, state });
        }
    }
}
