use adw::prelude::*;
use gtk::glib::subclass::types::ObjectSubclassIsExt;

use crate::Window;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainTab {
    Home,
    Liked,
    Search,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushedPageKind {
    Item,
    Grid,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationContext {
    Mpv,
    Placeholder,
    MediaViewer,
    Modal,
    Pushed(PushedPageKind),
    Main(MainTab),
}

impl Window {
    pub fn resolve_navigation_context(&self) -> NavigationContext {
        if self.is_on_mpv_stack_pub() {
            return NavigationContext::Mpv;
        }
        if self.is_on_placeholder() {
            return NavigationContext::Placeholder;
        }
        if self.imp().media_viewer.get().is_revealed() {
            return NavigationContext::MediaViewer;
        }
        if self
            .imp()
            .active_settings
            .borrow()
            .as_ref()
            .is_some_and(|s| s.is_visible())
            || self
                .imp()
                .active_account_dialog
                .borrow()
                .as_ref()
                .is_some_and(|d| d.is_visible())
        {
            return NavigationContext::Modal;
        }

        if self.now_page_tag().as_deref() != Some("mainpage") {
            if let Some(page) = self.imp().mainview.visible_page() {
                if page
                    .downcast_ref::<crate::ui::widgets::item::ItemPage>()
                    .is_some()
                {
                    return NavigationContext::Pushed(PushedPageKind::Item);
                }
                if page
                    .downcast_ref::<crate::ui::widgets::single_grid::SingleGrid>()
                    .is_some()
                {
                    return NavigationContext::Pushed(PushedPageKind::Grid);
                }
            }
            return NavigationContext::Pushed(PushedPageKind::Other);
        }

        match self.imp().insidestack.visible_child_name().as_deref() {
            Some("likedpage") => NavigationContext::Main(MainTab::Liked),
            Some("searchpage") => NavigationContext::Main(MainTab::Search),
            _ => NavigationContext::Main(MainTab::Home),
        }
    }
}
