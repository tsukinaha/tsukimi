use adw::{prelude::*, subclass::prelude::*};
use gtk::{gio, glib, CompositeTemplate};

use crate::client::structs::{SGTitem, Urls};
use crate::utils::spawn;

mod imp {
    use std::cell::OnceCell;

    use glib::subclass::InitializingObject;

    use crate::ui::widgets::{singlelist::SingleListPage, window::Window};

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsukimi/horbu_scrolled.ui")]
    #[properties(wrapper_type = super::HorbuScrolled)]
    pub struct HorbuScrolled {
        #[property(get, set, construct_only)]
        pub isresume: OnceCell<bool>,
        #[property(get, set, nullable)]
        pub list_type: OnceCell<Option<String>>,
        #[template_child]
        pub label: TemplateChild<gtk::Label>,
        #[template_child]
        pub flow: TemplateChild<gtk::FlowBox>,
        #[template_child]
        pub revealer: TemplateChild<gtk::Revealer>,

        pub selection: gtk::SingleSelection,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HorbuScrolled {
        const NAME: &'static str = "HorbuScrolled";
        type Type = super::HorbuScrolled;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for HorbuScrolled {}

    impl WidgetImpl for HorbuScrolled {}

    impl BinImpl for HorbuScrolled {}

    impl HorbuScrolled {
        pub fn activate(&self, result: &SGTitem, parentid: Option<String>) {
            let window = self.obj().root().and_downcast::<Window>().unwrap();
            let (view, title_var) = match window.current_view_name().as_str() {
                "homepage" => (&window.imp().homeview, "HOME_TITLE"),
                "searchpage" => (&window.imp().searchview, "SEARCH_TITLE"),
                "historypage" => (&window.imp().historyview, "HISTORY_TITLE"),
                _ => (&window.imp().searchview, "SEARCH_TITLE"),
            };
            window.set_title(&result.name);
            std::env::set_var(title_var, &result.name);

            let list_type = self.list_type.get().unwrap().as_ref().unwrap();
            Self::push_page(
                view,
                &window,
                &result.name,
                SingleListPage::new(result.id.to_string(), "".to_string(), list_type, parentid, true),
            );
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
    pub struct HorbuScrolled(ObjectSubclass<imp::HorbuScrolled>)
        @extends gtk::Widget, adw::Bin, @implements gtk::Accessible;
}

impl HorbuScrolled {
    pub fn new(is_resume: bool) -> Self {
        glib::Object::builder()
            .property("isresume", is_resume)
            .build()
    }

    pub fn set_items(&self, items: &Vec<SGTitem>) {
        if items.is_empty() {
            return;
        }

        self.set_visible(true);

        let items = items.clone();

        let imp = self.imp();

        imp.revealer.set_reveal_child(true);

        let flow = imp.flow.get();

        spawn(glib::clone!(@weak flow, @weak self as obj => async move {
            for result in items {
                let buttoncontent = adw::ButtonContent::builder()
                    .label(&result.name)
                    .icon_name("view-list-symbolic")
                    .build();

                let button = gtk::Button::builder()
                    .margin_start(10)
                    .child(&buttoncontent)
                    .build();

                button.connect_clicked(glib::clone!(@weak obj=> move |_| {
                    obj.imp().activate(&result, None);
                }));

                flow.append(&button);

                gtk::glib::timeout_future(std::time::Duration::from_millis(30)).await;
            }

        }));
    }

    pub fn set_links(&self, items: &Vec<Urls>) {
        if items.is_empty() {
            return;
        }

        self.set_visible(true);

        let items = items.clone();

        let imp = self.imp();

        imp.revealer.set_reveal_child(true);

        let flow = imp.flow.get();

        spawn(glib::clone!(@weak flow, @weak self as obj => async move {
            for result in items {
                let buttoncontent = adw::ButtonContent::builder()
                    .label(&result.name)
                    .icon_name("send-to-symbolic")
                    .build();

                let button = gtk::Button::builder()
                    .margin_start(10)
                    .child(&buttoncontent)
                    .build();

                button.connect_clicked(glib::clone!(@weak obj=> move |_| {
                    let _ = gio::AppInfo::launch_default_for_uri(
                        &result.url,
                        Option::<&gio::AppLaunchContext>::None,
                    );
                }));

                flow.append(&button);

                gtk::glib::timeout_future(std::time::Duration::from_millis(30)).await;
            }

        }));
    }

    pub fn set_title(&self, title: &str) {
        self.imp().label.set_text(title);
    }
}
