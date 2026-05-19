use adw::prelude::BinExt;
use gtk::prelude::*;

use crate::ui::{
    provider::tu_item::{
        PreferPoster,
        TuItem,
    },
    widgets::{
        hover_scale::HoverScale,
        picture_loader::PictureLoader,
        tu_list_item::imp::PosterType,
    },
};

use super::TuItemBasic;

pub trait TuItemOverlayPrelude {
    fn get_image_type_and_tag(&self, item: &TuItem) -> (&str, Option<String>, String) {
        if self.poster_type_ext() != PosterType::Poster {
            if let Some(imag_tags) = item.image_tags() {
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
}

pub trait TuItemOverlay: TuItemBasic + TuItemOverlayPrelude {
    fn set_picture(&self) -> PictureLoader;

    fn set_picture_with_hover_scale(&self) -> HoverScale;

    fn set_animated_picture(&self) -> PictureLoader;
}

impl<T> TuItemOverlay for T
where
    T: TuItemBasic + TuItemOverlayPrelude,
{
    fn set_picture(&self) -> PictureLoader {
        let item = self.item();
        let (image_type, tag, id) = self.get_image_type_and_tag(&item);
        let picture_loader = PictureLoader::new(&id, image_type, tag);
        picture_loader.add_css_class("inbox");
        self.overlay().set_child(Some(&picture_loader));
        picture_loader
    }

    fn set_picture_with_hover_scale(&self) -> HoverScale {
        let item = self.item();
        let (image_type, tag, id) = self.get_image_type_and_tag(&item);
        let picture_loader = PictureLoader::new(&id, image_type, tag);
        let hover_scale = HoverScale::new();
        hover_scale.set_child(Some(&picture_loader));
        self.overlay().set_child(Some(&hover_scale));
        hover_scale
    }

    fn set_animated_picture(&self) -> PictureLoader {
        let item = self.item();
        let (image_type, tag, id) = self.get_image_type_and_tag(&item);
        let picture_loader = PictureLoader::new_animated(&id, image_type, tag);
        picture_loader.add_css_class("inbox");
        self.overlay().set_child(Some(&picture_loader));
        picture_loader
    }
}
