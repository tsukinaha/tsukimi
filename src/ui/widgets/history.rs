use glib::Object;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use crate::client::client::EMBY_CLIENT;
use crate::client::error::UserFacingError;
use crate::client::structs::*;
use crate::utils::spawn_tokio;
use crate::{fraction, fraction_reset, toast};

mod imp {
    use glib::subclass::InitializingObject;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

    use crate::ui::widgets::hortu_scrolled::HortuScrolled;
    use crate::utils::spawn;

    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsukimi/history.ui")]
    pub struct HistoryPage {
        #[template_child]
        pub moviehortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub serieshortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub episodehortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub peoplehortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub albumhortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub boxsethortu: TemplateChild<HortuScrolled>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for HistoryPage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "HistoryPage";
        type Type = super::HistoryPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for HistoryPage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            spawn(glib::clone!(@weak obj =>async move {
                obj.set_lists().await;
            }));
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for HistoryPage {}

    // Trait shared by all windows
    impl WindowImpl for HistoryPage {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for HistoryPage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for HistoryPage {}
}

glib::wrapper! {
    pub struct HistoryPage(ObjectSubclass<imp::HistoryPage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Default for HistoryPage {
    fn default() -> Self {
        Self::new()
    }
}

impl HistoryPage {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub async fn set_lists(&self) {
        fraction_reset!(self);
        self.sets("Movie").await;
        self.sets("Series").await;
        self.sets("Episode").await;
        self.sets("People").await;
        self.sets("MusicAlbum").await;
        self.sets("BoxSet").await;
        fraction!(self);
    }

    pub async fn sets(&self, types: &str) {
        let hortu = match types {
            "Movie" => self.imp().moviehortu.get(),
            "Series" => self.imp().serieshortu.get(),
            "Episode" => self.imp().episodehortu.get(),
            "People" => self.imp().peoplehortu.get(),
            "MusicAlbum" => self.imp().albumhortu.get(),
            "BoxSet" => self.imp().boxsethortu.get(),
            _ => return,
        };

        hortu.set_title(&format!("Favourite {}", types));

        let types = types.to_string();

        let results =
            match spawn_tokio(async move { EMBY_CLIENT.get_favourite(&types).await }).await {
                Ok(history) => history,
                Err(e) => {
                    toast!(self, e.to_user_facing());
                    List::default()
                }
            };

        if results.items.is_empty() {
            hortu.set_visible(false);
            return;
        }

        hortu.set_items(&results.items);
    }
}
