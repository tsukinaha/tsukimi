use gettextrs::gettext;
use glib::Object;
use gtk::{
    gio,
    glib,
    subclass::prelude::*,
};

use super::single_grid::{
    SingleGrid,
    imp::ListType,
};
use crate::client::jellyfin_client::JELLYFIN_CLIENT;
mod imp {

    use std::cell::OnceCell;

    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib,
        prelude::*,
        subclass::prelude::*,
    };

    use crate::utils::spawn_g_timeout;

    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/list.ui")]
    #[properties(wrapper_type = super::ListPage)]
    pub struct ListPage {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub collectiontype: OnceCell<String>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ListPage {
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

    #[glib::derived_properties]
    impl ObjectImpl for ListPage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            spawn_g_timeout(glib::clone!(
                #[weak]
                obj,
                async move {
                    obj.set_pages().await;
                }
            ));
        }
    }

    impl WidgetImpl for ListPage {}

    impl WindowImpl for ListPage {}

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
            page.connect_sort_changed_tokio(false, move |_, _, _| async move {
                JELLYFIN_CLIENT.get_channels_list(0).await
            });
            page.connect_end_edge_overshot_tokio(move |_, _, n_items, _| async move {
                JELLYFIN_CLIENT.get_channels_list(n_items).await
            });
            stack.add_titled(&page, Some("channels"), &gettext("Channels"));
            return;
        }

        let include_item_types = get_include_item_types(collection_type);

        let pages = [
            ("all", &gettext("All"), ListType::All),
            ("resume", &gettext("Resume"), ListType::Resume),
            ("boxset", &gettext("BoxSet"), ListType::BoxSet),
            ("tags", &gettext("Tags"), ListType::Tags),
            ("genres", &gettext("Genres"), ListType::Genres),
            ("liked", &gettext("Liked"), ListType::Liked),
            ("folder", &gettext("Folder"), ListType::Folder),
        ];

        for (name, title, list_type) in pages {
            let page = SingleGrid::new();
            page.set_list_type(list_type);
            let id_clone1 = id.to_owned();
            let include_item_types_clone1 = include_item_types.to_owned();
            page.connect_sort_changed_tokio(
                list_type == ListType::Resume,
                move |sort_by, sort_order, filters_list| {
                    let id_clone1 = id_clone1.to_owned();
                    let include_item_types_clone1 = include_item_types_clone1.to_owned();
                    async move {
                        if list_type == ListType::Folder {
                            JELLYFIN_CLIENT
                                .get_folder_include(
                                    &id_clone1,
                                    &sort_by,
                                    &sort_order,
                                    0,
                                    &filters_list,
                                )
                                .await
                        } else {
                            JELLYFIN_CLIENT
                                .get_list(
                                    &id_clone1,
                                    0,
                                    &include_item_types_clone1,
                                    list_type,
                                    &sort_order,
                                    &sort_by,
                                    &filters_list,
                                )
                                .await
                        }
                    }
                },
            );
            let id_clone2 = id.to_owned();
            let include_item_types_clone2 = include_item_types.to_owned();

            if list_type != ListType::Resume {
                page.connect_end_edge_overshot_tokio(
                    move |sort_by, sort_order, n_items, filters_list| {
                        let id_clone2 = id_clone2.to_owned();
                        let include_item_types_clone2 = include_item_types_clone2.to_owned();
                        async move {
                            if list_type == ListType::Folder {
                                JELLYFIN_CLIENT
                                    .get_folder_include(
                                        &id_clone2,
                                        &sort_by,
                                        &sort_order,
                                        n_items,
                                        &filters_list,
                                    )
                                    .await
                            } else {
                                JELLYFIN_CLIENT
                                    .get_list(
                                        &id_clone2,
                                        n_items,
                                        &include_item_types_clone2,
                                        list_type,
                                        &sort_order,
                                        &sort_by,
                                        &filters_list,
                                    )
                                    .await
                            }
                        }
                    },
                );
            };

            stack.add_titled(&page, Some(name), title);
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
