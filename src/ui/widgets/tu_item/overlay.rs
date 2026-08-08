use gst::glib::subclass::types::ObjectSubclassIsExt;
use gtk::{
    glib,
    prelude::*,
};

use crate::{
    client::{
        picture_source::PictureSource,
        structs::SimpleListItem,
    },
    ui::{
        provider::tu_item::{
            TuItem,
            image_type::{
                BACKDROP,
                BANNER,
                PRIMARY,
                THUMB,
            },
            item_type::{
                EPISODE,
                SEASON,
            },
        },
        widgets::{
            picture_loader::PictureLoader,
            utils::{
                TU_ITEM_BANNER_SIZE,
                TU_ITEM_POST_SIZE,
                TU_ITEM_SQUARE_SIZE,
                TU_ITEM_VIDEO_SIZE,
            },
        },
    },
};

use super::TuItemBasic;

#[derive(Default, Hash, Eq, PartialEq, Clone, Copy, glib::Enum, Debug)]
#[repr(u32)]
#[enum_type(name = "CardShape")]
pub enum CardShape {
    #[default]
    Auto,
    Backdrop,
    Banner,
    Portrait,
    Square,
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct CardOptions {
    pub shape: CardShape,
    pub prefer_thumb: bool,
    pub prefer_parent_poster: bool,
}

impl CardShape {
    pub fn size(self) -> (i32, i32) {
        match self {
            Self::Auto => unreachable!(),
            Self::Backdrop => TU_ITEM_VIDEO_SIZE,
            Self::Banner => TU_ITEM_BANNER_SIZE,
            Self::Portrait => TU_ITEM_POST_SIZE,
            Self::Square => TU_ITEM_SQUARE_SIZE,
        }
    }

    fn is_wide(self) -> bool {
        matches!(self, Self::Backdrop | Self::Banner)
    }

