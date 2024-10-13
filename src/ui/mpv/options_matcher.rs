pub fn match_video_upscale<'a>(matcher: i32) -> &'a str {
    match matcher {
        0 => "lanczos",
        1 => "bilinear",
        2 => "ewa_lanczos",
        3 => "mitchell",
        4 => "hermite",
        5 => "oversample",
        6 => "linear",
        7 => "ewa_hanning",
        _ => "ewa_lanczossharp",
    }
}

pub fn match_audio_channels<'a>(matcher: i32) -> &'a str {
    match matcher {
        1 => "auto-safe",
        2 => "mono",
        3 => "stereo",
        _ => "auto",
    }
}

pub fn match_sub_border_style<'a>(matcher: i32) -> &'a str {
    match matcher {
        1 => "opaque-box",
        2 => "background-box",
        _ => "outline-and-shadow",
    }
}

pub fn match_hwdec_interop<'a>(matcher: i32) -> &'a str {
    match matcher {
        0 => "no",
        1 => "auto-safe",
        2 => "vaapi",
        _ => "no",
    }
}
