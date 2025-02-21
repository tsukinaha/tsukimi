use adw::subclass::prelude::*;
use gtk::{
    CompositeTemplate,
    glib,
    prelude::*,
};

use super::EpisodeButton;

mod imp {

    use glib::subclass::InitializingObject;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/episode_switcher.ui")]
    pub struct EpisodeSwitcher {
        #[template_child]
        pub container: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpisodeSwitcher {
        const NAME: &'static str = "EpisodeSwitcher";
        type Type = super::EpisodeSwitcher;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EpisodeSwitcher {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for EpisodeSwitcher {}

    impl BinImpl for EpisodeSwitcher {}
}

glib::wrapper! {
    /// A widget displaying a `EpisodeSwitcher`.
    pub struct EpisodeSwitcher(ObjectSubclass<imp::EpisodeSwitcher>)
        @extends gtk::Widget, adw::Bin, @implements gtk::Accessible;
}

#[gtk::template_callbacks]
impl EpisodeSwitcher {
    const EPISODES_PER_GROUP: usize = 50;

    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn clear(&self) {
        while let Some(widget) = self.imp().container.last_child() {
            self.imp().container.remove(&widget);
        }
    }

    pub fn add_button_from_index<F>(&self, start_index: u32, length: u32, callback: F)
    where
        F: Fn(&EpisodeButton) + 'static,
    {
        let button = EpisodeButton::new(start_index, length);

        button.connect_clicked(callback);

        self.add_button(&button);
    }

    pub fn load_from_n_items<F>(&self, n_items: usize, callback: F)
    where
        F: Fn(&EpisodeButton) + 'static + Clone,
    {
        self.clear();

        for i in (0..n_items).step_by(Self::EPISODES_PER_GROUP) {
            let start = i;
            let end = (i + Self::EPISODES_PER_GROUP).min(n_items);

            let callback = callback.clone();
            let cb = move |btn: &EpisodeButton| {
                callback(btn);
            };

            self.add_button_from_index(start as u32, (end - start) as u32, cb);
        }
    }

    fn add_button(&self, widget: &EpisodeButton) {
        self.imp().container.append(widget);
    }

    // object attribute with swapped will not return the self
    #[template_callback]
    fn scroll_cb(scrolled: &gtk::ScrolledWindow, _dx: f64, dy: f64) -> bool {
        if dy == 0.0 {
            return false;
        }

        let hadj = scrolled.hadjustment();
        let value = hadj.value();
        let step = hadj.step_increment();

        hadj.set_value(value + dy * step);
        true
    }
}

impl Default for EpisodeSwitcher {
    fn default() -> Self {
        Self::new()
    }
}
