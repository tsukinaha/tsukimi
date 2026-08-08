use gtk::glib;

#[derive(Clone, glib::Boxed)]
#[boxed_type(name = "TsukimiPictureSource", nullable)]
pub enum PictureSource {
    Item {
        id: String,
        tag: String,
        image_type: String,
        image_index: Option<u8>,
    },
    User {
        id: String,
        tag: String,
    },
    Url {
        image_type: String,
        url: String,
    },
}

impl PictureSource {
    pub fn cache_key(&self) -> String {
        match self {
            Self::Item {
                id,
                tag,
                image_type,
                image_index,
            } => format!("{}-{}-{}-{}", id, image_type, image_index.unwrap_or(0), tag),
            Self::User { id, tag } => format!("{id}-Primary-0-{tag}"),
            Self::Url { .. } => unreachable!(),
        }
    }
}
