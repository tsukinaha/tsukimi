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
        $crate::_add_toast!($widget, adw::Toast::new($message.as_ref()));
    }};
}

#[macro_export]
macro_rules! fraction {
    ($widget:expr) => {{
        use gtk::prelude::WidgetExt;
        if let Some(root) = $widget.root() {
            if let Some(window) = root.downcast_ref::<$crate::ui::widgets::window::Window>() {
                window.set_fraction(0.0);
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
                window.set_fraction(1.0);
            }
        }
    }};
}
