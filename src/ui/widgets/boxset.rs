use std::path::PathBuf;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Object;
use gtk::template_callbacks;
use gtk::{gio, glib};

use crate::client::{network::*, structs::*};
use crate::toast;
use crate::ui::image::set_image;
use crate::utils::{
    get_data_with_cache, get_image_with_cache, spawn, spawn_tokio, tu_list_item_factory,
    tu_list_view_connect_activate,
};

use super::fix::ScrolledWindowFixExt;
use super::included::IncludedDialog;
use super::window::Window;
mod imp {
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::{glib, CompositeTemplate};
    use std::cell::OnceCell;

    use crate::utils::spawn_g_timeout;
    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/boxset.ui")]
    #[properties(wrapper_type = super::BoxSetPage)]
    pub struct BoxSetPage {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,
        #[template_child]
        pub linksscrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub linksrevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub inscription: TemplateChild<gtk::Inscription>,
        #[template_child]
        pub initemlist: TemplateChild<gtk::ListView>,
        #[template_child]
        pub initemrevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub initemscrolled: TemplateChild<gtk::ScrolledWindow>,
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
        pub spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub favourite_button_content: TemplateChild<adw::ButtonContent>,
        pub selection: gtk::SingleSelection,
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
                obj.setup_background().await;
                obj.setup_pic().await;
                obj.setoverview().await;
                obj.set_included().await;
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

    #[template_callback]
    pub fn include_button_cb(&self) {
        let id = self.id();
        let dialog = IncludedDialog::new(&id);
        dialog.present(self);
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
            .unwrap_or_else(|e| {
                toast!(self,"Network Error");
                String::default()
            });
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
            .unwrap_or_else(|e| {
                toast!(self,"Network Error");
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
                obj.imp().spinner.set_visible(false);
        }));
    }

    pub fn setlinksscrolled(&self, links: Vec<Urls>) {
        let imp = self.imp();
        let linksscrolled = imp.linksscrolled.fix();
        let linksrevealer = imp.linksrevealer.get();
        if !links.is_empty() {
            linksrevealer.set_reveal_child(true);
        }
        let linkbox = gtk::Box::new(gtk::Orientation::Horizontal, 5);
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

    pub async fn set_included(&self) {
        let imp = self.imp();
        let factory = tu_list_item_factory("".to_string());
        imp.initemlist.set_factory(Some(&factory));
        let store = gtk::gio::ListStore::new::<glib::BoxedAnyObject>();
        imp.selection.set_model(Some(&store));
        imp.initemlist.set_model(Some(&imp.selection));
        imp.initemlist.connect_activate(
            glib::clone!(@weak self as obj => move |gridview, position| {
                let model = gridview.model().unwrap();
                let item = model.item(position).and_downcast::<glib::BoxedAnyObject>().unwrap();
                let result: std::cell::Ref<SimpleListItem> = item.borrow();
                let window = obj.root().and_downcast::<Window>().unwrap();
                tu_list_view_connect_activate(window, &result, None);
            }),
        );
        imp.initemscrolled.fix();
        let store = self
            .imp()
            .selection
            .model()
            .unwrap()
            .downcast::<gtk::gio::ListStore>()
            .unwrap();
        let boxset_list = self.get_included().await;
        spawn(glib::clone!(@weak store,@weak self as obj=> async move {
            obj.imp().initemrevealer.set_reveal_child(true);
            for result in boxset_list {
                let object = glib::BoxedAnyObject::new(result);
                store.append(&object);
                gtk::glib::timeout_future(std::time::Duration::from_millis(30)).await;
            }
        }));
    }

    pub async fn get_included(&self) -> Vec<SimpleListItem> {
        let imp = self.imp();
        let id = imp.id.get().unwrap().clone();
        let list = get_data_with_cache(id.to_string(), "include", async move {
            get_includedby(&id).await
        })
        .await
        .unwrap();
        list.items
    }
}
