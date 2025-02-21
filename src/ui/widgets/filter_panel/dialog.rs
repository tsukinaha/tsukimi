use super::{
    FiltersList,
    FiltersRow,
};
use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gtk::{
    glib,
    template_callbacks,
};

mod imp {
    use adw::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib,
    };

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/filter.ui")]
    pub struct FilterPanelDialog {
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub apply_button_row: TemplateChild<adw::ButtonRow>,
        #[template_child]
        pub navigation_view: TemplateChild<adw::NavigationView>,

        #[template_child]
        pub playback_status_combo: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub favourite_check: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub genres_row: TemplateChild<FiltersRow>,
        #[template_child]
        pub tags_row: TemplateChild<FiltersRow>,
        #[template_child]
        pub years_row: TemplateChild<FiltersRow>,
        #[template_child]
        pub ratings_row: TemplateChild<FiltersRow>,
        #[template_child]
        pub studios_row: TemplateChild<FiltersRow>,
        #[template_child]
        pub containers_row: TemplateChild<FiltersRow>,
        #[template_child]
        pub encoders_row: TemplateChild<FiltersRow>,
        #[template_child]
        pub video_types_row: TemplateChild<FiltersRow>,
        #[template_child]
        pub resolution_combo: TemplateChild<adw::ComboRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FilterPanelDialog {
        const NAME: &'static str = "FilterPanelDialog";
        type Type = super::FilterPanelDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            FiltersRow::ensure_type();
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

impl Default for FilterPanelDialog {
    fn default() -> Self {
        Self::new()
    }
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

    pub fn connect_applied<F: Fn(&adw::ButtonRow) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.imp().apply_button_row.connect_activated(f)
    }

    pub fn push_page<P>(&self, page: &P)
    where
        P: IsA<adw::NavigationPage>,
    {
        self.imp().navigation_view.push(page);
    }

    #[template_callback]
    fn on_clear_activated(&self) {
        let imp = self.imp();
        imp.playback_status_combo.set_selected(0);
        imp.favourite_check.set_active(false);
        imp.genres_row.clear_filters();
        imp.tags_row.clear_filters();
        imp.years_row.clear_filters();
        imp.ratings_row.clear_filters();
        imp.studios_row.clear_filters();
        imp.containers_row.clear_filters();
        imp.encoders_row.clear_filters();
        imp.video_types_row.clear_filters();
        imp.resolution_combo.set_selected(0);
    }

    pub fn filters_list(&self) -> FiltersList {
        FiltersList {
            playback_status: self.imp().playback_status_combo.selected(),
            favourite: self.imp().favourite_check.is_active(),
            genres: self.imp().genres_row.filters(),
            tags: self.imp().tags_row.filters(),
            years: self.imp().years_row.filters(),
            ratings: self.imp().ratings_row.filters(),
            studios: self.imp().studios_row.filters(),
            containers: self.imp().containers_row.filters(),
            encoders: self.imp().encoders_row.filters(),
            video_types: self.imp().video_types_row.filters(),
            resolution: self.imp().resolution_combo.selected(),
        }
    }
}
