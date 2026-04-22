#[derive(Debug, Clone)]
pub struct GameSession {
    pub server_name: String,
    pub map_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConnectionParams {
    pub version: String,
    pub host: String,
    pub port: String,
    pub access_type: Option<String>,
    pub access_token: Option<String>,
    pub server_name: String,
    pub map_name: Option<String>,
    pub server_id: Option<String>,
    #[cfg_attr(not(any(target_os = "windows", target_os = "linux")), allow(dead_code))]
    pub launcher_key: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum PresenceState {
    InLauncher,
    Playing {
        server_name: String,
        player_count: u32,
        map_name: Option<String>,
    },
    #[allow(dead_code)]
    Disconnected,
}

#[allow(dead_code)]
pub trait PresenceProvider: Send + Sync {
    /// Returns the name of this presence provider (for logging)
    fn name(&self) -> &'static str;

    /// Update the presence state
    fn update_presence(&self, state: &PresenceState);

    /// Clear all presence data
    fn clear_presence(&self);
}
