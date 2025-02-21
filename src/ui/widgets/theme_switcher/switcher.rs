use adw::subclass::prelude::*;
use gtk::{
    CompositeTemplate,
    gio,
    glib,
    prelude::*,
};

use crate::ui::models::SETTINGS;

mod imp {

    use glib::subclass::InitializingObject;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/theme_switcher.ui")]
    pub struct ThemeSwitcher {}

    #[glib::object_subclass]
    impl ObjectSubclass for ThemeSwitcher {
        const NAME: &'static str = "ThemeSwitcher";
        type Type = super::ThemeSwitcher;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ThemeSwitcher {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().init();
        }
    }

    impl WidgetImpl for ThemeSwitcher {}

    impl BinImpl for ThemeSwitcher {}
}

glib::wrapper! {
    /// A widget displaying a `ThemeSwitcher`.
    pub struct ThemeSwitcher(ObjectSubclass<imp::ThemeSwitcher>)
        @extends gtk::Widget, adw::Bin, @implements gtk::Accessible;
}

impl ThemeSwitcher {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn init(&self) {
        self.set_theme(SETTINGS.main_theme());
        let action_group = gio::SimpleActionGroup::new();
        let action_vo = gio::ActionEntry::builder("color-scheme")
            .parameter_type(Some(&i32::static_variant_type()))
            .state(SETTINGS.main_theme().to_variant())
            .activate(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |_, action, parameter| {
                    let parameter = parameter
                        .expect("Could not get parameter.")
                        .get::<i32>()
                        .expect("The variant needs to be of type `i32`.");

                    SETTINGS.set_main_theme(parameter).unwrap();
                    obj.set_theme(parameter);

                    action.set_state(&parameter.to_variant());
                }
            ))
            .build();

        action_group.add_action_entries([action_vo]);
        self.insert_action_group("app", Some(&action_group));
    }

    pub fn set_theme(&self, theme: i32) {
        let style_manager = adw::StyleManager::default();

        match theme {
            1 => {
                style_manager.set_color_scheme(adw::ColorScheme::Default);
            }
            2 => {
                style_manager.set_color_scheme(adw::ColorScheme::ForceLight);
            }
            _ => {
                style_manager.set_color_scheme(adw::ColorScheme::ForceDark);
            }
        }
    }
}

impl Default for ThemeSwitcher {
    fn default() -> Self {
        Self::new()
    }
}
