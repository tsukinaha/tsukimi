use glib::Object;
use gtk::{gio, glib};

pub mod imp {
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{gdk, glib, graphene, CompositeTemplate};
    use gtk::prelude::*;
    use once_cell::sync::Lazy;

    static MASK: Lazy<gdk::RGBA> = Lazy::new(|| {
        if gtk::Settings::default()
            .map(|s| s.is_gtk_application_prefer_dark_theme())
            .unwrap_or(false)
        {
            gdk::RGBA::new(0.0, 0.0, 0.0, 0.3)
        } else {
            gdk::RGBA::new(1.0, 1.0, 1.0, 0.3)
        }
    });

    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsukimi/item_carousel.ui")]
    pub struct ItemCarousel {
        #[template_child]
        pub backdrop: TemplateChild<gtk::Picture>,
        #[template_child]
        pub carousel: TemplateChild<adw::Carousel>,
        #[template_child]
        pub backrevealer: TemplateChild<gtk::Revealer>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for ItemCarousel {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "ItemCarousel";
        type Type = super::ItemCarousel;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ItemCarousel {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for ItemCarousel {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let obj = self.obj();

            let width = obj.width() as f32;
            let height = obj.height() as f32;

            snapshot.save();
            let upper_height = (2.0 * height) / 3.0;
            snapshot.push_clip(&graphene::Rect::new(0.0, 0.0, width, upper_height));
            self.parent_snapshot(snapshot);
            snapshot.pop(); 
            snapshot.restore();

            snapshot.save();
            let lower_y = (2.0 * height) / 3.0;
            let lower_height = height / 3.0;
            snapshot.push_clip(&graphene::Rect::new(0.0, lower_y, width, lower_height));
            snapshot.push_blur(30.0);
            self.parent_snapshot(snapshot);
            snapshot.pop(); 

            snapshot.append_color(
                &MASK,
                &graphene::Rect::new(0.0, lower_y, width, lower_height),
            );

            snapshot.pop();
            snapshot.restore();
        }
    }

    impl BinImpl for ItemCarousel {}

    impl ItemCarousel {}
}

glib::wrapper! {
    pub struct ItemCarousel(ObjectSubclass<imp::ItemCarousel>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl ItemCarousel {
    pub fn new() -> Self {
        Object::new()
    }
}
