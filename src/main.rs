#![cfg_attr(
    all(target_os = "windows", not(feature = "console")),
    windows_subsystem = "windows"
)]

fn main() -> gtk::glib::ExitCode {
    tsukimi::run()
}
