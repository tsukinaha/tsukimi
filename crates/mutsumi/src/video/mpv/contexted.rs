use super::epoxy_library;

//This struct is actually noting, so it can be safely sent across threads without synchronization.
#[derive(Clone, Copy)]
pub struct ContextedMPV {
    pub mpv: MpvActor,
}

impl Default for ContextedMPV {
    fn default() -> Self {
        unsafe {
            use libc::{
                LC_NUMERIC,
                setlocale,
            };
            setlocale(LC_NUMERIC, c"C".as_ptr() as *const _);
        }
        epoxy_library();
        Self {
            mpv: MpvActor::new(),
        }
    }
}

use crate::{
    TrackSelection,
    arm_mpv_proxy,
    video::{
        MpvActor,
        MpvValue,
        MpvValueType,
    },
};

impl ContextedMPV {
    pub fn shutdown(&self) {
        self.mpv.command("quit", &[]);
    }

    pub fn set_position(&self, value: f64) {
        self.mpv.set_property("time-pos", value);
    }

    pub fn set_percent_position(&self, value: f64) {
        self.mpv.set_property("percent-pos", value);
    }

    pub async fn position(&self) -> f64 {
        let Ok(MpvValue::F64(pos)) = self.mpv.get_property("time-pos", MpvValueType::F64).await
        else {
            return 0.0;
        };

        pos
    }

    pub async fn duration(&self) -> f64 {
        let Ok(MpvValue::F64(duration)) =
            self.mpv.get_property("duration", MpvValueType::F64).await
        else {
            return 0.0;
        };

        duration
    }

    pub async fn paused(&self) -> bool {
        let Ok(MpvValue::Bool(paused)) = self.mpv.get_property("pause", MpvValueType::Bool).await
        else {
            return false;
        };

        paused
    }

    pub async fn volume(&self) -> i64 {
        let Ok(MpvValue::I64(volume)) = self.mpv.get_property("volume", MpvValueType::I64).await
        else {
            return 0;
        };

        volume
    }

    pub async fn speed(&self) -> f64 {
        let Ok(MpvValue::F64(speed)) = self.mpv.get_property("speed", MpvValueType::F64).await
        else {
            return 1.0;
        };

        speed
    }

    pub fn pause(&self, pause: bool) {
        self.mpv.set_property("pause", pause);
    }

    pub fn command_pause(&self) {
        self.mpv.command("cycle", &["pause"]);
    }

    pub fn add_sub(&self, url: &str) {
        self.mpv.command("sub-add", &[url, "select"]);
    }

    pub fn load_video(&self, url: &str) {
        // mpv will read "WAYLAND_DISPLAY" everytime on loading file
        arm_mpv_proxy();

        self.mpv.command("loadfile", &[url, "replace"]);
    }

    pub fn set_playlist(&self, urls: &[String]) {
        if urls.is_empty() {
            self.mpv.command("playlist-clear", &[]);
            self.mpv.command("stop", &[]);
            return;
        }

        arm_mpv_proxy();

        let mut iter = urls.iter();
        if let Some(first) = iter.next() {
            self.mpv.command("loadfile", &[first, "replace"]);
        }
        for url in iter {
            self.mpv.command("loadfile", &[url, "append"]);
        }
    }

    pub fn set_playlist_pos(&self, pos: i64) {
        arm_mpv_proxy();
        self.mpv.set_property("playlist-pos", pos);
    }

    pub fn set_loop_playlist(&self, loop_: &str) {
        self.mpv.set_property("loop-playlist", loop_);
    }

    pub fn set_loop_file(&self, loop_: &str) {
        self.mpv.set_property("loop-file", loop_);
    }

    pub fn playlist_shuffle(&self) {
        self.mpv.command("playlist-shuffle", &[]);
    }

    pub fn playlist_unshuffle(&self) {
        self.mpv.command("playlist-unshuffle", &[]);
    }

    pub fn playlist_add(&self, url: &str, index: i64) {
        arm_mpv_proxy();
        self.mpv
            .command("loadfile", &[url, "insert-at", &index.to_string()]);
    }

    pub fn playlist_remove(&self, index: i64) {
        self.mpv.command("playlist-remove", &[&index.to_string()]);
    }

    pub fn playlist_move(&self, from: i64, to: i64) {
        self.mpv
            .command("playlist-move", &[&from.to_string(), &to.to_string()]);
    }

    pub fn set_start_time(&self, second: u64) {
        self.mpv.set_property("start", second.to_string());
    }

    pub fn set_start(&self, second: f64) {
        self.mpv.set_property("start", format!("{second:.2}"));
    }

    pub fn set_volume(&self, volume: i64) {
        self.mpv.set_property("volume", volume);
    }

    pub fn volume_scroll(&self, value: i64) {
        self.mpv.command("add", &["volume", &value.to_string()]);
    }

    pub fn set_speed(&self, speed: f64) {
        self.mpv.set_property("speed", speed);
    }

    pub fn set_aid(&self, aid: TrackSelection) {
        self.mpv.set_property("aid", aid.to_string());
    }

    pub fn set_sid(&self, sid: TrackSelection) {
        self.mpv.set_property("sid", sid.to_string());
    }

    pub fn seek_forward(&self, value: i64) {
        self.mpv.command("seek", &[&value.to_string()]);
    }

