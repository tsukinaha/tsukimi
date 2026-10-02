#![cfg_attr(all(windows, not(feature = "console")), windows_subsystem = "windows")]

fn main() -> gtk::glib::ExitCode {
    tsukimi::run()
}
