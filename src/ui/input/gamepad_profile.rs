#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GamepadProfile {
    #[default]
    Generic,
    Xbox,
    PlayStation,
    SteamDeck,
    Nintendo,
}

impl GamepadProfile {
    pub fn detect(name: &str) -> Self {
        let lower = name.to_lowercase();
        if lower.contains("xbox") || lower.contains("microsoft") || lower.contains("360 controller")
        {
            Self::Xbox
        } else if lower.contains("sony")
            || lower.contains("playstation")
            || lower.contains("dualshock")
            || lower.contains("dualsense")
        {
            Self::PlayStation
        } else if lower.contains("steam deck") || lower.contains("steam controller") {
            Self::SteamDeck
        } else if lower.contains("nintendo")
            || lower.contains("pro controller")
            || lower.contains("joy-con")
        {
            Self::Nintendo
        } else {
            Self::Generic
        }
    }

    pub fn activate_label(self) -> &'static str {
        match self {
            Self::PlayStation => "✕",
            Self::Nintendo => "B",
            Self::Xbox | Self::SteamDeck | Self::Generic => "A",
        }
    }

    pub fn back_label(self) -> &'static str {
        match self {
            Self::PlayStation => "○",
            Self::Nintendo => "A",
            Self::Xbox | Self::SteamDeck | Self::Generic => "B",
        }
    }
}
