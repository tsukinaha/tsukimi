use gtk::{
    gio,
    prelude::*,
};
use mpris_server::zbus::fdo;

pub async fn raise_window() -> fdo::Result<()> {
    if let Some(app) = gio::Application::default() {
        if let Ok(gtk_app) = app.downcast::<gtk::Application>() {
            if let Some(window) = gtk_app.active_window() {
                window.present();
                return Ok(());
            }
        }
    }
    Err(fdo::Error::Failed("Failed to raise window".to_string()))
}

pub async fn quit_application() -> fdo::Result<()> {
    if let Some(app) = gio::Application::default() {
        app.quit();
        Ok(())
    } else {
        Err(fdo::Error::Failed("Failed to quit".to_string()))
    }
}
