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
    use once_cell::sync::Lazy;
    use url::Url;

    use crate::{
        client::jellyfin_client::{
            DEVICE_ID,
            JELLYFIN_CLIENT,
        },
        ui::{
            SETTINGS,
            match_audio_channels,
            match_hwdec_interop,
            match_sub_border_style,
            match_video_upscale,
        },
    };

    use super::*;

    const MAX_VOLUME: i64 = 100;

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

            configure_mpv();

            let obj = self.obj();
            obj.set_application_id(Some(crate::APP_ID));
            obj.set_resource_base_path(Some(crate::APP_RESOURCE_PATH));

            obj.set_accels_for_action("win.about", &["<Ctrl>N"]);
        }
    }

    impl ApplicationImpl for TsukimiApplication {
        fn activate(&self) {
            self.parent_activate();

            let Some(window) = self.obj().active_window() else {
                return;
            };

            window.present();
        }

        fn startup(&self) {
            self.parent_startup();

            // Eagerly initialize `DEVICE_ID` and `JELLYFIN_CLIENT` because they depend on `SETTINGS`, which can only be accessed from the main thread.
            // If either is first accessed inside `spawn_tokio`, the application will panic.
            Lazy::force(&DEVICE_ID);
            Lazy::force(&JELLYFIN_CLIENT);

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

    fn configure_mpv() {
        mutsumi::set_mpv_initializer(|init| {
            init.set_option("input-vo-keyboard", true)?;
            init.set_option("input-default-bindings", true)?;

            if SETTINGS.mpv_config() {
                init.set_option("config", true)?;
                init.set_option("config-dir", SETTINGS.mpv_config_dir().as_str())?;
            }

            init.set_option("user-agent", crate::USER_AGENT.as_str())?;
            init.set_option("video-timing-offset", 0_i64)?;
            init.set_option("video-sync", "audio")?;
            init.set_option("osc", false)?;
            init.set_option("osd-level", 0_i64)?;

            let demuxer_max_bytes = format!("{}MiB", SETTINGS.mpv_cache_size());
            init.set_option("demuxer-max-bytes", demuxer_max_bytes.as_str())?;
            init.set_option("cache-secs", SETTINGS.mpv_cache_time() as f64)?;
            init.set_option("volume-max", MAX_VOLUME)?;
            init.set_option("volume", SETTINGS.mpv_default_volume() as i64)?;

            init.set_option("sub-bold", SETTINGS.mpv_subtitle_bold())?;
            init.set_option("sub-italic", SETTINGS.mpv_subtitle_italic())?;
            init.set_option(
                "sub-justify",
                match SETTINGS.mpv_subtitle_justify() {
                    0 => "left",
                    2 => "right",
                    _ => "center",
                },
            )?;
            init.set_option("sub-pos", SETTINGS.mpv_subtitle_position() as f64)?;
            init.set_option("sub-font-size", SETTINGS.mpv_subtitle_size() as f64)?;
            init.set_option("sub-scale", SETTINGS.mpv_subtitle_scale())?;
            init.set_option("sub-font", SETTINGS.mpv_subtitle_font().as_str())?;
            init.set_option(
                "sub-border-style",
                match_sub_border_style(SETTINGS.mpv_subtitle_border_style()),
            )?;
            init.set_option(
                "sub-border-size",
                SETTINGS.mpv_subtitle_border_size() as f64,
            )?;
            init.set_option(
                "sub-shadow-offset",
                SETTINGS.mpv_subtitle_shadow_offset() as f64,
            )?;
            init.set_option(
                "stretch-image-subs-to-screen",
                SETTINGS.mpv_subtitle_stretch_image_subs_to_screen(),
            )?;

            let sub_color =
                settings_color_to_mpv(SETTINGS.mpv_subtitle_text_color(), (1.0, 1.0, 1.0, 1.0));
            let sub_border_color =
                settings_color_to_mpv(SETTINGS.mpv_subtitle_border_color(), (0.0, 0.0, 0.0, 1.0));
            let sub_back_color = settings_color_to_mpv(
                SETTINGS.mpv_subtitle_background_color(),
                (0.0, 0.0, 0.0, 0.0),
            );
            init.set_option("sub-color", sub_color.as_str())?;
            init.set_option("sub-border-color", sub_border_color.as_str())?;
            init.set_option("sub-back-color", sub_back_color.as_str())?;

            init.set_option("hwdec", match_hwdec_interop(SETTINGS.mpv_hwdec()))?;
            init.set_option("scale", match_video_upscale(SETTINGS.mpv_video_scale()))?;
            init.set_option(
                "loop-file",
                if SETTINGS.mpv_action_after_video_end() == 1 {
                    "inf"
                } else {
                    "no"
                },
            )?;
            init.set_option(
                "audio-channels",
                match_audio_channels(SETTINGS.mpv_audio_channel()),
            )?;

            if let Some(uri) = crate::client::proxy::get_proxy_settings() {
                let url = if Url::parse(&uri).is_ok() {
                    uri
                } else {
                    format!("http://{uri}")
                };
                init.set_option("http-proxy", url.as_str())?;
            }

            init.set_option(
                "alang",
                match SETTINGS.mpv_audio_preferred_lang() {
                    0 => "",
                    1 => "eng",
                    2 => "chs",
                    3 => "jpn",
                    4 => "chi",
                    5 => "ara",
                    6 => "nob",
                    7 => "por",
                    8 => "fre",
                    9 => "rus",
                    _ => "",
                },
            )?;

            Ok(())
        })
        .expect("Failed to set mpv initializer");
    }

    fn settings_color_to_mpv(value: String, default: (f32, f32, f32, f32)) -> String {
        let rgba = RGBA::parse(&value)
            .unwrap_or_else(|_| RGBA::new(default.0, default.1, default.2, default.3));

        format!(
            "{}/{}/{}/{}",
            rgba.red(),
            rgba.green(),
            rgba.blue(),
            rgba.alpha()
        )
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
