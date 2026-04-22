//! Manages multiple presence providers and game session state
#![allow(clippy::unwrap_used)] // Mutex::lock().unwrap() is idiomatic - panic on poison

use std::process::Child;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::Manager;

use super::traits::{ConnectionParams, GameSession, PresenceProvider, PresenceState};
use crate::servers::ServerState;

/// Manages game session state and multiple presence providers
pub struct PresenceManager {
    providers: Vec<Box<dyn PresenceProvider>>,
    game_session: Arc<Mutex<Option<GameSession>>>,
    game_process: Arc<Mutex<Option<Child>>>,
    game_process_pid: Arc<Mutex<Option<u32>>>,
    last_connection_params: Arc<Mutex<Option<ConnectionParams>>>,
}

impl PresenceManager {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            game_session: Arc::new(Mutex::new(None)),
            game_process: Arc::new(Mutex::new(None)),
            game_process_pid: Arc::new(Mutex::new(None)),
            last_connection_params: Arc::new(Mutex::new(None)),
        }
    }

    #[allow(dead_code)]
    pub fn add_provider(&mut self, provider: Box<dyn PresenceProvider>) {
        tracing::info!("Adding presence provider: {}", provider.name());
        provider.update_presence(&PresenceState::InLauncher);
        self.providers.push(provider);
    }

    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub fn start_game_session(
        &self,
        server_name: String,
        map_name: Option<String>,
        player_count: u32,
        process: Child,
    ) {
        tracing::info!("Starting game session on {}", server_name);
        {
            let mut session = self.game_session.lock().unwrap();
            *session = Some(GameSession {
                server_name: server_name.clone(),
                map_name: map_name.clone(),
            });
        }
        {
            let mut proc = self.game_process.lock().unwrap();
            *proc = Some(process);
        }

        self.update_all_presence(&PresenceState::Playing {
            server_name,
            player_count,
            map_name,
        });
    }

    #[cfg_attr(not(any(target_os = "windows", target_os = "linux")), allow(dead_code))]
    pub fn start_game_session_by_pid(
        &self,
        server_name: String,
        map_name: Option<String>,
        player_count: u32,
        pid: u32,
    ) {
        tracing::info!(
            "Starting game session on {} (tracking PID {})",
            server_name,
            pid
        );
        {
            let mut session = self.game_session.lock().unwrap();
            *session = Some(GameSession {
                server_name: server_name.clone(),
                map_name: map_name.clone(),
            });
        }
        {
            let mut pid_guard = self.game_process_pid.lock().unwrap();
            *pid_guard = Some(pid);
        }

        self.update_all_presence(&PresenceState::Playing {
            server_name,
            player_count,
            map_name,
        });
    }

    pub fn check_game_running(&self) -> bool {
        // First check if we're tracking a Child process
        let mut proc_guard = self.game_process.lock().unwrap();

        if let Some(ref mut child) = *proc_guard {
            match child.try_wait() {
                Ok(Some(_status)) => {
                    drop(proc_guard);
                    self.clear_game_session();
                    return false;
                }
                Ok(None) => return true,
                Err(_) => {
                    drop(proc_guard);
                    self.clear_game_session();
                    return false;
                }
            }
        }
        drop(proc_guard);

        // Check if we're tracking by PID instead
        let pid_guard = self.game_process_pid.lock().unwrap();
        if let Some(pid) = *pid_guard {
            drop(pid_guard);
            if Self::is_process_running(pid) {
                return true;
            }
        } else {
            drop(pid_guard);
        }

        #[cfg(any(target_os = "windows", target_os = "linux"))]
        {
            let params = self.last_connection_params.lock().unwrap();
            if let Some(ref key) = params.as_ref().and_then(|p| p.launcher_key.as_ref()) {
                if crate::byond::find_dreamseeker_pid_by_key(key).is_some() {
                    return true;
                }
            }
        }

        if self.game_session.lock().unwrap().is_some() {
            self.clear_game_session();
        }
        false
    }

    /// Check if a process with the given PID is still running
    fn is_process_running(pid: u32) -> bool {
        use sysinfo::{Pid, System};
        let s = System::new_all();
        s.process(Pid::from_u32(pid)).is_some()
    }

    pub fn get_game_session(&self) -> Option<GameSession> {
        self.game_session.lock().unwrap().clone()
    }

    pub fn clear_game_session(&self) {
        {
            let mut session = self.game_session.lock().unwrap();
            *session = None;
        }
        {
            let mut proc = self.game_process.lock().unwrap();
            *proc = None;
        }
        {
            let mut pid = self.game_process_pid.lock().unwrap();
            *pid = None;
        }
        self.update_all_presence(&PresenceState::InLauncher);
    }

    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub fn set_last_connection_params(&self, params: ConnectionParams) {
        let mut connection_params = self.last_connection_params.lock().unwrap();
        *connection_params = Some(params);
    }

    pub fn get_last_connection_params(&self) -> Option<ConnectionParams> {
        self.last_connection_params.lock().unwrap().clone()
    }

    pub fn kill_game_process(&self) -> bool {
        let mut proc_guard = self.game_process.lock().unwrap();

        if let Some(ref mut child) = *proc_guard {
            match child.kill() {
                Ok(()) => {
                    tracing::info!("Game process killed successfully via Child handle");
                    let _ = child.wait();
                    drop(proc_guard);
                    self.clear_game_session();
                    return true;
                }
                Err(e) => {
                    tracing::warn!("Failed to kill game process via Child handle: {}", e);
                }
            }
        }
        drop(proc_guard);

        #[cfg(any(target_os = "windows", target_os = "linux"))]
        {
            let params = self.last_connection_params.lock().unwrap();
            if let Some(ref key) = params.as_ref().and_then(|p| p.launcher_key.as_ref()) {
                if crate::byond::kill_dreamseeker_by_key(key) {
                    drop(params);
                    self.clear_game_session();
                    return true;
                }
            }
        }

        tracing::debug!("No game process to kill");
        false
    }

    pub fn update_all_presence(&self, state: &PresenceState) {
        tracing::debug!("Updating presence: {:?}", state);
        for provider in &self.providers {
            provider.update_presence(state);
        }
    }

    #[allow(dead_code)]
    pub fn clear_all_presence(&self) {
        for provider in &self.providers {
            provider.clear_presence();
        }
    }
}

