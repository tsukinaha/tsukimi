use adw::subclass::prelude::*;
use gtk::{
    CompositeTemplate,
    glib,
};

mod imp {

    use std::cell::RefCell;

    use glib::subclass::InitializingObject;
    use gtk::prelude::*;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/volume_bar.ui")]
    #[properties(wrapper_type = super::VolumeBar)]
    pub struct VolumeBar {
        #[property(get, set = Self::set_level, default_value = 100.0)]
        pub level: RefCell<f64>,
        #[template_child]
        pub progress: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub revealer: TemplateChild<gtk::Revealer>,

        pub timeout: RefCell<Option<glib::source::SourceId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VolumeBar {
        const NAME: &'static str = "VolumeBar";
        type Type = super::VolumeBar;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for VolumeBar {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for VolumeBar {}

    impl BinImpl for VolumeBar {}

    impl VolumeBar {
        fn set_level(&self, level: f64) {
            self.level.replace(level);
            self.progress.set_fraction(level);
            self.progress.remove_css_class("warning");

            match level {
                0.0 => self.icon.set_icon_name(Some("audio-volume-muted-symbolic")),
                level if level < 0.33 => self.icon.set_icon_name(Some("audio-volume-low-symbolic")),
                level if level < 0.66 => self
                    .icon
                    .set_icon_name(Some("audio-volume-medium-symbolic")),
                level if level <= 1.0 => {
                    self.icon.set_icon_name(Some("audio-volume-high-symbolic"))
                }
                _ => {
                    self.icon
                        .set_icon_name(Some("audio-volume-overamplified-symbolic"));
                    self.progress.add_css_class("warning");
                }
            }

            self.revealer.set_reveal_child(true);

            self.remove_timeout();

            let source_id = glib::timeout_add_seconds_local_once(
                2,
                glib::clone!(
                    #[weak(rename_to = imp)]
                    self,
                    move || {
                        imp.revealer.set_reveal_child(false);
                        imp.timeout.replace(None);
                    }
                ),
            );

            self.timeout.replace(Some(source_id));
        }

        fn remove_timeout(&self) {
            if let Some(source_id) = self.timeout.take() {
                glib::source::SourceId::remove(source_id);
            }
        }
    }
}

glib::wrapper! {
    /// A widget displaying a `VolumeBar`.
    pub struct VolumeBar(ObjectSubclass<imp::VolumeBar>)
        @extends gtk::Widget, adw::Bin, @implements gtk::Accessible;
}

impl VolumeBar {
    pub fn new() -> Self {
        glib::Object::new()
    }
}

impl Default for VolumeBar {
    fn default() -> Self {
        Self::new()
    }
}
