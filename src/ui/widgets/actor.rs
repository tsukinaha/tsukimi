use crate::client::client::EMBY_CLIENT;
use crate::client::error::UserFacingError;
use crate::client::{network::*, structs::*};
use crate::{fraction, fraction_reset, toast};
use crate::ui::image::set_image;
use crate::utils::{get_data_with_cache, req_cache, spawn, tu_list_view_connect_activate};
use glib::Object;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use super::tu_list_item::tu_list_item_register;
use crate::ui::widgets::fix::ScrolledWindowFixExt;

mod imp {
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::{glib, CompositeTemplate};
    use std::cell::OnceCell;

    use crate::ui::widgets::hortu_scrolled::HortuScrolled;
    use crate::utils::spawn_g_timeout;
    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/actor.ui")]
    #[properties(wrapper_type = super::ActorPage)]
    pub struct ActorPage {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,
        #[template_child]
        pub actorpicbox: TemplateChild<gtk::Box>,
        #[template_child]
        pub inscription: TemplateChild<gtk::Inscription>,
        #[template_child]
        pub inforevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub title: TemplateChild<gtk::Label>,
        #[template_child]
        pub moviehortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub serieshortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub episodehortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub linksrevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub linksscrolled: TemplateChild<gtk::ScrolledWindow>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for ActorPage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "ActorPage";
        type Type = super::ActorPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            HortuScrolled::ensure_type();
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    #[glib::derived_properties]
    impl ObjectImpl for ActorPage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            spawn_g_timeout(glib::clone!(@weak obj => async move {
                obj.setup_pic();
                obj.get_item().await;
                obj.set_lists().await;
            }));
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for ActorPage {}

    // Trait shared by all windows
    impl WindowImpl for ActorPage {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for ActorPage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for ActorPage {}
}

glib::wrapper! {
    pub struct ActorPage(ObjectSubclass<imp::ActorPage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl ActorPage {
    pub fn new(id: &str) -> Self {
        Object::builder().property("id", id).build()
    }

    pub fn setup_pic(&self) {
        let imp = self.imp();
        let id = self.id();
        let pic = set_image(id, "Primary", None);
        pic.set_size_request(218, 328);
        pic.set_halign(gtk::Align::Start);
        pic.set_valign(gtk::Align::Start);
        imp.actorpicbox.append(&pic);
    }

    pub async fn get_item(&self) {
        let imp = self.imp();
        let id = self.id();
        let inscription = imp.inscription.get();
        let inforevealer = imp.inforevealer.get();
        let title = imp.title.get();
        let item = get_data_with_cache(id.to_string(), "item", async {
            get_item_overview(id).await
        })
        .await
        .unwrap_or_else(|_| {
            toast!(self, "Network Error");
            Item::default()
        });
        spawn(glib::clone!(@weak self as obj=>async move {
                if let Some(overview) = item.overview {
                    inscription.set_text(Some(&overview));
                }
                if let Some(links) = item.external_urls {
                    obj.setlinksscrolled(links);
                }
                title.set_text(&item.name);
                inforevealer.set_reveal_child(true);
        }));
    }

    pub async fn set_lists(&self) {
        fraction_reset!(self);
        self.sets("Movie").await;
        self.sets("Series").await;
        self.sets("Episode").await;
        fraction!(self);
    }

    pub async fn sets(&self, types: &str) {
        let hortu = 
            match types {
                "Movie" => self.imp().moviehortu.clone(),
                "Series" => self.imp().serieshortu.clone(),
                "Episode" => self.imp().episodehortu.clone(),
                _ => return,
            };

        hortu.set_title(&format!("{}", types));

        let types = types.to_string();

        let results = 
            match req_cache(&format!("actor_{}_{}", types, self.id()), 
                async move {
                    EMBY_CLIENT.get_favourite(&types).await
                }
            ).await {
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

    pub fn setlinksscrolled(&self, links: Vec<Urls>) {
        let imp = self.imp();
        let linksscrolled = imp.linksscrolled.fix();
        let linksrevealer = imp.linksrevealer.get();
        if !links.is_empty() {
            linksrevealer.set_reveal_child(true);
        }
        let linkbox = gtk::Box::new(gtk::Orientation::Horizontal, 5);
        linkbox.add_css_class("flat");
        while linkbox.last_child().is_some() {
            if let Some(child) = linkbox.last_child() {
                linkbox.remove(&child)
            }
        }
        for url in links {
            let linkbutton = gtk::Button::builder()
                .margin_start(10)
                .margin_top(10)
                .build();
            let buttoncontent = adw::ButtonContent::builder()
                .label(&url.name)
                .icon_name("send-to-symbolic")
                .build();
            linkbutton.set_child(Some(&buttoncontent));
            linkbutton.connect_clicked(move |_| {
                let _ = gio::AppInfo::launch_default_for_uri(
                    &url.url,
                    Option::<&gio::AppLaunchContext>::None,
                );
            });
            linkbox.append(&linkbutton);
        }
        linksscrolled.set_child(Some(&linkbox));
        linksrevealer.set_reveal_child(true);
    }
}
