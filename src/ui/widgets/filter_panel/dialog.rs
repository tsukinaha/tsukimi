use adw::subclass::prelude::*;
use gtk::{
    glib,
    template_callbacks,
};


mod imp {
    use adw::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        glib,
        CompositeTemplate,
    };

    use super::*;
    use crate::ui::widgets::filter_panel::FilterRow;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/filter.ui")]
    pub struct FilterPanelDialog {
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FilterPanelDialog {
        const NAME: &'static str = "FilterPanelDialog";
        type Type = super::FilterPanelDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            FilterRow::ensure_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for FilterPanelDialog {}

    impl WidgetImpl for FilterPanelDialog {}
    impl AdwDialogImpl for FilterPanelDialog {}
}

glib::wrapper! {

    pub struct FilterPanelDialog(ObjectSubclass<imp::FilterPanelDialog>)
        @extends gtk::Widget, adw::Dialog, adw::PreferencesDialog, @implements gtk::Accessible, gtk::Root;
}

#[template_callbacks]
impl FilterPanelDialog {
    const LOADING_STACK_PAGE: &'static str = "loading";
    const VIEW_STACK_PAGE: &'static str = "view";

    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn loading_page(&self) {
        self.imp()
            .stack
            .set_visible_child_name(Self::LOADING_STACK_PAGE);
    }

    pub fn view_page(&self) {
        self.imp()
            .stack
            .set_visible_child_name(Self::VIEW_STACK_PAGE);
    }

    pub fn add_toast(&self, toast: adw::Toast) {
        self.imp().toast_overlay.add_toast(toast);
    }
}
