use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use crate::client::client::EMBY_CLIENT;
use crate::client::structs::SimpleListItem;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::ui::provider::tu_item::{self, TuItem};
use crate::ui::provider::tu_object::TuObject;
use crate::utils::{spawn, spawn_tokio};
use gtk::gdk::Texture;
use gtk::glib::{self, clone};
use gtk::{prelude::*, IconPaintable, Revealer};
use tracing::{debug, warn};
use crate::bing_song_model;
use crate::ui::models::emby_cache_path;
use crate::ui::provider::core_song::CoreSong;
use crate::ui::widgets::song_widget::State;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::gio;
use gtk::gio::ListStore;
use gtk::{ template_callbacks, CompositeTemplate};

use super::song_widget::format_duration;

pub(crate) mod imp {
    use std::cell::{OnceCell, RefCell};
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    use crate::ui::provider::tu_object::TuObject;
    use crate::ui::widgets::item_actionbox::ItemActionsBox;
    use crate::ui::widgets::utils::TuItemBuildExt;
    use crate::{
        ui::{
            provider::tu_item::TuItem,
            widgets::{hortu_scrolled::HortuScrolled, star_toggle::StarToggle},
        },
        utils::spawn_g_timeout,
    };

    use super::*;
    use glib::subclass::InitializingObject;
    use glib::SignalHandlerId;
    use gtk::SignalListItemFactory;

    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/tuview_scrolled.ui")]
    #[properties(wrapper_type = super::TuViewScrolled)]
    pub struct TuViewScrolled {
        #[template_child]
        pub scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub grid: TemplateChild<gtk::GridView>,
        pub selection: gtk::SingleSelection,
        pub lock: Arc<AtomicBool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TuViewScrolled {
        const NAME: &'static str = "TuViewScrolled";
        type Type = super::TuViewScrolled;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for TuViewScrolled {
        fn constructed(&self) {
            self.parent_constructed();
            let store = gio::ListStore::new::<TuObject>();
            self.selection.set_model(Some(&store));
            let factory = SignalListItemFactory::new();
            self.grid.set_factory(Some(factory.tu_item()));
            self.grid.set_model(Some(&self.selection));
        }
    }

    impl WidgetImpl for TuViewScrolled {}
    impl BinImpl for TuViewScrolled {}
}

glib::wrapper! {
    pub struct TuViewScrolled(ObjectSubclass<imp::TuViewScrolled>)
        @extends gtk::Widget, @implements gtk::Accessible;
}

#[template_callbacks]
impl TuViewScrolled {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_grid<const C: bool>(&self, items: Vec<SimpleListItem>) {
        let imp = self.imp();
        let store = imp
            .selection
            .model()
            .unwrap()
            .downcast::<gio::ListStore>()
            .unwrap();

        if C {
            store.remove_all();
        }

        for item in items {
            let tu_item = TuItem::from_simple(&item, None);
            let tu_item = TuObject::new(&tu_item);
            store.append(&tu_item);
        }
    }



    #[template_callback]
    fn on_item_activated(&self, position: u32, listview: &gtk::GridView) {
        let model = listview.model().unwrap();
        let tu_obj = model
            .item(position)
            .and_downcast::<TuObject>()
            .unwrap();
        tu_obj.activate(listview);
    }

    pub fn connect_end_edge_reached<F>(&self, cb: F)
    where
        F: Fn(&Self, Arc<AtomicBool>) + 'static,
    {
        self.imp().scrolled_window.connect_edge_overshot(clone!(
            #[weak(rename_to = obj)]
            self,
            move |_scrolled, pos|
            if pos == gtk::PositionType::Bottom {
                let is_running = Arc::clone(&obj.imp().lock);

                if is_running
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_err()
                {
                    return;
                }

                cb(&obj, is_running);
            }
        ));
    }

    pub fn n_items(&self) -> u32 {
        let imp = self.imp();
        let store = imp.selection.model().unwrap().downcast::<gio::ListStore>().unwrap();
        store.n_items()
    }
}
