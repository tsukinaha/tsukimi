use once_cell::sync::Lazy;

pub mod list;
pub mod player;

pub static MUSIC_PLAYER: Lazy<player::MusicPlayer> = Lazy::new(player::MusicPlayer::new);
