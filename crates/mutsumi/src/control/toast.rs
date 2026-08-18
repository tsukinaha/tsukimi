use gtk::prelude::*;

// need manually impl GlobalToast for widget
pub trait GlobalToast: IsA<gtk::Widget> {
    fn toast_overlay(&self) -> Option<adw::ToastOverlay> {
        self.upcast_ref::<gtk::Widget>()
            .ancestor(adw::ToastOverlay::static_type())
            .and_downcast()
    }

    fn toast(&self, message: impl Into<String>) {
        let message = message.into();

        let Some(overlay) = self.toast_overlay() else {
            tracing::warn!("No ToastOverlay ancestor, dropping toast: {message}");
            return;
        };

        let toast = adw::Toast::builder()
            .timeout(2)
            .use_markup(false)
            .title(message)
            .build();
        overlay.add_toast(toast);
    }
}
