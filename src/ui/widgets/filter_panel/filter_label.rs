use adw::subclass::prelude::*;
use gst::prelude::CastNone;
use gtk::{
    glib,
    prelude::{
        WidgetExt,
        *,
    },
    template_callbacks,
    CompositeTemplate,
};

mod imp {
    use std::cell::RefCell;

    use glib::subclass::InitializingObject;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/filter_label.ui")]
    #[properties(wrapper_type = super::FilterLabel)]
    pub struct FilterLabel {
        #[property(get, set, nullable)]
        pub label: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FilterLabel {
        const NAME: &'static str = "FilterLabel";
        type Type = super::FilterLabel;
        type ParentType = gtk::Button;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FilterLabel {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().add_css_class("no-padding");
        }
    }

    impl WidgetImpl for FilterLabel {}

    impl ButtonImpl for FilterLabel {}
}

glib::wrapper! {
    pub struct FilterLabel(ObjectSubclass<imp::FilterLabel>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::ActionRow, adw::PreferencesRow, @implements gtk::Actionable, gtk::Accessible;
}

impl Default for FilterLabel {
    fn default() -> Self {
        Self::new()
    }
}

#[template_callbacks]
impl FilterLabel {
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[template_callback]
    async fn on_delete_button_clicked(&self) {
        let Some(flowbox) = self
            .ancestor(gtk::FlowBox::static_type())
            .and_downcast::<gtk::FlowBox>()
        else {
            return;
        };

        flowbox.remove(self);
    }
}
