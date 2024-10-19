pub(crate) mod models;
mod mpv;
pub mod provider;
pub mod widgets;
use adw::prelude::*;
use gettextrs::gettext;
use gtk::{
    gdk::Display,
    CssProvider,
};

use self::models::SETTINGS;

pub fn build_ui(app: &adw::Application) {
    // Create new window and present it
    let window = widgets::window::Window::new(app);
    let about_action = gtk::gio::ActionEntry::builder("about")
        .activate(|_, _, _| {
            let about = adw::AboutDialog::builder()
                .application_name("Tsukimi")
                .version(crate::config::VERSION)
                .comments(
                    "A simple third-party Emby client.\nVersion: tsukimi 0.16.3 \n2024.10.18 20:05",
                )
                // TRANSLATORS: 'Name <email@domain.com>' or 'Name https://website.example'
                .translator_credits(gettext("translator-credits"))
                .website("https://github.com/tsukinaha/tsukimi")
                .application_icon("tsukimi")
                .license_type(gtk::License::Gpl30)
                .build();
            about.add_acknowledgement_section(Some("Code"), &["Inaha", "Kosette"]);
            about.add_acknowledgement_section(
                Some("Special Thanks"),
                &["Qound", "Eikano", "amtoaer"],
            );
            about.present(None::<&gtk::Widget>);
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
