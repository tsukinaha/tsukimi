use gtk::prelude::*;
use gtk::Application;
mod ui;

#[tokio::main]
async fn main() {
    let app = Application::builder()
        .application_id("moe.tsuna.tsukimi")
        .build();
    app.connect_startup(|_| ui::load_css());
    app.connect_activate(ui::build_ui);
    app.run();
}