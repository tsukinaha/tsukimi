use adw::prelude::NavigationPageExt;
use glib::Object;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::Label;
use gtk::{gio, glib};
use std::env;

use crate::ui::network::List;
use crate::ui::widgets::item::ItemPage;
use crate::ui::widgets::movie::MoviePage;
use crate::ui::widgets::window::Window;

mod imp {

    use glib::subclass::InitializingObject;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsukimi/search.ui")]
    pub struct SearchPage {
        #[template_child]
        pub searchentry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub searchgrid: TemplateChild<gtk::GridView>,
        #[template_child]
        pub spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub searchscrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub searchrevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub recommendbox: TemplateChild<gtk::Box>,
        pub selection: gtk::SingleSelection,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for SearchPage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "SearchPage";
        type Type = super::SearchPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for SearchPage {
        fn constructed(&self) {
            let obj = self.obj();
            self.parent_constructed();
            obj.setup_recommend();
            obj.setup_search();
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for SearchPage {}

    // Trait shared by all windows
    impl WindowImpl for SearchPage {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for SearchPage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for SearchPage {}
}

glib::wrapper! {
    pub struct SearchPage(ObjectSubclass<imp::SearchPage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Default for SearchPage {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchPage {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn setup_recommend(&self) {
        let (sender, receiver) = async_channel::bounded::<List>(1);
        crate::ui::network::runtime().spawn(async move {
            let list = crate::ui::network::get_search_recommend()
                .await
                .expect("msg");
            sender
                .send(list)
                .await
                .expect("The channel needs to be open.");
        });
        glib::spawn_future_local(glib::clone!(@weak self as obj =>async move {
            while let Ok(list) = receiver.recv().await {
                let imp = obj.imp();
                let recommendbox = imp.recommendbox.get();
                for item in list.items {
                    let button = gtk::Button::new();
                    let buttoncontent = adw::ButtonContent::builder()
                            .label(&item.name)
                            .icon_name(if item.latest_type == "Movie" {
                                "video-display-symbolic"
                            } else {
                                "video-x-generic"
                            })
                            .build();
                    button.set_halign(gtk::Align::Center);
                    button.set_child(Some(&buttoncontent));
                    button.connect_clicked(glib::clone!(@weak obj => move |_| {
                        let window = obj.root().and_downcast::<Window>().unwrap();
                        if item.latest_type == "Movie" {
                            let item_page = MoviePage::new(item.id.clone(),item.name.clone());
                            item_page.set_tag(Some(&item.name));
                            window.imp().searchview.push(&item_page);
                            window.set_title(&item.name);
                            window.change_pop_visibility();
                            env::set_var("HOME_TITLE", &item.name)
                        } else if item.latest_type == "Series" {
                            let item_page = ItemPage::new(item.id.clone(),item.id.clone());
                            item_page.set_tag(Some(&item.name));
                            window.imp().searchview.push(&item_page);
                            window.set_title(&item.name);
                            window.change_pop_visibility();
                            env::set_var("HOME_TITLE", &item.name)
                        }
                    }));
                    recommendbox.append(&button);
                }
            }
        }));
    }

    pub fn setup_search(&self) {
        let imp = self.imp();
        let spinner = imp.spinner.get();
        let searchrevealer = imp.searchrevealer.get();
        let recommendbox = imp.recommendbox.get();
        let (sender, receiver) = async_channel::bounded::<Vec<crate::ui::network::SearchResult>>(1);
        imp.searchentry.connect_activate(glib::clone!(@strong sender,@weak spinner=> move |entry| {
                spinner.set_visible(true);
                recommendbox.set_visible(false);
                let search_content = entry.text().to_string();
                crate::ui::network::runtime().spawn(glib::clone!(@strong sender => async move {
                    let search_results = crate::ui::network::search(search_content).await.unwrap_or_else(|e| {
                        eprintln!("Error: {}", e);
                        Vec::<crate::ui::network::SearchResult>::new()
                    });
                    sender.send(search_results).await.expect("search results not received.");
                }));
            }));

        let store = gio::ListStore::new::<glib::BoxedAnyObject>();
        glib::spawn_future_local(glib::clone!(@weak store=> async move {
            while let Ok(search_results) = receiver.recv().await {
                spinner.set_visible(false);
                store.remove_all();
                for result in search_results {
                    if result.result_type == "Series" || result.result_type == "Movie" {
                        let object = glib::BoxedAnyObject::new(result);
                        store.append(&object);
                    }
                }
                searchrevealer.set_reveal_child(true);
            }
        }));

        imp.selection.set_model(Some(&store));
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_bind(|_, item| {
            let listitem = item.downcast_ref::<gtk::ListItem>().unwrap();
            let entry = listitem
                .item()
                .and_downcast::<glib::BoxedAnyObject>()
                .unwrap();
            let result: std::cell::Ref<crate::ui::network::SearchResult> = entry.borrow();
            let vbox = gtk::Box::new(gtk::Orientation::Vertical, 2);
            let overlay = gtk::Overlay::new();
            let imgbox = crate::ui::image::setimage(result.id.clone());
            imgbox.set_size_request(167, 275);
            overlay.set_child(Some(&imgbox));
            if let Some(userdata) = &result.user_data {
                if let Some(unplayeditemcount) = userdata.unplayed_item_count {
                    if unplayeditemcount > 0 {
                        let mark = gtk::Label::new(Some(
                            &userdata
                                .unplayed_item_count
                                .expect("no unplayeditemcount")
                                .to_string(),
                        ));
                        mark.set_valign(gtk::Align::Start);
                        mark.set_halign(gtk::Align::End);
                        mark.set_height_request(40);
                        mark.set_width_request(40);
                        overlay.add_overlay(&mark);
                    }
                }
                if userdata.played {
                    let mark = gtk::Image::from_icon_name("object-select-symbolic");
                    mark.set_halign(gtk::Align::End);
                    mark.set_valign(gtk::Align::Start);
                    mark.set_height_request(40);
                    mark.set_width_request(40);
                    overlay.add_overlay(&mark);
                }
            }
            vbox.append(&overlay);
            let label = Label::new(Some(&result.name));
            label.set_wrap(true);
            label.set_size_request(-1, 24);
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            let labeltype = Label::new(Some(&result.result_type));
            let markup = format!(
                "<span color='lightgray' font='small'>{}</span>",
                result.result_type
            );
            labeltype.set_markup(markup.as_str());
            labeltype.set_size_request(-1, 24);
            vbox.append(&label);
            vbox.append(&labeltype);
            listitem.set_child(Some(&vbox));
        });
        factory.connect_unbind(|_, item| {
            let listitem = item.downcast_ref::<gtk::ListItem>().unwrap();
            listitem.set_child(None::<&gtk::Widget>);
        });
        imp.searchgrid.set_factory(Some(&factory));
        imp.searchgrid.set_model(Some(&imp.selection));
        imp.searchgrid.set_min_columns(1);
        imp.searchgrid.set_max_columns(15);
        imp.searchgrid.connect_activate(
            glib::clone!(@weak self as obj => move |gridview, position| {
                let model = gridview.model().unwrap();
                let item = model.item(position).and_downcast::<glib::BoxedAnyObject>().unwrap();
                let result: std::cell::Ref<crate::ui::network::SearchResult> = item.borrow();
                let window = obj.root().and_downcast::<Window>().unwrap();
                if result.result_type == "Movie" {
                    let item_page = MoviePage::new(result.id.clone(),result.name.clone());
                    item_page.set_tag(Some(&result.name));
                    window.imp().searchview.push(&item_page);
                    window.change_pop_visibility();
                } else {
                    let item_page = ItemPage::new(result.id.clone(),result.id.clone());
                    item_page.set_tag(Some(&result.name));
                    window.imp().searchview.push(&item_page);
                    window.change_pop_visibility();
                }
                window.set_title(&result.name);
                std::env::set_var("SEARCH_TITLE", &result.name)
            }),
        );
    }
}
