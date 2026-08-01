use dandanapi_client::CommentData;
use mutsumi::*;

pub trait DanmakuExt {
    fn into_danmaku(self) -> Danmaku;
}

impl DanmakuExt for CommentData {
    fn into_danmaku(self) -> Danmaku {
        let Some(m) = self.m else {
            return Danmaku {
                content: String::new(),
                start: 0.0,
                color: Color {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 0,
                },
                mode: DanmakuMode::Scroll,
            };
        };

        let Some(p) = self.p else {
            return Danmaku {
                content: m,
                start: 0.0,
                color: Color {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
                mode: DanmakuMode::Scroll,
            };
        };

        let parts: Vec<&str> = p.split(',').collect();
        let start = parts
            .first()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or_default();
        let mode = parts
            .get(1)
            .and_then(|s| s.parse::<u8>().ok())
            .unwrap_or_default();
        let color = parts
            .get(2)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or_default();

        Danmaku {
            content: m,
            start: start * 1000.0,
            color: Color {
                r: ((color >> 16) & 0xFF) as u8,
                g: ((color >> 8) & 0xFF) as u8,
                b: (color & 0xFF) as u8,
                a: 255,
            },
            mode: match mode {
                4 => DanmakuMode::BottomCenter,
                5 => DanmakuMode::TopCenter,
                _ => DanmakuMode::Scroll,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_dandanplay_comment_parameters() {
        let top = CommentData {
            cid: None,
            p: Some("12.5,5,16711680,0".to_owned()),
            m: Some("top".to_owned()),
        }
        .into_danmaku();
        assert_eq!(top.content, "top");
        assert_eq!(top.start, 12_500.0);
        assert_eq!(
            (top.color.r, top.color.g, top.color.b, top.color.a),
            (255, 0, 0, 255)
        );
        assert_eq!(top.mode, DanmakuMode::TopCenter);

        let bottom = CommentData {
            cid: None,
            p: Some("1,4,255,0".to_owned()),
            m: Some("bottom".to_owned()),
        }
        .into_danmaku();
        assert_eq!(bottom.mode, DanmakuMode::BottomCenter);
    }
}
