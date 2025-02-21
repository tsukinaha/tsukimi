use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gtk::{
    CompositeTemplate,
    glib,
};

mod imp {
    use std::cell::{
        Cell,
        RefCell,
    };

    use glib::subclass::InitializingObject;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/listexpand_row.ui")]
    #[properties(wrapper_type = super::ListExpandRow)]
    pub struct ListExpandRow {
        #[property(get, set, nullable)]
        pub label: RefCell<Option<String>>,
        #[property(get, set, default_value = true)]
        pub expanded: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ListExpandRow {
        const NAME: &'static str = "ListExpandRow";
        type Type = super::ListExpandRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ListExpandRow {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().set_up();
        }
    }

    impl WidgetImpl for ListExpandRow {}
    impl ListBoxRowImpl for ListExpandRow {
        fn activate(&self) {
            let obj = self.obj();
            obj.set_expanded(!obj.expanded());
            obj.update();
        }
    }
}

glib::wrapper! {
    /// A sidebar row expand servers/content
    pub struct ListExpandRow(ObjectSubclass<imp::ListExpandRow>)
        @extends gtk::Widget, gtk::ListBoxRow, @implements gtk::Accessible;
}

impl ListExpandRow {
    pub fn new(label: String) -> Self {
        glib::Object::builder().property("label", label).build()
    }

    pub fn set_up(&self) {
        self.add_css_class("expand");
    }

    fn update(&self) {
        let expanded = self.expanded();

        if expanded {
            self.remove_css_class("expanded")
        } else {
            self.add_css_class("expanded")
        }

        self.add_css_class("interacted")
    }
}
