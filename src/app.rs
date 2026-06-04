use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gtk::glib;

mod imp {
    use std::cell::{
        Cell,
        OnceCell,
    };

    use gtk::{
        CssProvider,
        gdk::{
            Display,
            RGBA,
        },
    };

    use crate::ui::SETTINGS;

    use super::*;

    #[derive(Debug, Default)]
    pub struct TsukimiApplication {
        accent_provider: OnceCell<CssProvider>,
        accent_provider_added: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TsukimiApplication {
        const NAME: &'static str = "TsukimiApplication";
        type Type = super::TsukimiApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for TsukimiApplication {
        fn constructed(&self) {
            self.parent_constructed();
            self.update_accent_provider();

            SETTINGS.connect_changed(
                Some("use-custom-accent-color"),
                glib::clone!(
                    #[weak(rename_to = obj)]
                    self.obj(),
                    move |_, _| obj.imp().update_accent_provider()
                ),
            );
            SETTINGS.connect_changed(
                Some("accent-color-code"),
                glib::clone!(
                    #[weak(rename_to = obj)]
                    self.obj(),
                    move |_, _| obj.imp().update_accent_provider()
                ),
            );
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
        fn update_accent_provider(&self) {
            let display = Display::default().expect("Could not connect to a display.");

            if !SETTINGS.use_custom_accent_color() {
                if let Some(provider) = self.accent_provider.get()
                    && self.accent_provider_added.get()
                {
                    gtk::style_context_remove_provider_for_display(&display, provider);
                    self.accent_provider_added.set(false);
                }
                return;
            }

            let provider = self.accent_provider.get_or_init(CssProvider::new);
            let accent_color = SETTINGS.accent_color_code();
            let accent_fg_color = readable_foreground_color(&accent_color);

            provider.load_from_string(&format!(
                "
                @define-color accent_color {accent_color};
                @define-color accent_bg_color {accent_color};
                @define-color accent_fg_color {accent_fg_color};

                :root {{
                    --accent-color:{accent_color};
                    --accent-bg-color:{accent_color};
                    --accent-fg-color:{accent_fg_color};
                }}",
            ));

            if !self.accent_provider_added.get() {
                gtk::style_context_add_provider_for_display(
                    &display,
                    provider,
                    gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
                );
                self.accent_provider_added.set(true);
            }
        }
    }

    fn readable_foreground_color(color: &str) -> &'static str {
        let Ok(color) = color.parse::<RGBA>() else {
            return "#000000";
        };

        // Calculate WCAG relative luminance from sRGB channels.
        let srgb_to_linear = |channel: f32| {
            if channel <= 0.04045 {
                channel / 12.92
            } else {
                ((channel + 0.055) / 1.055).powf(2.4)
            }
        };

        let luminance = 0.2126 * srgb_to_linear(color.red())
            + 0.7152 * srgb_to_linear(color.green())
            + 0.0722 * srgb_to_linear(color.blue());

        // 0.179 is the contrast crossover where black becomes more readable than white.
        if luminance >= 0.179 {
            "#000000"
        } else {
            "#ffffff"
        }
    }
}

glib::wrapper! {
    pub struct TsukimiApplication(ObjectSubclass<imp::TsukimiApplication>)
        @extends gtk::gio::Application, gtk::Application, adw::Application, @implements gtk::gio::ActionGroup, gtk::gio::ActionMap;
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