    pub fn seek_backward(&self, value: i64) {
        self.mpv.command("seek", &[&(-value).to_string()]);
    }

    pub fn press_key(&self, key: u32, state: gtk::gdk::ModifierType) {
        let keystr = get_full_keystr(key, state);
        if let Some(keystr) = keystr {
            tracing::debug!("MPV Catch Key pressed: {}", keystr);
            self.mpv.command("keypress", &[&keystr]);
        }
    }

    pub fn release_key(&self, key: u32, state: gtk::gdk::ModifierType) {
        let keystr = get_full_keystr(key, state);
        if let Some(keystr) = keystr {
            tracing::debug!("MPV Catch Key released: {}", keystr);
            self.mpv.command("keyup", &[&keystr]);
        }
    }

    pub fn stop(&self) {
        self.mpv.command("stop", &[]);
    }

    pub fn display_stats_toggle(&self) {
        self.mpv
            .command("script-binding", &["stats/display-stats-toggle"]);
    }

    pub fn set_slang(&self, value: String) {
        self.mpv.set_property("slang", value);
    }

    pub async fn get_track_id(&self, type_: &str) -> i64 {
        let Ok(MpvValue::String(track)) = self.mpv.get_property(type_, MpvValueType::String).await
        else {
            return 0;
        };

        track.parse().unwrap_or(0)
    }
}

fn get_full_keystr(key: u32, state: gtk::gdk::ModifierType) -> Option<String> {
    Some(format!("{}{}", get_modstr(state), keyval_to_keystr(key)?))
}

fn get_modstr(state: gtk::gdk::ModifierType) -> String {
    struct ModMap {
        mask: gtk::gdk::ModifierType,
        str: &'static str,
    }

    let mod_map = [
        ModMap {
            mask: gtk::gdk::ModifierType::SHIFT_MASK,
            str: "Shift+",
        },
        ModMap {
            mask: gtk::gdk::ModifierType::CONTROL_MASK,
            str: "Ctrl+",
        },
        ModMap {
            mask: gtk::gdk::ModifierType::ALT_MASK,
            str: "Alt+",
        },
        ModMap {
            mask: gtk::gdk::ModifierType::SUPER_MASK
                | gtk::gdk::ModifierType::META_MASK
                | gtk::gdk::ModifierType::HYPER_MASK,
            str: "Meta+",
        },
    ];

    let mut result = String::new();

    for mod_item in &mod_map {
        if state.intersects(mod_item.mask) {
            result.push_str(mod_item.str);
        }
    }

    result
}

use gtk::glib::translate::FromGlib;

const KEYSTRING_MAP: &[(&str, &str)] = &[
    ("PGUP", "Page_Up"),
    ("PGDWN", "Page_Down"),
    ("BS", "\x08"),
    ("SHARP", "#"),
    ("UP", "KP_Up"),
    ("DOWN", "KP_Down"),
    ("RIGHT", "KP_Right"),
    ("LEFT", "KP_Left"),
    ("RIGHT", "Right"),
    ("LEFT", "Left"),
    ("UP", "Up"),
    ("DOWN", "Down"),
    ("ESC", "\x1b"),
    ("DEL", "\x7f"),
    ("ENTER", "\r"),
    ("ENTER", "Return"),
    ("INS", "Insert"),
    ("VOLUME_DOWN", "AudioLowerVolume"),
    ("MUTE", "AudioMute"),
    ("VOLUME_UP", "AudioRaiseVolume"),
    ("PLAY", "AudioPlay"),
    ("STOP", "AudioStop"),
    ("PREV", "AudioPrev"),
    ("NEXT", "AudioNext"),
    ("FORWARD", "AudioForward"),
    ("REWIND", "AudioRewind"),
    ("MENU", "Menu"),
    ("HOMEPAGE", "HomePage"),
    ("MAIL", "Mail"),
    ("FAVORITES", "Favorites"),
    ("SEARCH", "Search"),
    ("SLEEP", "Sleep"),
    ("CANCEL", "Cancel"),
    ("RECORD", "AudioRecord"),
    ("SPACE", " "),
];

const MODIFIER_KEY_NAMES: &[&str] = &[
    "Shift_L",
    "Shift_R",
    "Control_L",
    "Control_R",
    "Alt_L",
    "Alt_R",
    "Meta_L",
    "Meta_R",
    "Super_L",
    "Super_R",
    "Hyper_L",
    "Hyper_R",
    "ISO_Level3_Shift",
    "ISO_Level5_Shift",
    "Caps_Lock",
    "Num_Lock",
    "Scroll_Lock",
];

fn key_name_to_keystr(key_name: &str) -> Option<&str> {
    if MODIFIER_KEY_NAMES.contains(&key_name) {
        return None;
    }

    Some(
        KEYSTRING_MAP
            .iter()
            .find(|(_, gdk_name)| *gdk_name == key_name)
            .map_or(key_name, |(mpv_name, _)| mpv_name),
    )
}

fn keyval_to_keystr(keyval: u32) -> Option<String> {
    let key = unsafe { gtk::gdk::Key::from_glib(keyval) };

    let key_name = if let Some(c) = key.to_unicode() {
        c.to_string()
    } else {
        key.name()?.to_string()
    };

    key_name_to_keystr(&key_name).map(str::to_owned)
}
