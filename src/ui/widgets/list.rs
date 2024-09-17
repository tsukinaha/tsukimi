use gettextrs::gettext;
use glib::Object;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use crate::client::client::EMBY_CLIENT;

use super::single_grid::imp::ListType;
use super::single_grid::SingleGrid;
mod imp {

    use std::cell::OnceCell;

    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

    use crate::utils::spawn_g_timeout;
    use crate::{fraction, fraction_reset};

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
            spawn_g_timeout(glib::clone!(
                #[weak]
                obj,
                async move {
                    fraction_reset!(obj);
                    obj.set_pages().await;
                    fraction!(obj);
                }
            ));
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
        let id = self.id();
        let collection_type = self.collectiontype();
        let stack = imp.stack.get();

        if &collection_type == "livetv" {
            let page = SingleGrid::new();
            page.connect_sort_changed_tokio(false, move |_, _| async move {
                EMBY_CLIENT.get_channels_list(0).await
            });
            page.connect_end_edge_overshot_tokio(false, move |_, _, n_items| async move {
                EMBY_CLIENT.get_channels_list(n_items).await
            });
            page.emit_by_name::<()>("sort-changed", &[]);
            stack.add_titled(&page, Some("channels"), &gettext("Channels"));
            return;
        }

        let include_item_types = get_include_item_types(collection_type);

        let pages = [
            ("all", "All", ListType::All),
            ("resume", "Resume", ListType::Resume),
            ("boxset", "Boxset", ListType::BoxSet),
            ("tags", "Tags", ListType::Tags),
            ("genres", "Genres", ListType::Genres),
            ("liked", "Liked", ListType::Liked),
        ];

        for (name, title, list_type) in pages {
            let page = SingleGrid::new();
            page.set_list_type(list_type);
            page.handle_type();
            let id_clone1 = id.clone();
            let include_item_types_clone1 = include_item_types.clone();
            page.connect_sort_changed_tokio(
                list_type == ListType::Resume,
                move |sort_by, sort_order| {
                    let id_clone1 = id_clone1.clone();
                    let include_item_types_clone1 = include_item_types_clone1.clone();
                    async move {
                        EMBY_CLIENT
                            .get_list(
                                &id_clone1,
                                0,
                                &include_item_types_clone1,
                                list_type,
                                &sort_order,
                                &sort_by,
                            )
                            .await
                    }
                },
            );
            let id_clone2 = id.clone();
            let include_item_types_clone2 = include_item_types.clone();
            page.connect_end_edge_overshot_tokio(
                list_type == ListType::Resume,
                move |sort_by, sort_order, n_items| {
                    let id_clone2 = id_clone2.clone();
                    let include_item_types_clone2 = include_item_types_clone2.clone();
                    async move {
                        EMBY_CLIENT
                            .get_list(
                                &id_clone2,
                                n_items,
                                &include_item_types_clone2,
                                list_type,
                                &sort_order,
                                &sort_by,
                            )
                            .await
                    }
                },
            );
            page.emit_by_name::<()>("sort-changed", &[]);
            stack.add_titled(&page, Some(name), &gettext(title));
        }
    }
}

pub fn get_include_item_types(c: String) -> String {
    let item_type = match c.as_str() {
        "movies" => "Movie",
        "tvshows" => "Series",
        "music" => "MusicAlbum",
        _ => "Movie, Series",
    };
    item_type.to_string()
}
