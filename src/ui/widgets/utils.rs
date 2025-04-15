use gtk::{
    SignalListItemFactory,
    prelude::*,
};

use super::{
    filter_panel::FilterPanelDialog,
    identify::IdentifyDialog,
    image_dialog::ImageDialog,
    tu_list_item::{
        TuListItem,
        imp::PosterType,
    },
    tu_overview_item::{
        TuOverviewItem,
        imp::ViewGroup,
    },
};

use crate::ui::provider::tu_object::TuObject;

pub trait TuItemBuildExt {
    fn tu_item(&self, poster: PosterType) -> &Self;
    fn tu_overview_item(&self, view_group: ViewGroup) -> &Self;
}

impl TuItemBuildExt for SignalListItemFactory {
    fn tu_item(&self, poster: PosterType) -> &Self {
        self.connect_setup(move |_, item| {
            let tu_item = TuListItem::default();
            tu_item.set_poster_type(poster);

            let list_item = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");
            list_item.set_child(Some(&tu_item));
            list_item
                .property_expression("item")
                .chain_property::<TuObject>("item")
                .bind(&tu_item, "item", gtk::Widget::NONE);
        });
        self
    }

    fn tu_overview_item(&self, view_group: ViewGroup) -> &Self {
        self.connect_setup(move |_, item| {
            let tu_item = TuOverviewItem::default();
            tu_item.set_view_group(view_group);
            let list_item = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");
            list_item.set_child(Some(&tu_item));
            list_item
                .property_expression("item")
                .chain_property::<TuObject>("item")
                .bind(&tu_item, "item", gtk::Widget::NONE);
        });
        self
    }
}

pub const TU_ITEM_POST_SIZE: (i32, i32) = (167, 260);
pub const TU_ITEM_VIDEO_SIZE: (i32, i32) = (250, 141);
pub const TU_ITEM_SQUARE_SIZE: (i32, i32) = (190, 190);
pub const TU_ITEM_BANNER_SIZE: (i32, i32) = (375, 70);

pub trait GlobalToast {
    fn toast(&self, message: impl Into<String>);

    fn add_toast_inner(&self, toast: adw::Toast);
}

impl<T> GlobalToast for T
where
    T: IsA<gtk::Widget>,
{
    fn toast(&self, message: impl Into<String>) {
        let toast = adw::Toast::builder()
            .timeout(2)
            .use_markup(false)
            .title(message.into())
            .build();
        self.add_toast_inner(toast);
    }

    fn add_toast_inner(&self, toast: adw::Toast) {
        if let Some(dialog) = self
            .ancestor(adw::PreferencesDialog::static_type())
            .and_downcast::<adw::PreferencesDialog>()
        {
            use adw::prelude::PreferencesDialogExt;
            dialog.add_toast(toast);
        } else if let Some(overlay) = self
            .ancestor(adw::ToastOverlay::static_type())
            .and_downcast::<adw::ToastOverlay>()
        {
            overlay.add_toast(toast);
        } else if let Some(dialog) = self
            .ancestor(FilterPanelDialog::static_type())
            .and_downcast::<FilterPanelDialog>()
        {
            dialog.add_toast(toast);
        } else if let Some(dialog) = self
            .ancestor(IdentifyDialog::static_type())
            .and_downcast::<IdentifyDialog>()
        {
            dialog.add_toast(toast);
        } else if let Some(dialog) = self
            .ancestor(ImageDialog::static_type())
            .and_downcast::<ImageDialog>()
        {
            dialog.add_toast(toast);
        } else if let Some(root) = self.root() {
            #[allow(deprecated)]
            if let Some(window) = root.downcast_ref::<adw::PreferencesWindow>() {
                use adw::prelude::PreferencesWindowExt;
                window.add_toast(toast);
            } else if let Some(window) = root.downcast_ref::<crate::Window>() {
                window.add_toast(toast);
            } else {
                panic!("Trying to display a toast when the parent doesn't support it");
            }
        }
    }
}
