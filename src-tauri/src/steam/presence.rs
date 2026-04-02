use steamworks::Client;

use crate::presence::{PresenceProvider, PresenceState};

pub struct SteamPresence {
    client: Client,
}

impl SteamPresence {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    fn set_playing_status(&self, server_name: &str, player_count: u32, map_name: Option<&str>) {
        tracing::debug!(
            "Setting Steam presence: Playing on {} ({} players, map: {:?})",
            server_name,
            player_count,
            map_name
        );
        let friends = self.client.friends();

        let status = match map_name {
            Some(map) => format!("{player_count} players on {map}"),
            None => format!("Playing on {server_name}"),
        };
        friends.set_rich_presence("status", Some(&status));

        let encoded_server =
            url::form_urlencoded::byte_serialize(server_name.as_bytes()).collect::<String>();
        friends.set_rich_presence("connect", Some(&encoded_server));

        friends.set_rich_presence("players", Some(&player_count.to_string()));
        friends.set_rich_presence("name", Some(server_name));
        if let Some(map) = map_name {
            friends.set_rich_presence("map", Some(map));
        }

        friends.set_rich_presence("steam_display", Some("#Status_Playing_Map"));
        friends.set_rich_presence("steam_player_group", Some(server_name));
        friends.set_rich_presence("steam_player_group_size", Some(&player_count.to_string()));
    }

    fn set_launcher_status(&self) {
        tracing::debug!("Setting Steam presence: In Launcher");
        self.clear_presence();

        let friends = self.client.friends();

        friends.set_rich_presence("status", Some("In the Launcher"));
        friends.set_rich_presence("steam_display", Some("#Status_Launcher"));
    }
}

impl PresenceProvider for SteamPresence {
    fn name(&self) -> &'static str {
        "Steam"
    }

    fn update_presence(&self, state: &PresenceState) {
        match state {
            PresenceState::InLauncher => self.set_launcher_status(),
            PresenceState::Playing {
                server_name,
                player_count,
                map_name,
            } => self.set_playing_status(server_name, *player_count, map_name.as_deref()),
            PresenceState::Disconnected => self.clear_presence(),
        }
    }

    fn clear_presence(&self) {
        self.client.friends().clear_rich_presence();
    }
}
