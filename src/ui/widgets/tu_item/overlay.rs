use gtk::prelude::*;

use crate::ui::{
    provider::tu_item::TuItem,
    widgets::{
        picture_loader::PictureLoader,
        tu_list_item::imp::PosterType,
        utils::*,
    },
};

use super::TuItemBasic;

pub trait TuItemOverlayPrelude {
    fn set_overlay_size(overlay: &gtk::Overlay, width: i32, height: i32) {
        overlay.set_size_request(width, height);
    }

    fn get_image_type_and_tag(&self, item: &TuItem) -> (&str, Option<String>, String) {
        if self.poster_type_ext() != PosterType::Poster {
            if let Some(imag_tags) = item.image_tags() {
                match self.poster_type_ext() {
                    PosterType::Banner => {
                        Self::set_overlay_size(
                            &self.overlay(),
                            TU_ITEM_BANNER_SIZE.0,
                            TU_ITEM_BANNER_SIZE.1,
                        );
                        if imag_tags.banner().is_some() {
                            return ("Banner", None, item.id());
                        } else if imag_tags.thumb().is_some() {
                            return ("Thumb", None, item.id());
                        } else if imag_tags.backdrop().is_some() {
                            return ("Backdrop", Some(0.to_string()), item.id());
                        }
                    }
                    PosterType::Backdrop => {
                        Self::set_overlay_size(
                            &self.overlay(),
                            TU_ITEM_VIDEO_SIZE.0,
                            TU_ITEM_VIDEO_SIZE.1,
                        );
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
        if item.is_resume() {
            Self::set_overlay_size(&self.overlay(), TU_ITEM_VIDEO_SIZE.0, TU_ITEM_VIDEO_SIZE.1);
            if let Some(parent_thumb_item_id) = item.parent_thumb_item_id() {
                ("Thumb", None, parent_thumb_item_id)
            } else if let Some(parent_backdrop_item_id) = item.parent_backdrop_item_id() {
                ("Backdrop", Some(0.to_string()), parent_backdrop_item_id)
            } else {
                ("Backdrop", Some(0.to_string()), item.id())
            }
        } else if let Some(img_tags) = item.primary_image_item_id() {
            ("Primary", None, img_tags)
        } else {
            ("Primary", None, item.id())
        }
    }

    fn overlay(&self) -> gtk::Overlay;

    fn poster_type_ext(&self) -> PosterType;
}

pub trait TuItemOverlay: TuItemBasic + TuItemOverlayPrelude {
    fn set_picture(&self);

    fn set_animated_picture(&self);

    fn set_played(&self);

    fn set_rating(&self);

    fn set_count(&self);

    fn set_folder(&self);
}

impl<T> TuItemOverlay for T
where
    T: TuItemBasic + TuItemOverlayPrelude,
{
    fn set_picture(&self) {
        let item = self.item();
        let (image_type, tag, id) = self.get_image_type_and_tag(&item);
        let picture_loader = PictureLoader::new(&id, image_type, tag);
        self.overlay().set_child(Some(&picture_loader));
    }

    fn set_animated_picture(&self) {
        let item = self.item();
        let (image_type, tag, id) = self.get_image_type_and_tag(&item);
        let picture_loader = PictureLoader::new_animated(&id, image_type, tag);
        self.overlay().set_child(Some(&picture_loader));
    }

    fn set_played(&self) {
        let item = self.item();
        if item.played() {
            let mark = gtk::Button::builder()
                .icon_name("emblem-ok-symbolic")
                .halign(gtk::Align::End)
                .valign(gtk::Align::Start)
                .margin_end(4)
                .margin_top(4)
                .build();
            mark.add_css_class("circular");
            mark.add_css_class("small");
            mark.add_css_class("accent");
            self.overlay().add_overlay(&mark);
        }
    }

    fn set_rating(&self) {
        let item = self.item();
        if let Some(rating) = item.rating() {
            let rating = gtk::Button::builder()
                .label(rating.to_string())
                .halign(gtk::Align::Start)
                .valign(gtk::Align::End)
                .margin_start(8)
                .margin_bottom(8)
                .build();
            rating.add_css_class("pill");
            rating.add_css_class("small");
            rating.add_css_class("suggested-action");
            self.overlay().add_overlay(&rating);
        }
    }

    fn set_count(&self) {
        let item = self.item();
        let count = item.unplayed_item_count();
        if count > 0 {
            let mark = gtk::Button::builder()
                .label(count.to_string())
                .halign(gtk::Align::End)
                .valign(gtk::Align::Start)
                .margin_end(8)
                .margin_top(8)
                .build();
            mark.add_css_class("pill");
            mark.add_css_class("small");
            mark.add_css_class("suggested-action");
            self.overlay().add_overlay(&mark);
        }
    }

    fn set_folder(&self) {
        let mark = gtk::Button::builder()
            .icon_name("folder-symbolic")
            .halign(gtk::Align::End)
            .valign(gtk::Align::Start)
            .margin_top(10)
            .margin_start(10)
            .build();
        mark.add_css_class("pill");
        mark.add_css_class("small");
        mark.add_css_class("suggested-action");
        self.overlay().add_overlay(&mark);
    }
}
