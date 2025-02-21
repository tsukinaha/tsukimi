use glib::Object;
use gtk::{
    gio,
    glib,
};

pub mod imp {
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        gdk,
        glib,
        graphene,
        prelude::*,
    };
    use once_cell::sync::Lazy;

    use super::CUBIC_POINTS;

    static MASK: Lazy<gdk::RGBA> = Lazy::new(|| {
        if gtk::Settings::default()
            .map(|s| s.is_gtk_application_prefer_dark_theme())
            .unwrap_or(false)
        {
            gdk::RGBA::new(0.0, 0.0, 0.0, 0.4)
        } else {
            gdk::RGBA::new(1.0, 1.0, 1.0, 0.2)
        }
    });

    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/item_carousel.ui")]
    pub struct ItemCarousel {
        #[template_child]
        pub backdrop: TemplateChild<gtk::Picture>,
        #[template_child]
        pub carousel: TemplateChild<adw::Carousel>,
        #[template_child]
        pub backrevealer: TemplateChild<gtk::Revealer>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ItemCarousel {
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

    impl WidgetImpl for ItemCarousel {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            // blur the lower 1/3 of the widget, and apply a linear gradient
            let obj = self.obj();

            let width = obj.width() as f32;
            let height = obj.height() as f32;

            let start_point = graphene::Point::new(0.0, 0.0);
            let end_point = graphene::Point::new(0.0, height);
            let rect = graphene::Rect::new(0.0, 0.0, width, height);

            let upper_height = (height - 300.0).max(0.0);

            let heights = [300.0, 290.0, 280.0, 270.0, 260.0, 250.0, 240.0, 230.0];
            let stops: Vec<gtk::gsk::ColorStop> = heights
                .iter()
                .enumerate()
                .map(|(i, &h)| {
                    let height_ratio = (height - h).max(0.0) / height;
                    gtk::gsk::ColorStop::new(
                        f32::min(1.0, height_ratio),
                        gdk::RGBA::new(
                            CUBIC_POINTS[i] as f32,
                            CUBIC_POINTS[i] as f32,
                            CUBIC_POINTS[i] as f32,
                            1.0,
                        ),
                    )
                })
                .collect();

            snapshot.save();
            snapshot.push_clip(&graphene::Rect::new(0.0, 0.0, width, upper_height));
            self.parent_snapshot(snapshot);
            snapshot.pop();
            snapshot.restore();

            snapshot.save();
            snapshot.push_clip(&graphene::Rect::new(0.0, upper_height, width, 300.0));
            snapshot.push_blur(35.0);
            self.parent_snapshot(snapshot);
            snapshot.pop();

            snapshot.append_color(&MASK, &graphene::Rect::new(0.0, upper_height, width, 300.0));

            snapshot.pop();
            snapshot.restore();

            snapshot.push_mask(gtk::gsk::MaskMode::Luminance);
            snapshot.append_linear_gradient(&rect, &start_point, &end_point, &stops);
            snapshot.pop();

            self.parent_snapshot(snapshot);

            snapshot.pop();
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

impl Default for ItemCarousel {
    fn default() -> Self {
        Self::new()
    }
}

impl ItemCarousel {
    pub fn new() -> Self {
        Object::new()
    }
}

pub const CUBIC_POINTS: [f64; 8] = [
    1.0,
    0.629737609329446,
    0.3644314868804665,
    0.1865889212827988,
    0.07871720116618075,
    0.02332361516034985,
    0.0029154518950437313,
    0.0,
];

#[cfg(test)]
mod tests {
    use super::*;

    fn generate_cubic_points() -> [f64; 8] {
        let mut points = [0.0; 8];
        points.iter_mut().enumerate().for_each(|(i, point)| {
            *point = ((8 - i - 1) as f64 / 7.0).powi(3);
        });
        points
    }

    #[test]
    fn test_generate_cubic_points() {
        let points = generate_cubic_points();
        assert_eq!(points, CUBIC_POINTS);
    }
}
