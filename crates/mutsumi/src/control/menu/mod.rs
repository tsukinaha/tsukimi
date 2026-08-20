use adw::subclass::prelude::*;
use gtk::{CompositeTemplate, glib, prelude::*};

mod imp {

    use glib::subclass::InitializingObject;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/io/github/mutsumiuniverse/mutsumi/ui/menu_actions.ui")]
    pub struct MenuActions {
        #[template_child]
        pub play_pause_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MenuActions {
        const NAME: &'static str = "MenuActions";
        type Type = super::MenuActions;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MenuActions {}

    impl WidgetImpl for MenuActions {}

    impl BinImpl for MenuActions {}
}

glib::wrapper! {
    /// The play / pause / seek button row embedded in the context menu.
    pub struct MenuActions(ObjectSubclass<imp::MenuActions>)
        @extends gtk::Widget, adw::Bin, @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl MenuActions {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_paused(&self, paused: bool) {
        let button = self.imp().play_pause_button.get();
        if paused {
            button.set_icon_name("media-playback-start-symbolic");
            button.set_tooltip_text(Some("Play"));
        } else {
            button.set_icon_name("media-playback-pause-symbolic");
            button.set_tooltip_text(Some("Pause"));
        }
    }
}

impl Default for MenuActions {
    fn default() -> Self {
        Self::new()
    }
}
