use std::path::PathBuf;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Object;
use gtk::template_callbacks;
use gtk::{gio, glib};

use crate::client::client::EMBY_CLIENT;
use crate::client::error::UserFacingError;
use crate::client::{network::*, structs::*};
use crate::ui::image::set_image;
use crate::utils::{get_data_with_cache, get_image_with_cache, req_cache, spawn, spawn_tokio};
use crate::{fraction, fraction_reset, toast};

mod imp {
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::{glib, CompositeTemplate};
    use std::cell::OnceCell;

    use crate::ui::widgets::horbu_scrolled::HorbuScrolled;
    use crate::ui::widgets::hortu_scrolled::HortuScrolled;
    use crate::utils::spawn_g_timeout;
    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/boxset.ui")]
    #[properties(wrapper_type = super::BoxSetPage)]
    pub struct BoxSetPage {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,
        #[template_child]
        pub inscription: TemplateChild<gtk::Inscription>,
        #[template_child]
        pub boxset_title: TemplateChild<gtk::Label>,
        #[template_child]
        pub line2: TemplateChild<gtk::Label>,
        #[template_child]
        pub orating: TemplateChild<gtk::Label>,
        #[template_child]
        pub inforevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub favourite_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub picbox: TemplateChild<gtk::Box>,
        #[template_child]
        pub favourite_button_content: TemplateChild<adw::ButtonContent>,
        #[template_child]
        pub linkshorbu: TemplateChild<HorbuScrolled>,
        #[template_child]
        pub inititemhortu: TemplateChild<HortuScrolled>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for BoxSetPage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "BoxSetPage";
        type Type = super::BoxSetPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
            klass.install_action_async("like", None, |window, _action, _parameter| async move {
                window.like().await;
            });
            klass.install_action_async("unlike", None, |window, _action, _parameter| async move {
                window.unlike().await;
            });
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    #[glib::derived_properties]
    impl ObjectImpl for BoxSetPage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            spawn_g_timeout(glib::clone!(@weak obj => async move {
                obj.setup().await;
            }));
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for BoxSetPage {}

    // Trait shared by all windows
    impl WindowImpl for BoxSetPage {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for BoxSetPage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for BoxSetPage {}
}

glib::wrapper! {
    pub struct BoxSetPage(ObjectSubclass<imp::BoxSetPage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

#[template_callbacks]
impl BoxSetPage {
    pub fn new(id: &str) -> Self {
        Object::builder().property("id", id).build()
    }

    pub async fn setup(&self) {
        fraction_reset!(self);
        self.setup_background().await;
        self.setup_pic().await;
        self.setoverview().await;
        self.set_included().await;
        fraction!(self);
    }

    pub async fn setup_pic(&self) {
        let imp = self.imp();
        let id = self.id();
        let pic = set_image(id, "Primary", None);
        pic.set_halign(gtk::Align::Start);
        pic.set_valign(gtk::Align::Start);
        imp.picbox.append(&pic);
    }

    pub async fn like(&self) {
        let imp = self.imp();
        let spilt_button_content = imp.favourite_button_content.get();
        let spilt_button = imp.favourite_button.get();
        imp.favourite_button.set_sensitive(false);
        let id = self.id();
        spawn_tokio(async move {
            like(&id).await.unwrap();
        })
        .await;
        spawn(glib::clone!(@weak self as obj=>async move {
            obj.imp().favourite_button.set_sensitive(true);
            spilt_button.set_action_name(Some("unlike"));
            spilt_button_content.set_icon_name("starred-symbolic");
            spilt_button_content.set_label("Unlike");
            let window = obj.root().and_downcast::<super::window::Window>().unwrap();
            window.toast("Liked the Item successfully.");
        }));
    }

    pub async fn unlike(&self) {
        let imp = self.imp();
        let spilt_button_content = imp.favourite_button_content.get();
        let spilt_button = imp.favourite_button.get();
        imp.favourite_button.set_sensitive(false);
        let id = self.id();
        spawn_tokio(async move {
            unlike(&id).await.unwrap();
        })
        .await;
        spawn(glib::clone!(@weak self as obj=>async move {
            obj.imp().favourite_button.set_sensitive(true);
            spilt_button.set_action_name(Some("like"));
            spilt_button_content.set_icon_name("non-starred-symbolic");
            spilt_button_content.set_label("Like");
            let window = obj.root().and_downcast::<super::window::Window>().unwrap();
            window.toast("Unliked the Item successfully.");
        }));
    }

    pub async fn setup_background(&self) {
        let id = self.id();

        let path = get_image_with_cache(&id, "Backdrop", Some(0))
            .await
            .unwrap_or_else(|_| String::default());
        let file = gtk::gio::File::for_path(&path);
        let pathbuf = PathBuf::from(&path);
        if pathbuf.exists() {
            let window = self.root().and_downcast::<super::window::Window>().unwrap();
            window.set_rootpic(file);
        }
    }

    pub async fn setoverview(&self) {
        let imp = self.imp();
        let id = imp.id.get().unwrap().clone();
        let itemoverview = imp.inscription.get();
        let item = get_data_with_cache(id.clone(), "item", async { get_item_overview(id).await })
            .await
            .unwrap_or_else(|_| {
                toast!(self, "Network Error");
                Item::default()
            });
        spawn(glib::clone!(@weak self as obj=>async move {
                {
                    let mut str = String::new();
                    if let Some(rating) = item.official_rating {
                        let orating = obj.imp().orating.get();
                        orating.set_text(&rating);
                        orating.set_visible(true);
                    }
                    if let Some(genres) = &item.genres {
                        for genre in genres {
                            str.push_str(&genre.name);
                            str.push(',');
                        }
                        str.pop();
                    }
                    obj.imp().line2.get().set_text(&str);
                }
                if let Some(overview) = item.overview {
                    itemoverview.set_text(Some(&overview));
                }
                if let Some(links) = item.external_urls {
                    obj.setlinksscrolled(links);
                }
                if let Some(userdata) = item.user_data {
                    if let Some (is_favourite) = userdata.is_favorite {
                        let imp = obj.imp();
                        if is_favourite {
                            imp.favourite_button.set_action_name(Some("unlike"));
                            imp.favourite_button_content.set_icon_name("starred-symbolic");
                            imp.favourite_button_content.set_label("Unlike");
                        } else {
                            imp.favourite_button.set_action_name(Some("like"));
                            imp.favourite_button_content.set_icon_name("non-starred-symbolic");
                            imp.favourite_button_content.set_label("Like");
                        }
                    }
                }
                obj.imp().boxset_title.set_text(&item.name);
                obj.imp().inforevealer.set_reveal_child(true);
        }));
    }

    pub fn setlinksscrolled(&self, links: Vec<Urls>) {
        let imp = self.imp();

        let linkshorbu = imp.linkshorbu.get();

        linkshorbu.set_title("Links");

        linkshorbu.set_links(&links);
    }

    pub async fn set_included(&self) {
        let imp = self.imp();

        let id = self.id();

        imp.inititemhortu.set_title("Items");

        let results = match req_cache(&format!("boxset_{}", &id), async move {
            EMBY_CLIENT.get_includedby(&id).await
        })
        .await
        {
            Ok(history) => history,
            Err(e) => {
                toast!(self, e.to_user_facing());
                List::default()
            }
        };

        imp.inititemhortu.set_items(&results.items);
    }
}
