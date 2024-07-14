use glib::Object;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use super::singlelist::SingleListPage;
mod imp {

    use std::cell::OnceCell;

    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

    use crate::{fraction, fraction_reset};
    use crate::utils::spawn_g_timeout;

    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/list.ui")]
    #[properties(wrapper_type = super::ListPage)]
    pub struct ListPage {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub collectiontype: OnceCell<String>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for ListPage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "ListPage";
        type Type = super::ListPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    #[glib::derived_properties]
    impl ObjectImpl for ListPage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            spawn_g_timeout(glib::clone!(#[weak] obj, async move {
                fraction_reset!(obj);
                obj.set_pages().await;
                fraction!(obj);
            }));
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for ListPage {}

    // Trait shared by all windows
    impl WindowImpl for ListPage {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for ListPage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for ListPage {}
}

glib::wrapper! {
    pub struct ListPage(ObjectSubclass<imp::ListPage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl ListPage {
    pub fn new(id: String, collection_type: String) -> Self {
        Object::builder()
            .property("id", id)
            .property("collectiontype", collection_type)
            .build()
    }

    pub async fn set_pages(&self) {
        let imp = self.imp();
        let id = imp.id.get().unwrap();
        let collection_type = imp.collectiontype.get().unwrap();
        let stack = imp.stack.get();

        let pages = [
            ("all", "All"),
            ("resume", "Resume"),
            ("boxset", "Boxset"),
            ("tags", "Tags"),
            ("genres", "Genres"),
            ("liked", "Liked"),
        ];

        for (name, title) in &pages {
            let page = SingleListPage::new(id.clone(), collection_type.clone(), name, None, false);
            stack.add_titled(&page, Some(name), title);
        }
    }
}
