use gettextrs::gettext;

pub struct LanguageOption {
    pub code: &'static str,
    pub label: &'static str,
}

pub const LANGUAGE_OPTIONS: &[LanguageOption] = &[
    LanguageOption {
        code: "eng",
        label: "English",
    },
    LanguageOption {
        code: "jpn",
        label: "Japanese",
    },
    LanguageOption {
        code: "deu",
        label: "German",
    },
    LanguageOption {
        code: "fra",
        label: "French",
    },
    LanguageOption {
        code: "spa",
        label: "Spanish",
    },
    LanguageOption {
        code: "ita",
        label: "Italian",
    },
    LanguageOption {
        code: "por",
        label: "Portuguese",
    },
    LanguageOption {
        code: "rus",
        label: "Russian",
    },
    LanguageOption {
        code: "kor",
        label: "Korean",
    },
    LanguageOption {
        code: "zho",
        label: "Chinese",
    },
    LanguageOption {
        code: "cmn",
        label: "Mandarin",
    },
    LanguageOption {
        code: "yue",
        label: "Cantonese",
    },
    LanguageOption {
        code: "ara",
        label: "Arabic",
    },
    LanguageOption {
        code: "hin",
        label: "Hindi",
    },
    LanguageOption {
        code: "nld",
        label: "Dutch",
    },
    LanguageOption {
        code: "swe",
        label: "Swedish",
    },
    LanguageOption {
        code: "nor",
        label: "Norwegian",
    },
    LanguageOption {
        code: "dan",
        label: "Danish",
    },
    LanguageOption {
        code: "fin",
        label: "Finnish",
    },
    LanguageOption {
        code: "pol",
        label: "Polish",
    },
    LanguageOption {
        code: "ces",
        label: "Czech",
    },
    LanguageOption {
        code: "hun",
        label: "Hungarian",
    },
    LanguageOption {
        code: "ron",
        label: "Romanian",
    },
    LanguageOption {
        code: "tur",
        label: "Turkish",
    },
    LanguageOption {
        code: "tha",
        label: "Thai",
    },
    LanguageOption {
        code: "vie",
        label: "Vietnamese",
    },
    LanguageOption {
        code: "ind",
        label: "Indonesian",
    },
    LanguageOption {
        code: "msa",
        label: "Malay",
    },
    LanguageOption {
        code: "heb",
        label: "Hebrew",
    },
    LanguageOption {
        code: "ukr",
        label: "Ukrainian",
    },
];

pub fn language_combo_labels() -> Vec<String> {
    LANGUAGE_OPTIONS
        .iter()
        .map(|opt| format!("{} ({})", opt.label, opt.code))
        .collect()
}

pub fn code_at_index(index: u32) -> String {
    LANGUAGE_OPTIONS
        .get(index as usize)
        .map(|opt| opt.code.to_string())
        .unwrap_or_else(|| "eng".to_string())
}

pub fn index_for_code(code: &str) -> u32 {
    LANGUAGE_OPTIONS
        .iter()
        .position(|opt| opt.code.eq_ignore_ascii_case(code))
        .unwrap_or(0) as u32
}

pub fn when_language_combo_labels() -> Vec<String> {
    let mut labels = vec![gettext("Any")];
    labels.extend(language_combo_labels());
    labels
}

pub fn code_at_when_index(index: u32) -> Option<String> {
    if index == 0 {
        return None;
    }
    Some(code_at_index(index - 1))
}

pub fn index_for_when_code(code: Option<&str>) -> u32 {
    match code {
        None | Some("") => 0,
        Some(code) => index_for_code(code) + 1,
    }
}
