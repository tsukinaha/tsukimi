
use crate::client::client::EMBY_CLIENT;
use crate::client::error::UserFacingError;
use crate::client::{network::*, structs::*};
use crate::ui::image::set_image;
use crate::ui::models::SETTINGS;
use crate::utils::{
    get_data_with_cache_else, req_cache, tu_list_view_connect_activate
};
use crate::toast;
use chrono::{Datelike, Local};
use glib::Object;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use gtk::{prelude::*, template_callbacks};

use super::hortu_scrolled::HortuScrolled;

mod imp {

    use std::cell::RefCell;

    use glib::subclass::InitializingObject;
    use gst::glib::types::StaticTypeExt;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

    use crate::ui::widgets::hortu_scrolled::HortuScrolled;
    use crate::utils::spawn_g_timeout;

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
            spawn_g_timeout(glib::clone!(@weak obj => async move {
                obj.set_carousel().await;
                obj.setup_history().await;
                obj.setup_library().await;
            }));
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

    #[template_callback]
    fn carousel_pressed_cb(&self) {
        let position = self.imp().carousel.position();
        if let Some(item) = self.imp().carouset_items.borrow().get(position as usize) {
            let window = self.root().and_downcast::<super::window::Window>().unwrap();
            tu_list_view_connect_activate(window, item, None)
        }
    }

    pub async fn setup_history(&self) {
        let hortu = self.imp().hishortu.get();

        let results = 
            match req_cache("history", 
                async {
                    EMBY_CLIENT.get_resume().await
                }
            ).await {
                Ok(history) => history,
                Err(e) => {
                    toast!(self, e.to_user_facing());
                    List::default()
                }
            };

        hortu.set_title("Continue Watching");

        hortu.set_items(&results.items);
    }

    pub async fn setup_library(&self) {
        let hortu = self.imp().libhortu.get();

        let results = 
            match req_cache("library", 
                async {
                    EMBY_CLIENT.get_library().await
                }
            ).await {
                Ok(history) => history.items,
                Err(e) => {
                    toast!(self, e.to_user_facing());
                    Vec::new()
                }
            };

        hortu.set_title("Library");

        hortu.set_items(&results);

        self.setup_libsview(results).await;
    }

    pub async fn setup_libsview(&self, items: Vec<SimpleListItem>) {
        for view in items {

            let results = 
            match req_cache(&format!("library_{}", view.id), 
                async move {
                    EMBY_CLIENT.get_latest(&view.id).await
                }
            ).await {
                Ok(history) => history,
                Err(e) => {
                    toast!(self, e.to_user_facing());
                    Vec::new()
                }
            };

            let hortu = HortuScrolled::new(false);

            hortu.set_title(&format!("{} - Latest", view.name));

            hortu.set_items(&results);

            self.imp().libsbox.append(&hortu);
        }
    }

    pub async fn set_carousel(&self) {
        if !SETTINGS.daily_recommend() {
            self.imp().carouseloverlay.set_visible(false);
            return;
        }

        let date = Local::now();
        let formatted_date = format!("{:04}{:02}{:02}", date.year(), date.month(), date.day());
        let results =
            match get_data_with_cache_else(formatted_date, "carousel", async { get_random().await })
                .await {
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

        glib::timeout_add_seconds_local(7, move || {
            let current_page = carousel.position();
            let n_pages = carousel.n_pages();
            let new_page_position = (current_page + 1. + n_pages as f64) % n_pages as f64;
            carousel.scroll_to(&carousel.nth_page(new_page_position as u32), true);

            glib::ControlFlow::Continue
        });
    }

    pub fn carousel_add_child(&self, item: SimpleListItem) {
        let imp = self.imp();
        let id = item.id;

        let image = set_image(id.clone(), "Backdrop", Some(0));
        image.set_halign(gtk::Align::Center);

        let overlay = gtk::Overlay::builder()
            .valign(gtk::Align::Fill)
            .halign(gtk::Align::Center)
            .child(&image)
            .build();

        let logo = set_image(id, "Logo", None);
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