    pub fn resolve(self, items: &[SimpleListItem]) -> Self {
        if self != Self::Auto {
            return self;
        }
        let mut ratios = items
            .iter()
            .filter_map(|item| item.primary_image_aspect_ratio)
            .filter(|ratio| *ratio != 0.0)
            .collect::<Vec<_>>();

        ratios.sort_by(f64::total_cmp);
        let ratio = match ratios.len() {
            0 => return Self::Square,
            n if n % 2 == 1 => ratios[n / 2],
            n => (ratios[n / 2 - 1] + ratios[n / 2]) / 2.0,
        };

        let ratio = if (2.0 / 3.0 - ratio).abs() <= 0.15 {
            2.0 / 3.0
        } else if (16.0 / 9.0 - ratio).abs() <= 0.2 {
            16.0 / 9.0
        } else if (1.0 - ratio).abs() <= 0.15 {
            1.0
        } else if (4.0 / 3.0 - ratio).abs() <= 0.15 {
            4.0 / 3.0
        } else {
            ratio
        };

        if ratio >= 3.0 {
            Self::Banner
        } else if ratio >= 1.33 {
            Self::Backdrop
        } else if ratio > 0.8 {
            Self::Square
        } else {
            Self::Portrait
        }
    }
}

fn tagged_source(
    id: Option<String>, tag: Option<String>, image_type: &str, image_index: Option<u8>,
) -> Option<PictureSource> {
    let (id, tag) = (id?, tag?);
    Some(PictureSource::Item {
        id,
        tag,
        image_type: image_type.to_string(),
        image_index,
    })
}

pub fn select_backdrop_picture_source(item: &TuItem) -> Option<PictureSource> {
    if item.item_type() == EPISODE {
        parent_backdrop_source(item).or_else(|| current_source(item, BACKDROP, Some(0)))
    } else {
        current_source(item, BACKDROP, Some(0))
    }
}

fn current_source(
    item: &TuItem, image_type: &str, image_index: Option<u8>,
) -> Option<PictureSource> {
    let image_tags = item.image_tags()?;
    let tag = match image_type {
        PRIMARY => image_tags.primary(),
        THUMB => image_tags.thumb(),
        BACKDROP => image_tags.backdrop(),
        BANNER => image_tags.banner(),
        _ => None,
    };
    tagged_source(Some(item.id()), tag, image_type, image_index)
}

fn parent_thumb_source(item: &TuItem) -> Option<PictureSource> {
    tagged_source(
        item.parent_thumb_item_id(),
        item.parent_thumb_image_tag(),
        THUMB,
        None,
    )
}

fn parent_backdrop_source(item: &TuItem) -> Option<PictureSource> {
    tagged_source(
        item.parent_backdrop_item_id(),
        item.parent_backdrop_image_tag(),
        BACKDROP,
        Some(0),
    )
}

fn primary_image_source(item: &TuItem) -> Option<PictureSource> {
    tagged_source(
        Some(item.primary_image_item_id().unwrap_or_else(|| item.id())),
        item.primary_image_tag(),
        PRIMARY,
        None,
    )
}

fn parent_primary_source(item: &TuItem) -> Option<PictureSource> {
    tagged_source(
        item.parent_primary_image_item_id(),
        item.parent_primary_image_tag(),
        PRIMARY,
        None,
    )
}

fn series_primary_source(item: &TuItem) -> Option<PictureSource> {
    tagged_source(
        item.series_id(),
        item.series_primary_image_tag(),
        PRIMARY,
        None,
    )
}

fn series_thumb_source(item: &TuItem) -> Option<PictureSource> {
    tagged_source(
        item.series_id(),
        item.series_thumb_image_tag(),
        THUMB,
        None,
    )
}

fn album_primary_source(item: &TuItem) -> Option<PictureSource> {
    tagged_source(
        item.album_id(),
        item.album_primary_image_tag(),
        PRIMARY,
        None,
    )
}

pub fn select_picture_source(item: &TuItem, options: CardOptions) -> Option<PictureSource> {
    if let Some(url) = item.image_url().filter(|url| !url.trim().is_empty()) {
        return Some(PictureSource::Url { url });
    }

    let CardOptions {
        shape: card_shape,
        prefer_thumb,
        prefer_parent_poster,
    } = options;

    if prefer_thumb && let Some(source) = current_source(item, THUMB, None) {
        return Some(source);
    }

    if card_shape == CardShape::Banner
        && let Some(source) = current_source(item, BANNER, None)
    {
        return Some(source);
    }

    if prefer_thumb && let Some(source) = series_thumb_source(item) {
        return Some(source);
    }

    if prefer_thumb && let Some(source) = parent_thumb_source(item) {
        return Some(source);
    }

    if prefer_thumb && let Some(source) = current_source(item, BACKDROP, Some(0)) {
        return Some(source);
    }

    if prefer_thumb
        && item.item_type() == EPISODE
        && let Some(source) = parent_backdrop_source(item)
    {
        return Some(source);
    }

    if prefer_parent_poster
        && !prefer_thumb
        && item.item_type() == EPISODE
        && let Some(source) = parent_primary_source(item).or_else(|| series_primary_source(item))
    {
        return Some(source);
    }

    if (item.item_type() != EPISODE || item.imp().child_count.get() != Some(0))
        && let Some(source) = current_source(item, PRIMARY, None)
    {
        return Some(source);
    }

    let skip_episode_parent_poster = item.item_type() == EPISODE && card_shape.is_wide();

    if !skip_episode_parent_poster && let Some(source) = series_primary_source(item) {
        return Some(source);
    }

    if let Some(source) = primary_image_source(item) {
        return Some(source);
    }

    if !skip_episode_parent_poster && let Some(source) = parent_primary_source(item) {
        return Some(source);
    }

    if let Some(source) = album_primary_source(item) {
        return Some(source);
    }

    if item.item_type() == SEASON
        && let Some(source) = current_source(item, THUMB, None)
    {
        return Some(source);
    }

    current_source(item, BACKDROP, Some(0))
        .or_else(|| current_source(item, THUMB, None))
        .or_else(|| series_thumb_source(item))
        .or_else(|| parent_thumb_source(item))
        .or_else(|| parent_backdrop_source(item))
}

pub trait TuItemOverlayPrelude {
    fn get_image_source(&self, item: &TuItem) -> Option<PictureSource> {
        select_picture_source(item, self.card_options_ext(item))
    }

    fn overlay(&self) -> gtk::Overlay;

    fn card_options_ext(&self, item: &TuItem) -> CardOptions;
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
        let overlay = self.overlay();

        let Some(source) = self.get_image_source(&item) else {
            return;
        };

        if let Some(picture_loader) = overlay.child().and_downcast::<PictureLoader>() {
            picture_loader.reload_source(source);
            return;
        }

        let picture_loader = PictureLoader::new_for_source(source);
        picture_loader.add_css_class("inbox");
        overlay.set_child(Some(&picture_loader));
    }
}
