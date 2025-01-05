#[doc(hidden)]
#[macro_export]
macro_rules! _add_toast {
    ($widget:expr, $toast:expr) => {{
        use gtk::prelude::WidgetExt;
        use $crate::ui::widgets::{
            filter_panel::FilterPanelDialog,
            identify::IdentifyDialog,
            image_dialog::ImageDialog,
        };
        if let Some(dialog) = $widget
            .ancestor(adw::PreferencesDialog::static_type())
            .and_downcast::<adw::PreferencesDialog>()
        {
            use adw::prelude::PreferencesDialogExt;
            dialog.add_toast($toast);
        } else if let Some(overlay) = $widget
            .ancestor(adw::ToastOverlay::static_type())
            .and_downcast::<adw::ToastOverlay>()
        {
            overlay.add_toast($toast);
        } else if let Some(dialog) = $widget
            .ancestor(FilterPanelDialog::static_type())
            .and_downcast::<FilterPanelDialog>()
        {
            dialog.add_toast($toast);
        } else if let Some(dialog) = $widget
            .ancestor(IdentifyDialog::static_type())
            .and_downcast::<IdentifyDialog>()
        {
            dialog.add_toast($toast);
        } else if let Some(dialog) = $widget
            .ancestor(ImageDialog::static_type())
            .and_downcast::<ImageDialog>()
        {
            dialog.add_toast($toast);
        } else if let Some(root) = $widget.root() {
            #[allow(deprecated)]
            if let Some(window) = root.downcast_ref::<adw::PreferencesWindow>() {
                use adw::prelude::PreferencesWindowExt;
                window.add_toast($toast);
            } else if let Some(window) = root.downcast_ref::<$crate::ui::widgets::window::Window>()
            {
                window.add_toast($toast);
            } else {
                panic!("Trying to display a toast when the parent doesn't support it");
            }
        }
    }};
}

#[macro_export]
macro_rules! toast {
    ($widget:expr, $message:expr) => {{
        $crate::_add_toast!(
            $widget,
            adw::Toast::builder()
                .timeout(2)
                .use_markup(false)
                .title($message)
                .build()
        );
    }};
}

#[macro_export]
macro_rules! fraction {
    ($widget:expr) => {{
        use gtk::prelude::WidgetExt;
        if let Some(root) = $widget.root() {
            if let Some(window) = root.downcast_ref::<$crate::ui::widgets::window::Window>() {
                window.set_progressbar_fade();
            }
        }
    }};
}

#[macro_export]
macro_rules! fraction_reset {
    ($widget:expr) => {{
        use gtk::prelude::WidgetExt;
        if let Some(root) = $widget.root() {
            if let Some(window) = root.downcast_ref::<$crate::ui::widgets::window::Window>() {
                window.set_progressbar_opacity(1.0);
                window.hard_set_fraction(0.0);
                window.set_fraction(1.0);
            }
        }
    }};
}

#[macro_export]
macro_rules! insert_editm_dialog {
    ($widget:expr, $dialog:expr) => {{
        use adw::prelude::*;
        use gtk::prelude::WidgetExt;
        if let Some(root) = $widget.root() {
            if let Some(window) = root.downcast_ref::<$crate::ui::widgets::window::Window>() {
                $dialog.present(Some(window));
            } else {
                panic!("Trying to display a dialog when the parent doesn't support it");
            }
        }
    }};
}

#[macro_export]
macro_rules! bing_song_model {
    ($widget:expr, $active_model:expr, $active_core_song:expr) => {{
        use adw::prelude::*;
        use gtk::prelude::WidgetExt;
        if let Some(root) = $widget.root() {
            if let Some(window) = root.downcast_ref::<$crate::ui::widgets::window::Window>() {
                spawn(glib::clone!(
                    #[weak]
                    window,
                    #[weak(rename_to = active_core_song)]
                    $active_core_song,
                    async move {
                        window
                            .bind_song_model($active_model, active_core_song)
                            .await;
                    }
                ));
            } else {
                panic!("Trying to bind song model when the parent doesn't support it");
            }
        }
    }};
}

#[macro_export]
macro_rules! dyn_event {
    ($lvl:ident, $($arg:tt)+) => {
        match $lvl {
            ::gtk::glib::LogLevel::Debug => ::tracing::debug!($($arg)+),
            ::gtk::glib::LogLevel::Message | ::gtk::glib::LogLevel::Info => ::tracing::info!($($arg)+),
            ::gtk::glib::LogLevel::Warning => ::tracing::warn!($($arg)+),
            ::gtk::glib::LogLevel::Error | ::gtk::glib::LogLevel::Critical  => ::tracing::error!($($arg)+),
        }
    };
}

#[macro_export]
macro_rules! close_on_error {
    ($widget:expr, $des:expr) => {{
        use gtk::prelude::WidgetExt;
        if let Some(root) = $widget.root() {
            if let Some(window) = root.downcast_ref::<$crate::ui::widgets::window::Window>() {
                window.close_on_error($des);
            }
        }
    }};
}

#[macro_export]
macro_rules! alert_dialog {
    ($widget:expr, $dialog:expr) => {{
        use gtk::prelude::WidgetExt;
        if let Some(root) = $widget.root() {
            if let Some(window) = root.downcast_ref::<$crate::ui::widgets::window::Window>() {
                window.alert_dialog($dialog);
            }
        }
    }};
}
