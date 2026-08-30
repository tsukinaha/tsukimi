use mutsumi::{
    Color,
    Danmaku,
    DanmakuMode,
};
use quick_xml::{
    Reader,
    events::Event,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("XML parsing error: {0}")]
    Xml(#[from] quick_xml::Error),
    #[error("Attribute parsing error: {0}")]
    Attribute(#[from] quick_xml::events::attributes::AttrError),
    #[error("Invalid attribute format for 'p': {0}")]
    InvalidPFormat(String),
    #[error("Failed to parse float value: {0}")]
    ParseFloat(#[from] std::num::ParseFloatError),
    #[error("Failed to parse integer value: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("UTF-8 decoding error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
}

pub fn parse_bilibili_xml(xml_content: &str) -> Result<Vec<Danmaku>, ParseError> {
    let mut reader = Reader::from_str(xml_content);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut danmakus = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) if e.name().as_ref() == b"d" => {
                let p_attr_result = e
                    .attributes()
                    .find(|attr| attr.as_ref().is_ok_and(|a| a.key.as_ref() == b"p"))
                    .ok_or_else(|| ParseError::InvalidPFormat("Missing 'p' attribute".to_string()));

                let p_attr = match p_attr_result {
                    Ok(Ok(attr)) => attr,
                    Ok(Err(e)) => return Err(ParseError::Attribute(e)),
                    Err(e) => return Err(e),
                };

                let p_value_cow = p_attr.normalized_value(quick_xml::XmlVersion::Explicit1_1)?;
                let p_value = p_value_cow.as_ref();

                let parts: Vec<&str> = p_value.split(',').collect();
                if parts.len() < 8 {
                    eprintln!("Skipping invalid 'p' attribute format: {p_value}");
                    reader.read_to_end(e.name())?;
                    buf.clear();
                    continue;
                }

                let start: f64 = parts[0].parse()?;
                let mode_val: u8 = parts[1].parse()?;
                // parts[2] Font Size
                let color_val: u32 = parts[3].parse()?;
                // parts[4] Unix Timestamp
                // parts[5] Danmaku Pool ID
                // parts[6] User ID Hash
                // parts[7] Danmaku ID

                let mode = match mode_val {
                    1 => DanmakuMode::Scroll,
                    4 => DanmakuMode::BottomCenter,
                    5 => DanmakuMode::TopCenter,
                    _ => {
                        eprintln!("Unknown danmaku mode {mode_val}, defaulting to Scroll");
                        DanmakuMode::Scroll
                    }
                };

                let r = ((color_val >> 16) & 0xFF) as u8;
                let g = ((color_val >> 8) & 0xFF) as u8;
                let b = (color_val & 0xFF) as u8;
                let color = Color { r, g, b, a: 255 };

                let content = match reader.read_text(e.name()) {
                    Ok(text) => {
                        let raw = std::str::from_utf8(&text)?;
                        quick_xml::escape::unescape(raw)
                            .map_err(quick_xml::Error::from)?
                            .into_owned()
                    }
                    Err(err) => return Err(ParseError::Xml(err)),
                };

                danmakus.push(Danmaku {
                    content,
                    start: start * 1000.0,
                    color,
                    mode,
                });
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(ParseError::Xml(e)),
            _ => {}
        }

        buf.clear();
    }

    Ok(danmakus)
}
