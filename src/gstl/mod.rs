use once_cell::sync::Lazy;

pub mod player;
pub mod list;

pub static MUSIC_PLAYER: Lazy<player::MusicPlayer> = Lazy::new(player::MusicPlayer::new);

