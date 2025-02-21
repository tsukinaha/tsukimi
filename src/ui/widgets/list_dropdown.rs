use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gtk::{
    CompositeTemplate,
    glib,
};

mod imp {
    use glib::subclass::InitializingObject;

    use super::*;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/dropdown.ui")]
    pub struct ListDropdown {
        #[template_child]
        pub label1: TemplateChild<gtk::Label>,
        #[template_child]
        pub label2: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ListDropdown {
        const NAME: &'static str = "ListDropdown";
        type Type = super::ListDropdown;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ListDropdown {}

    impl WidgetImpl for ListDropdown {}
    impl BinImpl for ListDropdown {}
}

glib::wrapper! {
    /// A dropdown widget with two labels.
    pub struct ListDropdown(ObjectSubclass<imp::ListDropdown>)
        @extends gtk::Widget, adw::Dialog, adw::NavigationPage, @implements gtk::Accessible;
}

impl Default for ListDropdown {
    fn default() -> Self {
        Self::new()
    }
}

impl ListDropdown {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn set_label1(&self, label: &Option<String>) {
        if let Some(label_str) = label {
            self.imp().label1.set_text(label_str);
        }
    }

    pub fn set_label2(&self, label: &Option<String>) {
        if let Some(label_str) = label {
            self.imp().label2.set_text(label_str);
            self.imp().label2.set_visible(true);
        }
    }
}
