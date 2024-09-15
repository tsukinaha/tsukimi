pub(crate) mod models;
mod mpv;
pub mod provider;
pub mod widgets;
use self::models::SETTINGS;
use gtk::gdk::Display;
use gtk::{prelude::*, CssProvider};

pub fn build_ui(app: &adw::Application) {
    // Create new window and present it
    let window = widgets::window::Window::new(app);
    let about_action = gtk::gio::ActionEntry::builder("about")
        .activate(|_, _, _| {
            let about = adw::AboutWindow::builder()
                .application_name("Tsukimi")
                .version(crate::config::APP_VERSION)
                .comments(
                    "A simple third-party Emby client.\nVersion: tsukimi 0.12.1 \n2024.8.29 14:30",
                )
                .website("https://github.com/tsukinaha/tsukimi")
                .application_icon("tsukimi")
                .license_type(gtk::License::Gpl30)
                .build();
            about.add_acknowledgement_section(Some("Code"), &["Inaha", "Kosette"]);
            about.add_acknowledgement_section(
                Some("Special Thanks"),
                &["Qound", "Eikano", "amtoaer"],
            );
            about.present();
        })
        .build();
    window.add_action_entries([about_action]);
    window.present();
}

pub fn load_css() {
    let provider = CssProvider::new();

    let mut styles = String::new();

    styles.push_str(include_str!("style.css"));

    let accent_color = SETTINGS.accent_color_code();
    styles.push_str(&format!(
        "@define-color accent_color {};
                @define-color accent_bg_color {};
                @define-color accent_fg_color {};
                overlay>label {{
                    background-color: {};
                    border-radius: 999px;
                    margin: 3px;
                }}
                
                box>overlay>image {{
                    background-color: {};
                    border-radius: 999px;
                    margin: 3px;
                }}",
        accent_color,
        accent_color,
        SETTINGS.accent_fg_color_code(),
        accent_color,
        accent_color
    ));

    provider.load_from_string(&styles);

    gtk::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
