use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gtk::{
    CompositeTemplate,
    gio,
    glib,
};

use crate::{
    client::structs::{
        SGTitem,
        Urls,
    },
    utils::spawn,
};

mod imp {
    use std::cell::RefCell;

    use glib::subclass::InitializingObject;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/horbu_scrolled.ui")]
    #[properties(wrapper_type = super::HorbuScrolled)]
    pub struct HorbuScrolled {
        #[template_child]
        pub label: TemplateChild<gtk::Label>,
        #[template_child]
        pub wrapbox: TemplateChild<adw::WrapBox>,
        #[template_child]
        pub revealer: TemplateChild<gtk::Revealer>,

        #[property(get, set)]
        pub title: RefCell<String>,

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
}

glib::wrapper! {
    /// A scrolled list of items.
    pub struct HorbuScrolled(ObjectSubclass<imp::HorbuScrolled>)
        @extends gtk::Widget, adw::Bin, @implements gtk::Accessible;
}

impl Default for HorbuScrolled {
    fn default() -> Self {
        Self::new()
    }
}

impl HorbuScrolled {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_items(&self, items: &[SGTitem], type_: &str) {
        if items.is_empty() {
            return;
        }

        self.set_visible(true);

        let items = items.to_owned();

        let imp = self.imp();

        imp.revealer.set_reveal_child(true);

        let wrapbox = imp.wrapbox.get();
        let type_ = type_.to_string();

        spawn(glib::clone!(
            #[weak]
            wrapbox,
            #[weak(rename_to = obj)]
            self,
            async move {
                for result in items {
                    let buttoncontent = adw::ButtonContent::builder()
                        .label(&result.name)
                        .icon_name("view-list-symbolic")
                        .build();

                    let button = gtk::Button::builder().child(&buttoncontent).build();

                    let type_ = type_.to_string();
                    button.connect_clicked(glib::clone!(
                        #[weak]
                        obj,
                        move |_| {
                            result.activate(&obj, type_.to_string());
                        }
                    ));

                    wrapbox.append(&button);
                }
            }
        ));
    }

    pub fn set_links(&self, items: &[Urls]) {
        if items.is_empty() {
            return;
        }

        self.set_visible(true);

        let items = items.to_owned();

        let imp = self.imp();

        imp.revealer.set_reveal_child(true);

        let wrapbox = imp.wrapbox.get();

        while let Some(child) = wrapbox.last_child() {
            wrapbox.remove(&child);
        }

        spawn(glib::clone!(
            #[weak]
            wrapbox,
            async move {
                for result in items {
                    let buttoncontent = adw::ButtonContent::builder()
                        .label(&result.name)
                        .icon_name("external-link-symbolic")
                        .build();

                    let button = gtk::Button::builder().child(&buttoncontent).build();

                    button.connect_clicked(move |_| {
                        let _ = gio::AppInfo::launch_default_for_uri(
                            &result.url,
                            Option::<&gio::AppLaunchContext>::None,
                        );
                    });

                    wrapbox.append(&button);
                }
            }
        ));
    }
}
