use crate::client::client::EMBY_CLIENT;
use crate::client::error::UserFacingError;
use crate::client::structs::*;
use crate::ui::models::SETTINGS;
use crate::ui::provider::tu_item::TuItem;
use crate::utils::{get_data_with_cache_else, req_cache, req_cache_single, spawn};
use crate::{fraction, fraction_reset, toast};
use chrono::{Datelike, Local};
use gettextrs::gettext;
use glib::Object;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use gtk::{prelude::*, template_callbacks};

use super::hortu_scrolled::HortuScrolled;
use super::picture_loader::PictureLoader;

mod imp {

    use std::cell::RefCell;

    use glib::subclass::InitializingObject;
    use gtk::prelude::StaticTypeExt;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

    use crate::ui::widgets::hortu_scrolled::HortuScrolled;

    use super::SimpleListItem;
    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsukimi/home.ui")]
    pub struct HomePage {
        #[template_child]
        pub root: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub libsbox: TemplateChild<gtk::Box>,
        #[template_child]
        pub toast: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub hishortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub libhortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub carousel: TemplateChild<adw::Carousel>,
        pub carouset_items: RefCell<Vec<SimpleListItem>>,
        #[template_child]
        pub carouseloverlay: TemplateChild<gtk::Overlay>,
        pub selection: gtk::SingleSelection,
        pub timeout: RefCell<Option<glib::source::SourceId>>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for HomePage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "HomePage";
        type Type = super::HomePage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            HortuScrolled::ensure_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for HomePage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.init_load();
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for HomePage {}

    // Trait shared by all windows
    impl WindowImpl for HomePage {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for HomePage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for HomePage {}
}

glib::wrapper! {
    pub struct HomePage(ObjectSubclass<imp::HomePage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Default for HomePage {
    fn default() -> Self {
        Self::new()
    }
}

#[template_callbacks]
impl HomePage {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn init_load(&self) {
        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                obj.setup(true).await;
                gtk::glib::timeout_future_seconds(1).await;
                obj.setup_history(false).await;
            }
        ));
    }

