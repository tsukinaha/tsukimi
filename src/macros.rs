#[doc(hidden)]
#[macro_export]
macro_rules! _add_toast {
    ($widget:expr, $toast:expr) => {{
        use gtk::prelude::WidgetExt;
        if let Some(dialog) = $widget
            .ancestor(adw::PreferencesDialog::static_type())
            .and_downcast::<adw::PreferencesDialog>()
        {
            use adw::prelude::PreferencesDialogExt;
            dialog.add_toast($toast);
        } else if let Some(dialog) = $widget
            .ancestor(adw::ToastOverlay::static_type())
            .and_downcast::<adw::ToastOverlay>()
        {
            dialog.add_toast($toast);
        } else if let Some(root) = $widget.root() {
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
                window.bind_song_model($active_model, $active_core_song);
            } else {
                panic!("Trying to display a toast when the parent doesn't support it");
            }
        }
    }};
}
