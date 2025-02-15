use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gtk::glib;

mod imp {

    use gtk::{
        gdk::Display,
        CssProvider,
    };

    use crate::ui::SETTINGS;

    use super::*;

    #[derive(Debug, Default)]
    pub struct TsukimiApplication;

    #[glib::object_subclass]
    impl ObjectSubclass for TsukimiApplication {
        const NAME: &'static str = "TsukimiApplication";
        type Type = super::TsukimiApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for TsukimiApplication {
        fn constructed(&self) {
            self.parent_constructed();
            self.load_style_sheet();

            let obj = self.obj();
            obj.set_application_id(Some(crate::APP_ID));
            obj.set_resource_base_path(Some(crate::APP_RESOURCE_PATH));

            obj.set_accels_for_action("win.about", &["<Ctrl>N"]);
        }
    }

    impl ApplicationImpl for TsukimiApplication {
        fn activate(&self) {
            self.parent_activate();

            let window = crate::Window::new(&self.obj());
            window.load_window_state();
            window.present();
        }
    }

    impl GtkApplicationImpl for TsukimiApplication {}

    impl AdwApplicationImpl for TsukimiApplication {}

    impl TsukimiApplication {
        fn load_style_sheet(&self) {
            let provider = CssProvider::new();

            let accent_color = SETTINGS.accent_color_code();

            provider.load_from_string(&format!(
                "@define-color  accent_color     {};
                 @define-color  accent_bg_color  {};",
                accent_color, accent_color,
            ));

            gtk::style_context_add_provider_for_display(
                &Display::default().expect("Could not connect to a display."),
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }
}

glib::wrapper! {
    pub struct TsukimiApplication(ObjectSubclass<imp::TsukimiApplication>)
        @extends gtk::gio::Application, gtk::Application, adw::Application, @implements gtk::Accessible;
}

impl Default for TsukimiApplication {
    fn default() -> Self {
        Self::new()
    }
}

impl TsukimiApplication {
    pub fn new() -> Self {
        glib::Object::new()
    }
}
