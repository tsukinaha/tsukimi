use glib::Object;
use gtk::{gio, glib};

mod imp {
    use adw::prelude::NavigationPageExt;
    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{gio, glib, CompositeTemplate, Label};

    use crate::ui::widgets::item::ItemPage;
    use crate::ui::widgets::movie::MoviePage;
    use crate::ui::widgets::window::Window;

    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsukimi/history.ui")]
    pub struct HistoryPage {
        #[template_child]
        pub historygrid: TemplateChild<gtk::GridView>,
        #[template_child]
        pub spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub historyscrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub historyrevealer: TemplateChild<gtk::Revealer>,
        pub selection: gtk::SingleSelection,
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
            let spinner = self.spinner.get();
            let historyrevealer = self.historyrevealer.get();
            spinner.set_visible(true);
            let (sender, receiver) = async_channel::bounded::<Vec<crate::ui::network::Resume>>(1);
            crate::ui::network::runtime().spawn(glib::clone!(@strong sender => async move {
                let history_results = crate::ui::network::resume().await.unwrap_or_else(|e| {
                    eprintln!("Error: {}", e);
                    Vec::<crate::ui::network::Resume>::new()
                });
                sender.send(history_results).await.expect("history results not received.");
            }));
            let store = gio::ListStore::new::<glib::BoxedAnyObject>();
            glib::spawn_future_local(glib::clone!(@weak store=> async move {
                while let Ok(history_results) = receiver.recv().await {
                    for result in history_results {
                        let object = glib::BoxedAnyObject::new(result);
                        store.append(&object);
                    }
                    spinner.set_visible(false);
                    historyrevealer.set_reveal_child(true);
                }
            }));
            self.selection.set_autoselect(false);
            self.selection.set_model(Some(&store));
            let factory = gtk::SignalListItemFactory::new();
            factory.connect_bind(move |_factory, item| {
                let listitem = item.downcast_ref::<gtk::ListItem>().unwrap();
                let entry = listitem
                    .item()
                    .and_downcast::<glib::BoxedAnyObject>()
                    .unwrap();
                let result: std::cell::Ref<crate::ui::network::Resume> = entry.borrow();
                let vbox = gtk::Box::new(gtk::Orientation::Vertical, 5);
                let overlay = gtk::Overlay::new();
                let imgbox;
                if result.parent_thumb_item_id.is_some() && result.resume_type == "Episode" {
                    imgbox = crate::ui::image::setthumbimage(
                        result.parent_thumb_item_id.as_ref().expect("").clone(),
                    );
                } else if result.resume_type == "Movie" {
                    imgbox = crate::ui::image::setbackdropimage(result.id.clone(),0);
                } else if result.parent_thumb_item_id.is_some() {
                    imgbox = crate::ui::image::setthumbimage(
                        result.series_id.as_ref().expect("").to_string(),
                    );
                } else {
                    imgbox = crate::ui::image::setimage(result.id.clone());
                }
                imgbox.set_size_request(265, 169);
                overlay.set_child(Some(&imgbox));
                let progressbar = gtk::ProgressBar::new();
                progressbar.set_valign(gtk::Align::End);
                if let Some(userdata) = &result.user_data {
                    if let Some(percentage) = userdata.played_percentage {
                        progressbar.set_fraction(percentage / 100.0);
                    }
                    if userdata.played {
                        let mark = gtk::Image::from_icon_name("object-select-symbolic");
                        mark.set_halign(gtk::Align::End);
                        mark.set_valign(gtk::Align::Start);
                        mark.set_height_request(25);
                        mark.set_width_request(25);
                        overlay.add_overlay(&mark);
                    }
                }
                overlay.add_overlay(&progressbar);
                vbox.append(&overlay);
                let label = Label::builder().label(&result.name).build();
                let labeltype = Label::new(Some(&result.resume_type));
                if result.resume_type == "Episode" {
                    let markup = result.series_name.as_ref().expect("").clone().to_string();
                    label.set_markup(markup.as_str());
                    let markup = format!(
                        "<span color='lightgray' font='small'>S{}E{}: {}</span>",
                        result.parent_index_number.as_ref().expect("").clone(),
                        result.index_number.as_ref().expect("").clone(),
                        result.name
                    );
                    labeltype.set_markup(markup.as_str());
                } else {
                    let markup = result.name.to_string();
                    label.set_markup(markup.as_str());
                    let markup = format!(
                        "<span color='lightgray' font='small'>{}</span>",
                        result.resume_type
                    );
                    labeltype.set_markup(markup.as_str());
                }
                label.set_wrap(true);
                label.set_size_request(-1, 5);
                label.set_ellipsize(gtk::pango::EllipsizeMode::End);
                labeltype.set_ellipsize(gtk::pango::EllipsizeMode::End);
                label.set_size_request(-1, 5);
                vbox.append(&label);
                vbox.append(&labeltype);
                listitem.set_child(Some(&vbox));
            });
            factory.connect_unbind(|_, item| {
                let listitem = item.downcast_ref::<gtk::ListItem>().unwrap();
                listitem.set_child(None::<&gtk::Widget>);
            });
            self.historygrid.set_factory(Some(&factory));
            self.historygrid.set_model(Some(&self.selection));
            self.historygrid.set_min_columns(3);
            self.historygrid.connect_activate(glib::clone!(@weak obj => move |gridview, position| {
                let model = gridview.model().unwrap();
                let item = model.item(position).and_downcast::<glib::BoxedAnyObject>().unwrap();
                let result: std::cell::Ref<crate::ui::network::Resume> = item.borrow();
                let window = obj.root().and_downcast::<Window>().unwrap();
                if result.resume_type == "Movie" {
                    let item_page = MoviePage::new(result.id.clone(),result.name.clone());
                    item_page.set_tag(Some(&result.name));
                    window.imp().historyview.push(&item_page);
                    window.change_pop_visibility();
                } else if result.parent_thumb_item_id.is_none() {
                    let item_page = ItemPage::new(result.series_id.as_ref().expect("msg").clone(),result.id.clone());
                    item_page.set_tag(Some(&result.name));
                    window.imp().historyview.push(&item_page);
                    window.change_pop_visibility();
                } else {
                    let item_page = ItemPage::new(result.parent_thumb_item_id.as_ref().expect("msg").clone(),result.id.clone());
                    item_page.set_tag(Some(&result.name));
                    window.imp().historyview.push(&item_page);
                    window.change_pop_visibility();
                }
                if let Some(seriesname) = &result.series_name {
                    window.set_title(seriesname);
                    std::env::set_var("HISTORY_TITLE", seriesname)
                } else {
                    window.set_title(&result.name);
                    std::env::set_var("HISTORY_TITLE", &result.name)
                }
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
}
