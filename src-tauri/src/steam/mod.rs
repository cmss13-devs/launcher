pub mod commands;
pub mod presence;
pub mod state;

pub use commands::{
    authenticate_with_steam, cancel_steam_auth_ticket, get_steam_auth_ticket,
    get_steam_launch_options, get_steam_user_info, steam_authenticate,
};

pub use presence::SteamPresence;
pub use state::SteamState;

use crate::{DEFAULT_STEAM_ID, DEFAULT_STEAM_NAME};

pub fn get_steam_app_id() -> u32 {
    if let Some(env) = option_env!("STEAM_APP_ID") {
        #[allow(clippy::expect_used)] // Compile-time env var - invalid value is developer error
        env.parse().expect("invalid STEAM_APP_ID")
    } else {
        DEFAULT_STEAM_ID
    }
}

pub fn get_steam_app_name() -> String {
    if let Some(env) = option_env!("STEAM_APP_NAME") {
        env.to_string()
    } else {
        DEFAULT_STEAM_NAME.to_string()
    }
}
