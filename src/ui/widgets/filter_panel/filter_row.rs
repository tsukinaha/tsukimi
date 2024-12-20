use adw::subclass::prelude::*;
use gtk::{
    glib,
    template_callbacks,
    CompositeTemplate,
};

mod imp {
    use std::cell::RefCell;

    use glib::subclass::InitializingObject;
    use gtk::prelude::*;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/filter_row.ui")]
    #[properties(wrapper_type = super::FilterRow)]
    pub struct FilterRow {
        #[property(get, set, nullable)]
        pub title: RefCell<Option<String>>,
        #[template_child]
        pub flowbox: TemplateChild<gtk::FlowBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FilterRow {
        const NAME: &'static str = "FilterRow";
        type Type = super::FilterRow;
        type ParentType = adw::PreferencesRow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FilterRow {}

    impl WidgetImpl for FilterRow {}

    impl ListBoxRowImpl for FilterRow {}

    impl PreferencesRowImpl for FilterRow {}
}

glib::wrapper! {
    pub struct FilterRow(ObjectSubclass<imp::FilterRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::ActionRow, adw::PreferencesRow, @implements gtk::Actionable, gtk::Accessible;
}

#[template_callbacks]
impl FilterRow {
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[template_callback]
    async fn on_add_button_clicked(&self) {
        let label = super::FilterLabel::new();
        self.imp().flowbox.append(&label);
    }
}
