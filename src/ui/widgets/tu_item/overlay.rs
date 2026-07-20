use gtk::prelude::*;

use crate::ui::{
    provider::tu_item::{
        PreferPoster,
        TuItem,
    },
    widgets::{
        picture_loader::{
            ImageSource,
            PictureLoader,
        },
        tu_list_item::imp::PosterType,
    },
};

use super::TuItemBasic;

pub trait TuItemOverlayPrelude {
    fn get_image_type_and_tag(&self, item: &TuItem) -> (&str, Option<String>, String) {
        if self.poster_type_ext() != PosterType::Poster
            && let Some(imag_tags) = item.image_tags()
        {
            match self.poster_type_ext() {
                PosterType::Banner => {
                    if imag_tags.banner().is_some() {
                        return ("Banner", None, item.id());
                    } else if imag_tags.thumb().is_some() {
                        return ("Thumb", None, item.id());
                    } else if imag_tags.backdrop().is_some() {
                        return ("Backdrop", Some(0.to_string()), item.id());
                    }
                }
                PosterType::Backdrop => {
                    if imag_tags.backdrop().is_some() {
                        return ("Backdrop", Some(0.to_string()), item.id());
                    } else if imag_tags.thumb().is_some() {
                        return ("Thumb", None, item.id());
                    }
                }
                _ => {}
            }
        }
        match item.prefer_poster() {
            // Continue Watching, use parent video poster if possible
            PreferPoster::ParentVideo => {
                if let Some(parent_thumb_item_id) = item.parent_thumb_item_id() {
                    ("Thumb", None, parent_thumb_item_id)
                } else if let Some(parent_backdrop_item_id) = item.parent_backdrop_item_id() {
                    ("Backdrop", Some(0.to_string()), parent_backdrop_item_id)
                } else {
                    ("Backdrop", Some(0.to_string()), item.id())
                }
            }
            // Latest, use parent primary image if possible, this is for latest episodes
            PreferPoster::ParentPost
                if let Some(parent_backdrop_item_id) = item.parent_backdrop_item_id() =>
            {
                ("Primary", None, parent_backdrop_item_id)
            }
            _ => {
                if let Some(img_tags) = item.primary_image_item_id() {
                    // use primary image if possible
                    ("Primary", None, img_tags)
                } else if item.image_tags().is_none_or(|t| t.all_none())
                    && let Some(parent_backdrop_item_id) = item.parent_backdrop_item_id()
                {
                    // fallback to parent backdrop if no image tags and parent backdrop exists, this
                    // is for some season items that don't have image tags
                    ("Primary", None, parent_backdrop_item_id)
                } else {
                    // finally fallback to primary image with item id
                    ("Primary", None, item.id())
                }
            }
        }
    }

    fn overlay(&self) -> gtk::Overlay;

    fn poster_type_ext(&self) -> PosterType;

    fn fallback_sources(&self, item: &TuItem, image_type: &str, id: &str) -> Vec<ImageSource> {
        let mut fallbacks = Vec::new();
        if let Some(image_tags) = item.image_tags() {
            let mut candidates: Vec<(&str, Option<String>)> = Vec::new();
            match image_type {
                "Primary" => {
                    if image_tags.thumb().is_some() {
                        candidates.push(("Thumb", None));
                    }
                    if image_tags.backdrop().is_some() {
                        candidates.push(("Backdrop", Some("0".to_string())));
                    }
                    if image_tags.banner().is_some() {
                        candidates.push(("Banner", None));
                    }
                }
                "Thumb" => {
                    if image_tags.primary().is_some() {
                        candidates.push(("Primary", None));
                    }
                    if image_tags.backdrop().is_some() {
                        candidates.push(("Backdrop", Some("0".to_string())));
                    }
                }
                "Backdrop" => {
                    if image_tags.thumb().is_some() {
                        candidates.push(("Thumb", None));
                    }
                    if image_tags.primary().is_some() {
                        candidates.push(("Primary", None));
                    }
                }
                "Banner" => {
                    if image_tags.thumb().is_some() {
                        candidates.push(("Thumb", None));
                    }
                    if image_tags.backdrop().is_some() {
                        candidates.push(("Backdrop", Some("0".to_string())));
                    }
                }
                _ => {}
            }
            for (fallback_type, fallback_tag) in candidates {
                if fallback_type != image_type {
                    fallbacks.push(ImageSource::item(id, fallback_type, fallback_tag));
                }
            }
        }
        fallbacks
    }
}

pub trait TuItemOverlay: TuItemBasic + TuItemOverlayPrelude {
    fn set_picture(&self);
}

impl<T> TuItemOverlay for T
where
    T: TuItemBasic + TuItemOverlayPrelude,
{
    fn set_picture(&self) {
        let item = self.item();
        let (image_type, tag, id) = self.get_image_type_and_tag(&item);
        let overlay = self.overlay();
        let fallbacks = self.fallback_sources(&item, image_type, &id);

        if let Some(picture_loader) = overlay.child().and_downcast::<PictureLoader>() {
            picture_loader.reload_with_fallbacks(&id, image_type, tag, fallbacks);
            return;
        }

        let picture_loader = PictureLoader::new_with_fallbacks(&id, image_type, tag, fallbacks);
        picture_loader.add_css_class("inbox");
        overlay.set_child(Some(&picture_loader));
    }
}
