use std::env;

fn env_is(name: &str, value: &str) -> bool {
    env::var(name).ok().is_some_and(|v| v == value)
}

/// Detect Steam Big Picture / Steam Deck Gaming mode via environment variables.
pub fn is_steam_big_picture() -> bool {
    env_is("SteamTenfoot", "1")
        || (env_is("SteamOS", "1") && env_is("SteamGamepadUI", "1"))
        || env_is("SteamDeck", "1")
}
