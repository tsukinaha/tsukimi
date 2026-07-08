use crate::{
    client::structs::MediaStream,
    playback::rules::{
        self,
        AudioOutcome,
        PlaybackOutcome,
        PlaybackRules,
        SubtitleOutcome,
    },
    ui::{
        SETTINGS,
        widgets::item_utils::make_subtitle_version_choice,
    },
};

#[derive(Debug, Clone, Default)]
pub struct ResolvedPlaybackTracks {
    pub prefer_audio_lang: Option<String>,
    pub prefer_subtitle_lang: Option<String>,
    pub forced_subtitles_only: bool,
    pub subtitles_off: bool,
}

pub fn resolve_playback_tracks(audio_language: Option<&str>) -> ResolvedPlaybackTracks {
    let config = PlaybackRules::load();
    if !config.enabled {
        return legacy_fallback();
    }
    let outcome = PlaybackRules::evaluate(audio_language, &config);
    resolved_from_outcome(&outcome)
}

pub fn detect_default_audio_language(streams: &[MediaStream]) -> Option<String> {
    let audio_streams: Vec<_> = streams
        .iter()
        .filter(|stream| stream.stream_type == "Audio")
        .collect();
    if audio_streams.is_empty() {
        return None;
    }

    let preferred = SETTINGS.mpv_audio_preferred_lang_str();
    if !preferred.is_empty()
        && let Some(stream) = audio_streams
            .iter()
            .find(|stream| rules::language_matches(stream.language.as_deref(), &preferred))
    {
        return stream.language.clone();
    }

    audio_streams
        .first()
        .and_then(|stream| stream.language.clone())
}

pub fn resolve_slang(resolved: &ResolvedPlaybackTracks) -> String {
    if resolved.subtitles_off {
        return String::new();
    }
    resolved
        .prefer_subtitle_lang
        .clone()
        .unwrap_or_else(|| SETTINGS.mpv_subtitle_preferred_lang_str())
}

pub fn resolve_alang(resolved: &ResolvedPlaybackTracks) -> String {
    resolved
        .prefer_audio_lang
        .clone()
        .unwrap_or_else(|| SETTINGS.mpv_audio_preferred_lang_str())
}

fn legacy_fallback() -> ResolvedPlaybackTracks {
    ResolvedPlaybackTracks {
        prefer_audio_lang: {
            let lang = SETTINGS.mpv_audio_preferred_lang_str();
            if lang.is_empty() { None } else { Some(lang) }
        },
        prefer_subtitle_lang: {
            let lang = SETTINGS.mpv_subtitle_preferred_lang_str();
            if lang.is_empty() { None } else { Some(lang) }
        },
        forced_subtitles_only: false,
        subtitles_off: SETTINGS.mpv_subtitle_preferred_lang_str().is_empty(),
    }
}

fn resolved_from_outcome(outcome: &PlaybackOutcome) -> ResolvedPlaybackTracks {
    let mut resolved = legacy_fallback();

    match &outcome.audio {
        AudioOutcome::NoOverride => {}
        AudioOutcome::PreferLanguage { language } => {
            resolved.prefer_audio_lang = Some(language.clone());
        }
        AudioOutcome::Original => {
            resolved.prefer_audio_lang = None;
        }
    }

    match &outcome.subtitles {
        SubtitleOutcome::Off => {
            resolved.subtitles_off = true;
            resolved.prefer_subtitle_lang = None;
            resolved.forced_subtitles_only = false;
        }
        SubtitleOutcome::Forced { language } => {
            resolved.subtitles_off = false;
            resolved.forced_subtitles_only = true;
            if !language.is_empty() {
                resolved.prefer_subtitle_lang = Some(language.clone());
            }
        }
        SubtitleOutcome::Full { language } => {
            resolved.subtitles_off = false;
            resolved.forced_subtitles_only = false;
            if !language.is_empty() {
                resolved.prefer_subtitle_lang = Some(language.clone());
            }
        }
        SubtitleOutcome::PreferLanguage { language } => {
            resolved.subtitles_off = false;
            resolved.forced_subtitles_only = false;
            resolved.prefer_subtitle_lang = Some(language.clone());
        }
    }

    resolved
}

pub fn pick_subtitle_stream<'a>(
    streams: &'a [MediaStream], resolved: &ResolvedPlaybackTracks,
) -> Option<&'a MediaStream> {
    if resolved.subtitles_off {
        return None;
    }

    let subtitle_streams: Vec<_> = streams
        .iter()
        .filter(|s| s.stream_type == "Subtitle")
        .collect();

    if subtitle_streams.is_empty() {
        return None;
    }

    if resolved.forced_subtitles_only
        && let Some(stream) = subtitle_streams
            .iter()
            .find(|s| s.is_forced.unwrap_or(false))
    {
        return Some(*stream);
    }

    if let Some(lang) = resolved.prefer_subtitle_lang.as_deref()
        && let Some(stream) = subtitle_streams
            .iter()
            .find(|s| rules::language_matches(stream_language(s), lang))
    {
        return Some(*stream);
    }

    let lang_list: Vec<(i64, String)> = subtitle_streams
        .iter()
        .map(|s| {
            (
                s.index,
                s.display_title
                    .clone()
                    .or_else(|| s.title.clone())
                    .unwrap_or_default(),
            )
        })
        .collect();

    if let Some((_, list_index)) = make_subtitle_version_choice(lang_list) {
        return subtitle_streams.get(list_index).copied();
    }

    subtitle_streams.first().copied()
}

fn stream_language(stream: &MediaStream) -> Option<&str> {
    stream
        .language
        .as_deref()
        .or(stream.display_language.as_deref())
}
