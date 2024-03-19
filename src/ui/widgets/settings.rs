use gtk::glib;

mod imp {

    use adw::subclass::prelude::*;
    use glib::subclass;

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/moe/tsukimi/settings.ui")]
    pub struct AccountRow {}

    #[glib::object_subclass]
    impl ObjectSubclass for AccountRow {
        const NAME: &'static str = "AccountRow";
        type Type = super::AccountRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AccountRow {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }
    impl WidgetImpl for AccountRow {}
    impl ListBoxRowImpl for AccountRow {}
    impl PreferencesRowImpl for AccountRow {}
}

glib::wrapper! {
    pub struct AccountRow(ObjectSubclass<imp::AccountRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow;
}

impl AccountRow {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
}
