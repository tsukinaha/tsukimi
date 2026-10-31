use gettextrs::gettext;
use strsim::jaro_winkler;

use crate::{
    client::structs::MediaStream,
    ui::{
        models::SETTINGS,
        provider::descriptor::DescriptorType,
    },
};

pub fn make_video_version_choice_from_filter(dl_list: Vec<String>) -> Option<usize> {
    let descriptors = crate::ui::models::SETTINGS.preferred_version_descriptors();
    let mut current_list: Vec<_> = dl_list.iter().collect();

    for descriptor in descriptors {
        let content = &descriptor.content.to_lowercase();
        let previous_list = current_list.to_owned();

        current_list.retain(|&name| match descriptor.type_ {
            DescriptorType::String => name.to_lowercase().contains(content),
            DescriptorType::Regex => {
                regex::Regex::new(content).is_ok_and(|re| re.is_match(&name.to_lowercase()))
            }
        });

        if current_list.is_empty() {
            current_list = previous_list; // Revert to the previous list
        }
    }

    current_list
        .first()
        .and_then(|first_item| dl_list.iter().position(|name| name == *first_item))
}

pub fn make_video_version_choice_from_matcher(
    dl_list: Vec<String>, matcher: &str,
) -> Option<usize> {
    let mut best_match_index = None;
    let mut highest_similarity = 0.0;
    for (index, name) in dl_list.iter().enumerate() {
        let similarity = jaro_winkler(name, matcher);
        if similarity > highest_similarity {
            highest_similarity = similarity;
            best_match_index = Some(index);
        }
    }

    best_match_index
}

pub fn make_subtitle_version_choice(lang_list: Vec<(i64, String)>) -> Option<(i64, usize)> {
    let lang = match SETTINGS.mpv_subtitle_preferred_lang() {
        1 => "English",
        2 => "Chinese Simplified",
        3 => "Japanese",
        4 => "Chinese Traditional",
        5 => "Arabic",
        6 => "Norwegian Bokmål",
        7 => "Portuguese",
        8 => "French",
        9 => "Russian",
        _ => return None,
    };

    let mut best_match_index = None;
    let mut best_match_usize = None;
    let mut highest_similarity = 0.0;
    for (index, i) in lang_list.iter().enumerate() {
        let similarity = jaro_winkler(&i.1, lang);
        if similarity > highest_similarity {
            highest_similarity = similarity;
            best_match_index = Some(i.0);
            best_match_usize = Some(index);
        }
    }

    Some((best_match_index?, best_match_usize?))
}

pub fn subtitle_display_title(stream: &MediaStream) -> String {
    if let Some(title) = stream
        .display_title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
    {
        return title.to_owned();
    }

    let language = [
        stream.display_language.as_deref(),
        stream.language.as_deref(),
        stream.title.as_deref(),
    ]
    .into_iter()
    .flatten()
    .map(str::trim)
    .find(|label| !label.is_empty())
    .map(str::to_owned)
    .unwrap_or_else(|| gettext("Subtitle"));

    match stream
        .codec
        .as_deref()
        .map(str::trim)
        .filter(|codec| !codec.is_empty())
    {
        Some(codec) => format!("{language} · {codec} · #{}", stream.index),
        None => format!("{language} · #{}", stream.index),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn subtitle(fields: serde_json::Value) -> MediaStream {
        let mut value = json!({ "Type": "Subtitle", "Index": 7, "IsExternal": false });
        value
            .as_object_mut()
            .unwrap()
            .extend(fields.as_object().unwrap().clone());
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn subtitle_label_preserves_server_display_title() {
        let stream = subtitle(json!({ "DisplayTitle": "Chinese - Simplified - SRT" }));
        assert_eq!(
            subtitle_display_title(&stream),
            "Chinese - Simplified - SRT"
        );
    }

    #[test]
    fn subtitle_label_uses_language_and_distinguishes_tracks_without_display_titles() {
        let stream = subtitle(json!({ "Language": "chi", "Codec": "subrip" }));
        assert_eq!(subtitle_display_title(&stream), "chi · subrip · #7");
        let other = subtitle(json!({ "Language": "chi", "Codec": "subrip", "Index": 11 }));
        assert_ne!(
            subtitle_display_title(&stream),
            subtitle_display_title(&other)
        );
    }

    #[test]
    fn subtitle_label_skips_blank_metadata() {
        let stream = subtitle(json!({
            "DisplayTitle": " ", "DisplayLanguage": "", "Language": "  ",
            "Title": "Commentary", "Codec": " "
        }));
        assert_eq!(subtitle_display_title(&stream), "Commentary · #7");
        let unnamed = subtitle(json!({}));
        assert!(!subtitle_display_title(&unnamed).trim().is_empty());
        assert!(subtitle_display_title(&unnamed).contains("#7"));
    }
}
