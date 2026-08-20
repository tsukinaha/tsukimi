use adw::subclass::prelude::*;
use gtk::{CompositeTemplate, glib, prelude::*};

mod imp {
    use std::cell::{Cell, RefCell};

    use glib::subclass::InitializingObject;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/io/github/mutsumiuniverse/mutsumi/ui/scale_row.ui")]
    #[properties(wrapper_type = super::ScaleRow)]
    pub struct ScaleRow {
        #[template_child]
        pub scale: TemplateChild<gtk::Scale>,
        #[template_child]
        pub adjustment: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub current_label: TemplateChild<gtk::Label>,

        #[property(get, set = Self::set_model, explicit_notify, nullable)]
        pub model: RefCell<Option<gtk::StringList>>,

        #[property(get, set = Self::set_value, explicit_notify, default_value = 0.0)]
        pub value: Cell<f64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ScaleRow {
        const NAME: &'static str = "ScaleRow";
        type Type = super::ScaleRow;
        type ParentType = adw::PreferencesRow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ScaleRow {
        fn constructed(&self) {
            self.parent_constructed();
            self.refresh_scale();
        }
    }

    impl WidgetImpl for ScaleRow {}
    impl ListBoxRowImpl for ScaleRow {}
    impl PreferencesRowImpl for ScaleRow {}

    impl ScaleRow {
        fn set_model(&self, model: Option<gtk::StringList>) {
            self.model.replace(model);
            self.refresh_scale();
            self.update_current_label();
            self.obj().notify_model();
        }

        fn set_value(&self, value: f64) {
            if (self.value.get() - value).abs() <= f64::EPSILON {
                return;
            }

            self.value.set(value);

            let adjustment = self.adjustment.get();
            let clamped = value.clamp(adjustment.lower(), adjustment.upper());
            if (adjustment.value() - clamped).abs() > f64::EPSILON {
                adjustment.set_value(clamped);
            }

            self.update_current_label();
            self.obj().notify_value();
        }

        fn refresh_scale(&self) {
            self.scale.clear_marks();

            let count = self
                .model
                .borrow()
                .as_ref()
                .map(|model| model.n_items())
                .unwrap_or(0);

            let upper = count.saturating_sub(1) as f64;
            self.adjustment.set_lower(0.0);
            self.adjustment.set_upper(upper);
            self.adjustment.set_step_increment(1.0);
            self.adjustment.set_page_increment(1.0);

            for idx in 0..count {
                self.scale
                    .add_mark(idx as f64, gtk::PositionType::Bottom, None);
            }

            self.scale.set_sensitive(count > 1);

            if count == 0 {
                if self.adjustment.value().abs() > f64::EPSILON {
                    self.adjustment.set_value(0.0);
                }
                self.current_label.set_label("");
                return;
            }

            let clamped = self.value.get().clamp(0.0, upper);
            if (self.value.get() - clamped).abs() > f64::EPSILON {
                self.value.set(clamped);
                self.obj().notify_value();
            }

            if (self.adjustment.value() - clamped).abs() > f64::EPSILON {
                self.adjustment.set_value(clamped);
            }

            self.update_current_label();
        }

        fn update_current_label(&self) {
            let index = self.value.get().round().max(0.0) as u32;
            let text = self
                .model
                .borrow()
                .as_ref()
                .and_then(|model| model.string(index))
                .map(|text| text.to_string())
                .unwrap_or_default();
            self.current_label.set_label(&text);
        }
    }
}

glib::wrapper! {
    pub struct ScaleRow(ObjectSubclass<imp::ScaleRow>)
        @extends adw::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl ScaleRow {
    pub fn new() -> Self {
        glib::Object::new()
    }
}

impl Default for ScaleRow {
    fn default() -> Self {
        Self::new()
    }
}