    pub fn update(&self, enable_cache: bool) {
        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                obj.setup(enable_cache).await;
            }
        ));
    }

    pub async fn setup(&self, enable_cache: bool) {
        fraction_reset!(self);
        self.set_carousel().await;
        self.setup_history(enable_cache).await;
        self.setup_library().await;
        fraction!(self);
    }

    #[template_callback]
    fn carousel_pressed_cb(&self) {
        let position = self.imp().carousel.position();
        if let Some(item) = self.imp().carouset_items.borrow().get(position as usize) {
            let tu_item = TuItem::from_simple(item, None);
            tu_item.activate(self, None);
        }
    }

    pub async fn setup_history(&self, enable_cache: bool) {
        let hortu = self.imp().hishortu.get();

        let results = match req_cache_single(
            "history",
            async { EMBY_CLIENT.get_resume().await },
            enable_cache,
        )
        .await
        {
            Ok(history) => history,
            Err(e) => {
                toast!(self, e.to_user_facing());
                None
            }
        }
        .unwrap_or_default();

        hortu.set_title(&gettext("Continue Watching"));

        hortu.set_items(&results.items);
    }

    pub async fn setup_library(&self) {
        let hortu = self.imp().libhortu.get();

        let results = match req_cache("library", async { EMBY_CLIENT.get_library().await }).await {
            Ok(history) => history,
            Err(e) => {
                toast!(self, e.to_user_facing());
                return;
            }
        };

        let results = results.items;

        hortu.set_title(&gettext("Library"));

        hortu.set_items(&results);

        self.setup_libsview(results).await;
    }

    pub async fn setup_libsview(&self, items: Vec<SimpleListItem>) {
        let libsbox = &self.imp().libsbox;
        for _ in 0..libsbox.observe_children().n_items() {
            libsbox.remove(&libsbox.last_child().unwrap());
        }

        for view in items {
            let ac_view = view.clone();

            let Some(collection_type) = view.collection_type else {
                continue;
            };

            let results = match req_cache(&format!("library_{}", view.id), async move {
                if collection_type == "livetv" {
                    EMBY_CLIENT.get_channels().await.map(|x| x.items)
                } else {
                    EMBY_CLIENT.get_latest(&view.id).await
                }
            })
            .await
            {
                Ok(history) => history,
                Err(e) => {
                    toast!(self, e.to_user_facing());
                    return;
                }
            };

            let hortu = HortuScrolled::new(false);

            hortu.set_moreview(true);

            hortu.set_title(&format!("{} {}", gettext("Latest"), view.name));

            hortu.set_items(&results);

            hortu.connect_morebutton(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |_| {
                    let list_item = TuItem::default();
                    list_item.set_id(ac_view.id.clone());
                    list_item.set_name(ac_view.name.clone());
                    list_item.set_item_type(ac_view.latest_type.clone());
                    list_item.set_collection_type(ac_view.collection_type.clone());
                    list_item.activate(&obj, None);
                }
            ));

            libsbox.append(&hortu);
        }
    }

    pub async fn set_carousel(&self) {
        if !SETTINGS.daily_recommend() {
            self.imp().carouseloverlay.set_visible(false);
            return;
        }

        let carousel = self.imp().carousel.get();
        for _ in 0..carousel.observe_children().n_items() {
            carousel.remove(&carousel.last_child().unwrap());
        }
        self.imp().carouset_items.borrow_mut().clear();

        let date = Local::now();
        let formatted_date = format!("{:04}{:02}{:02}", date.year(), date.month(), date.day());
        let results = match get_data_with_cache_else(formatted_date, "carousel", async {
            EMBY_CLIENT.get_random().await
        })
        .await
        {
            Ok(results) => results,
            Err(e) => {
                toast!(self, e.to_user_facing());
                List::default()
            }
        };

        for result in results.items {
            if let Some(image_tags) = &result.image_tags {
                if let Some(backdrop_image_tags) = &result.backdrop_image_tags {
                    if image_tags.logo.is_some() && !backdrop_image_tags.is_empty() {
                        self.imp().carouset_items.borrow_mut().push(result.clone());
                        self.carousel_add_child(result);
                    }
                }
            }
        }

        let carousel = self.imp().carousel.get();

        if carousel.n_pages() <= 1 {
            return;
        }

        if let Some(timeout) = self.imp().timeout.borrow_mut().take() {
            glib::source::SourceId::remove(timeout);
        }

        let handler_id = glib::timeout_add_seconds_local(7, move || {
            let current_page = carousel.position();
            let n_pages = carousel.n_pages();
            let new_page_position = (current_page + 1. + n_pages as f64) % n_pages as f64;
            carousel.scroll_to(&carousel.nth_page(new_page_position as u32), true);

            glib::ControlFlow::Continue
        });

        self.imp().timeout.replace(Some(handler_id));
    }

    pub fn carousel_add_child(&self, item: SimpleListItem) {
        let imp = self.imp();
        let id = item.id;

        let image = PictureLoader::new(&id, "Backdrop", Some(0.to_string()));
        image.set_halign(gtk::Align::Center);

        let overlay = gtk::Overlay::builder()
            .valign(gtk::Align::Fill)
            .halign(gtk::Align::Center)
            .child(&image)
            .build();

        let logo = super::logo::set_logo(id, "Logo", None);
        logo.set_halign(gtk::Align::End);

        let logobox = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .margin_bottom(10)
            .margin_end(10)
            .height_request(150)
            .valign(gtk::Align::End)
            .halign(gtk::Align::Fill)
            .build();

        logobox.append(&logo);

        overlay.add_overlay(&logobox);

        imp.carousel.append(&overlay);
    }
}
