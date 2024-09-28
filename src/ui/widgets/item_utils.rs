use crate::ui::{models::SETTINGS, provider::descriptor::DescriptorType};
use strsim::jaro_winkler;

pub fn make_video_version_choice_from_filter(dl_list: Vec<String>) -> Option<usize> {
    let descriptors = crate::ui::models::SETTINGS.preferred_version_descriptors();
    let mut current_list: Vec<_> = dl_list.iter().collect();

    for descriptor in descriptors {
        let content = &descriptor.content.to_lowercase();
        let previous_list = current_list.clone();

        current_list.retain(|&name| match descriptor.type_ {
            DescriptorType::String => name.to_lowercase().contains(content),
            DescriptorType::Regex => {
                regex::Regex::new(content).map_or(false, |re| re.is_match(&name.to_lowercase()))
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
    dl_list: Vec<String>,
    matcher: &str,
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

pub fn make_subtitle_version_choice(lang_list: Vec<String>) -> Option<usize> {
    let lang = match SETTINGS.mpv_subtitle_preferred_lang() {
        1 => "English",
        2 => "Chinese Simplified",
        3 => "Japanese",
        4 => "Chinese Traditional",
        5 => "Arabic",
        6 => "Norwegian Bokmål",
        7 => "Portuguese",
        _ => return None,
    };

    let mut best_match_index = None;
    let mut highest_similarity = 0.0;
    for (index, name) in lang_list.iter().enumerate() {
        let similarity = jaro_winkler(name, lang);
        if similarity > highest_similarity {
            highest_similarity = similarity;
            best_match_index = Some(index);
        }
    }

    best_match_index
}