impl Default for PresenceManager {
    fn default() -> Self {
        Self::new()
    }
}

pub fn start_presence_background_task(
    presence_manager: Arc<PresenceManager>,
    poll_callback: Option<Box<dyn Fn() + Send + Sync>>,
    app_handle: tauri::AppHandle,
) {
    use tauri::Emitter;

    tauri::async_runtime::spawn(async move {
        let poll_interval = Duration::from_millis(100);
        let mut was_game_running = false;
        let mut last_player_count: Option<i32> = None;
        let mut last_map_name: Option<String> = None;

        loop {
            if let Some(ref callback) = poll_callback {
                callback();
            }

            let game_running = presence_manager.check_game_running();

            if game_running {
                was_game_running = true;

                if let Some(session) = presence_manager.get_game_session() {
                    let (player_count, map_name) = if let Some(server_state) =
                        app_handle.try_state::<Arc<ServerState>>()
                    {
                        let servers = server_state.get_servers().await;
                        if let Some(server) = servers.iter().find(|s| s.name == session.server_name)
                        {
                            let player_count = server.data.as_ref().map(|d| d.players);
                            let map_name = server
                                .data
                                .as_ref()
                                .map(|d| d.map_name.clone())
                                .or_else(|| session.map_name.clone());
                            (player_count, map_name)
                        } else {
                            (None, session.map_name.clone())
                        }
                    } else {
                        (None, session.map_name.clone())
                    };

                    if player_count != last_player_count || map_name != last_map_name {
                        last_player_count = player_count;
                        last_map_name.clone_from(&map_name);

                        presence_manager.update_all_presence(&PresenceState::Playing {
                            server_name: session.server_name.clone(),
                            #[allow(clippy::cast_sign_loss)] // Player count is non-negative
                            player_count: player_count.unwrap_or(0) as u32,
                            map_name,
                        });
                    }
                }
            } else if was_game_running {
                was_game_running = false;
                last_player_count = None;
                last_map_name = None;
                presence_manager.update_all_presence(&PresenceState::InLauncher);
                app_handle.emit("game-closed", ()).ok();
            }

            tokio::time::sleep(poll_interval).await;
        }
    });
}
