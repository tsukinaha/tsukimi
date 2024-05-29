use adw::{prelude::*, subclass::prelude::*};
use gtk::{gio, glib, CompositeTemplate};

use crate::client::structs::{Items, List, SimpleListItem};
use crate::ui::provider::tu_item::TuItem;
use crate::utils::spawn;
use crate::{config::Account, ui::provider::account_item::AccountItem};
use crate::ui::widgets::fix::ScrolledWindowFixExt;
use glib::Object;

use super::tu_list_item::TuListItem;

mod imp {
    use std::cell::OnceCell;

    use glib::subclass::InitializingObject;
    use gtk::gio;

    use crate::{client::structs::SimpleListItem, ui::widgets::{singlelist::SingleListPage, window::Window}, utils::{tu_list_item_factory, tu_list_view_connect_activate}};

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsukimi/hortu_scrolled.ui")]
    #[properties(wrapper_type = super::HortuScrolled)]
    pub struct HortuScrolled {
        #[property(get, set, construct_only)]
        pub isresume: OnceCell<bool>,
        #[template_child]
        pub label: TemplateChild<gtk::Label>,
        #[template_child]
        pub scrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub list: TemplateChild<gtk::ListView>,
        #[template_child]
        pub revealer: TemplateChild<gtk::Revealer>,

        pub selection: gtk::SingleSelection,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HortuScrolled {
        const NAME: &'static str = "HortuScrolled";
        type Type = super::HortuScrolled;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for HortuScrolled {
        fn constructed(&self) {
            self.parent_constructed();
            self.scrolled.fix();

            let store = gio::ListStore::new::<glib::BoxedAnyObject>();

            self.selection.set_model(Some(&store));

            self.list.set_model(Some(&self.selection));

            self.list.set_factory(Some(&self.factory(*self.isresume.get().unwrap_or(&false))));
        
            self.list.connect_activate(
                glib::clone!(@weak self as imp => move |listview, position| {
                    let window = imp.obj().root().and_downcast::<Window>().unwrap();
                    let model = listview.model().unwrap();
                    let item = model.item(position).and_downcast::<glib::BoxedAnyObject>().unwrap();
                    let result: std::cell::Ref<SimpleListItem> = item.borrow();
                    imp.activate(window, &result, None);
                }),
            );
        }
    }

    impl WidgetImpl for HortuScrolled {}

    impl BinImpl for HortuScrolled {}

    impl HortuScrolled {
        fn factory(&self, is_resume: bool) -> gtk::SignalListItemFactory {
            let factory = gtk::SignalListItemFactory::new();
            factory.connect_bind(move |_, item| {
                let list_item = item
                    .downcast_ref::<gtk::ListItem>()
                    .expect("Needs to be ListItem");
                let entry = item
                    .downcast_ref::<gtk::ListItem>()
                    .expect("Needs to be ListItem")
                    .item()
                    .and_downcast::<glib::BoxedAnyObject>()
                    .expect("Needs to be BoxedAnyObject");
                let item: std::cell::Ref<SimpleListItem> = entry.borrow();
                if list_item.child().is_none() {
                    let tu_item = TuItem::from_simple(&item, None);
                    match item.latest_type.as_str() {
                        "Movie" | "Series" | "Episode" | "MusicAlbum" | "BoxSet" | "Tag" | "Genre" | "Views"
                        | "Actor" | "Person" | "CollectionFolder" => {
                            let list_child = TuListItem::new(tu_item, &item.latest_type, is_resume);
                            list_item.set_child(Some(&list_child));
                        }
                        _ => {}
                    }
                }
            });
            factory
        }
    
        fn activate(
            &self,
            window: crate::ui::widgets::window::Window,
            result: &SimpleListItem,
            parentid: Option<String>,
        ) {
            let (view, title_var) = match window.current_view_name().as_str() {
                "homepage" => (&window.imp().homeview, "HOME_TITLE"),
                "searchpage" => (&window.imp().searchview, "SEARCH_TITLE"),
                "historypage" => (&window.imp().historyview, "HISTORY_TITLE"),
                _ => (&window.imp().searchview, "SEARCH_TITLE"),
            };
            window.set_title(&result.name);
            std::env::set_var(title_var, &result.name);
        
            match result.latest_type.as_str() {
                "Movie" => Self::push_page(
                    view,
                    &window,
                    &result.name,
                    crate::ui::widgets::movie::MoviePage::new(result.id.clone(), result.name.clone()),
                ),
                "Series" => Self::push_page(
                    view,
                    &window,
                    &result.name,
                    crate::ui::widgets::item::ItemPage::new(
                        result.id.clone(),
                        result.id.clone(),
                        result.name.clone(),
                    ),
                ),
                "Episode" => Self::push_page(
                    view,
                    &window,
                    &result.name,
                    crate::ui::widgets::item::ItemPage::new(
                        result.series_id.as_ref().unwrap().clone(),
                        result.id.clone(),
                        result.series_name.clone().unwrap_or("".to_string()),
                    ),
                ),
                "Actor" | "Person" => Self::push_page(
                    view,
                    &window,
                    &result.name,
                    crate::ui::widgets::actor::ActorPage::new(&result.id),
                ),
                "BoxSet" => Self::push_page(
                    view,
                    &window,
                    &result.name,
                    crate::ui::widgets::boxset::BoxSetPage::new(&result.id),
                ),
                "MusicAlbum" => {
                    let item = TuItem::from_simple(result, None);
                    Self::push_page(
                        view,
                        &window,
                        &result.name,
                        crate::ui::widgets::music_album::AlbumPage::new(item),
                    )
                }
                "CollectionFolder" => {
                    let item = TuItem::from_simple(result, None);
                    Self::push_page(
                        view,
                        &window,
                        &result.name,
                        crate::ui::widgets::list::ListPage::new(item.id(), item.collection_type().unwrap()),
                    )
                }
                _ => Self::push_page(
                    view,
                    &window,
                    &result.name,
                    SingleListPage::new(
                        result.id.clone(),
                        "".to_string(),
                        &result.latest_type,
                        parentid,
                    ),
                ),
            }
        }

        fn push_page<T: 'static + Clone + gtk::prelude::IsA<adw::NavigationPage>>(
            view: &adw::NavigationView,
            window: &crate::ui::widgets::window::Window,
            tag: &str,
            page: T,
        ) {
            if view.find_page(tag).is_some() {
                view.pop_to_tag(tag);
            } else {
                let item_page = page;
                item_page.set_tag(Some(tag));
                view.push(&item_page);
                window.set_pop_visibility(true);
            }
        }
    }
}

glib::wrapper! {
    /// A scrolled list of items.
    pub struct HortuScrolled(ObjectSubclass<imp::HortuScrolled>)
        @extends gtk::Widget, adw::Bin, @implements gtk::Accessible;
}

impl HortuScrolled {
    pub fn new(is_resume: bool) -> Self {
        glib::Object::builder().property("isresume", is_resume).build()
    }

    pub fn set_items(&self, items: &Vec<SimpleListItem>) {
        if items.is_empty() {
            return;
        }

        let items = items.clone();

        let imp = self.imp();
        let store = imp.selection.model().unwrap().downcast::<gio::ListStore>().unwrap();

        imp.revealer.set_reveal_child(true);

        spawn(glib::clone!(@weak store=> async move {
            for result in items {
                let object = glib::BoxedAnyObject::new(result);
                store.append(&object);
                gtk::glib::timeout_future(std::time::Duration::from_millis(30)).await;
            }
            
        }));
    }

    pub fn set_title(&self, title: &str) {
        self.imp().label.set_text(title);
    }

    
}
