use std::{
    fs,
    path::{
        Path,
        PathBuf,
    },
    process::Command,
};

use steam_shortcuts_util::{
    Shortcut,
    parse_shortcuts,
    shortcut::ShortcutOwned,
    shortcuts_to_bytes,
};

use crate::ui::widgets::utils::GlobalToast;

pub fn is_steam_running() -> bool {
    Command::new("pgrep")
        .arg("-x")
        .arg("steam")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn find_steam_root() -> Option<PathBuf> {
    let candidates = [
        dirs::home_dir().map(|h| h.join(".steam/root")),
        dirs::home_dir().map(|h| h.join(".local/share/Steam")),
        dirs::home_dir().map(|h| h.join(".var/app/com.valvesoftware.Steam/data/Steam")),
    ];
    candidates.into_iter().flatten().find(|path| path.is_dir())
}

fn find_active_user_id(steam_root: &Path) -> Option<String> {
    let loginusers = fs::read_to_string(steam_root.join("config/loginusers.vdf")).ok()?;
    let mut most_recent_id = None;
    let mut in_user = false;
    for line in loginusers.lines() {
        let line = line.trim();
        if line.starts_with('"') && line.ends_with('{') {
            in_user = true;
            most_recent_id = Some(
                line.trim_matches('"')
                    .trim_end_matches('{')
                    .trim()
                    .to_string(),
            );
        }
        if in_user && line.contains("\"MostRecent\"") && line.contains("\"1\"") {
            return most_recent_id;
        }
        if line == "}" {
            in_user = false;
        }
    }
    None
}

pub fn shortcut_exists(steam_root: &Path, user_id: &str, exe: &str) -> bool {
    let path = steam_root.join(format!("userdata/{user_id}/config/shortcuts.vdf"));
    let Ok(content) = fs::read(&path) else {
        return false;
    };
    let Ok(shortcuts) = parse_shortcuts(content.as_slice()) else {
        return false;
    };
    shortcuts
        .iter()
        .any(|s| s.exe == exe || s.exe.contains("tsukimi"))
}

pub fn add_tsukimi_to_steam(window: &crate::Window) -> Result<(), String> {
    if is_steam_running() {
        return Err(
            "Close Steam completely before adding Tsukimi. Steam overwrites shortcuts on exit."
                .into(),
        );
    }

    let steam_root = find_steam_root().ok_or("Steam installation not found")?;
    let user_id = find_active_user_id(&steam_root).ok_or("Could not find active Steam user")?;

    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_str = exe.to_string_lossy().to_string();
    let start_dir = exe
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    if shortcut_exists(&steam_root, &user_id, &exe_str) {
        return Err("Tsukimi is already in your Steam library".into());
    }

    let shortcuts_path = steam_root.join(format!("userdata/{user_id}/config/shortcuts.vdf"));
    let mut owned: Vec<ShortcutOwned> = if shortcuts_path.exists() {
        let content = fs::read(&shortcuts_path).map_err(|e| e.to_string())?;
        parse_shortcuts(content.as_slice())?
            .into_iter()
            .map(|s| s.to_owned())
            .collect()
    } else {
        Vec::new()
    };

    let order = owned.len().to_string();
    let new_shortcut = Shortcut::new(
        &order,
        "Tsukimi",
        &exe_str,
        &start_dir,
        "",
        "",
        "--tv-mode --fullscreen",
    );
    let mut owned_shortcut = new_shortcut.to_owned();
    owned_shortcut.tags = vec!["Installed".to_string(), "Ready to Play".to_string()];
    owned.push(owned_shortcut);

    let borrowed: Vec<Shortcut> = owned.iter().map(|s| s.borrow()).collect();

    if let Some(parent) = shortcuts_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&shortcuts_path, shortcuts_to_bytes(&borrowed)).map_err(|e| e.to_string())?;

    window.toast("Added to Steam. Restart Steam, then launch Tsukimi from Big Picture.");
    Ok(())
}
