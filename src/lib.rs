pub mod control;
mod danmaku;
mod error;
pub mod video;

pub use control::*;
pub use video::*;

pub fn control_init() {
    control::init();
}

#[cfg(test)]
mod tests {
    use gtk::gio;

    #[test]
    fn registers_control_template_resources() {
        crate::control::register_resources();

        for path in [
            "/io/github/mutsumi/ui/mpv_control_sidebar.ui",
            "/io/github/mutsumi/ui/menu_actions.ui",
            "/io/github/mutsumi/ui/volume_bar.ui",
        ] {
            gio::resources_lookup_data(path, gio::ResourceLookupFlags::NONE)
                .unwrap_or_else(|err| panic!("missing resource {path}: {err}"));
        }
    }
}
