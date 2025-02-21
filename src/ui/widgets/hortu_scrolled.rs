use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gtk::{
    CompositeTemplate,
    gio,
    glib,
    template_callbacks,
};

use crate::{
    client::structs::SimpleListItem,
    ui::{
        provider::tu_object::TuObject,
        widgets::fix::ScrolledWindowFixExt,
    },
};

pub const SHOW_BUTTON_ANIMATION_DURATION: u32 = 500;

mod imp {
    use std::cell::{
        OnceCell,
        RefCell,
    };

    use glib::subclass::InitializingObject;
    use gtk::{
        SignalListItemFactory,
        gio,
    };

    use super::*;
    use crate::ui::widgets::{
        tu_list_item::imp::PosterType,
        utils::TuItemBuildExt,
    };

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/hortu_scrolled.ui")]
    #[properties(wrapper_type = super::HortuScrolled)]
    pub struct HortuScrolled {
        #[property(get, set, construct_only, default_value = false)]
        pub isresume: OnceCell<bool>,
        #[template_child]
        pub label: TemplateChild<gtk::Label>,
        #[template_child]
        pub scrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub list: TemplateChild<gtk::ListView>,
        #[template_child]
        pub revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub morebutton: TemplateChild<gtk::Button>,
        #[template_child]
        pub left_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub right_button: TemplateChild<gtk::Button>,

        #[property(get, set, default_value = false)]
        pub moreview: RefCell<bool>,
        #[property(get, set)]
        pub title: RefCell<String>,

        pub show_button_animation: OnceCell<adw::TimedAnimation>,
        pub hide_button_animation: OnceCell<adw::TimedAnimation>,

        pub selection: gtk::SingleSelection,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HortuScrolled {
        const NAME: &'static str = "HortuScrolled";
        type Type = super::HortuScrolled;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for HortuScrolled {
        fn constructed(&self) {
            self.selection.set_autoselect(false);

            self.parent_constructed();

            self.scrolled.fix();

            let store = gio::ListStore::new::<TuObject>();

            self.selection.set_model(Some(&store));

            self.list.set_model(Some(&self.selection));

            self.list.set_factory(Some(
                SignalListItemFactory::new().tu_item(PosterType::default()),
            ));

            self.list.connect_activate(move |listview, position| {
                let model = listview.model().unwrap();
                let tu_obj = model.item(position).and_downcast::<TuObject>().unwrap();
                tu_obj.activate(listview);
            });
        }
    }

    impl WidgetImpl for HortuScrolled {}

    impl BinImpl for HortuScrolled {}
}

glib::wrapper! {
    /// A scrolled list of items.
    pub struct HortuScrolled(ObjectSubclass<imp::HortuScrolled>)
        @extends gtk::Widget, adw::Bin, @implements gtk::Accessible;
}

impl Default for HortuScrolled {
    fn default() -> Self {
        Self::new()
    }
}

#[template_callbacks]
impl HortuScrolled {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_morebutton(&self) {
        let imp = self.imp();
        imp.morebutton.set_visible(true);
    }

    pub fn set_items(&self, items: &[SimpleListItem]) {
        let imp = self.imp();

        let store = imp
            .selection
            .model()
            .unwrap()
            .downcast::<gio::ListStore>()
            .unwrap();

        store.remove_all();

        if items.is_empty() {
            self.set_visible(false);
            return;
        }

        self.set_visible(true);

        let items = items
            .iter()
            .map(|item| {
                let object = TuObject::from_simple(item, None);
                object.item().set_is_resume(self.isresume());
                object
            })
            .collect::<Vec<_>>();

        store.extend_from_slice(&items);

        imp.revealer.set_reveal_child(true);
    }

    fn set_control_opacity(&self, opacity: f64) {
        let imp = self.imp();
        imp.left_button.set_opacity(opacity);
        imp.right_button.set_opacity(opacity);
    }

    fn are_controls_visible(&self) -> bool {
        if self.hide_controls_animation().state() == adw::AnimationState::Playing {
            return false;
        }

        self.imp().left_button.opacity() >= 0.68
            || self.show_controls_animation().state() == adw::AnimationState::Playing
    }

    fn show_controls_animation(&self) -> &adw::TimedAnimation {
        self.imp().show_button_animation.get_or_init(|| {
            let target = adw::CallbackAnimationTarget::new(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |opacity| obj.set_control_opacity(opacity)
            ));

            adw::TimedAnimation::builder()
                .duration(SHOW_BUTTON_ANIMATION_DURATION)
                .widget(&self.imp().scrolled.get())
                .target(&target)
                .value_to(0.7)
                .build()
        })
    }

    fn hide_controls_animation(&self) -> &adw::TimedAnimation {
        self.imp().hide_button_animation.get_or_init(|| {
            let target = adw::CallbackAnimationTarget::new(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |opacity| obj.set_control_opacity(opacity)
            ));

            adw::TimedAnimation::builder()
                .duration(SHOW_BUTTON_ANIMATION_DURATION)
                .widget(&self.imp().scrolled.get())
                .target(&target)
                .value_to(0.)
                .build()
        })
    }

    #[template_callback]
    fn on_rightbutton_clicked(&self) {
        self.anime::<true>();
    }

    fn controls_opacity(&self) -> f64 {
        self.imp().left_button.opacity()
    }

    #[template_callback]
    fn on_enter_focus(&self) {
        if !self.are_controls_visible() {
            self.hide_controls_animation().pause();
            self.show_controls_animation()
                .set_value_from(self.controls_opacity());
            self.show_controls_animation().play();
        }
    }

    #[template_callback]
    fn on_leave_focus(&self) {
        if self.are_controls_visible() {
            self.show_controls_animation().pause();
            self.hide_controls_animation()
                .set_value_from(self.controls_opacity());
            self.hide_controls_animation().play();
        }
    }

    #[template_callback]
    fn on_leftbutton_clicked(&self) {
        self.anime::<false>();
    }

    pub fn connect_morebutton<F>(&self, cb: F)
    where
        F: Fn(&gtk::Button) + 'static,
    {
        self.imp().morebutton.connect_clicked(cb);
    }

    fn anime<const R: bool>(&self) {
        let scrolled = self.imp().scrolled.get();
        let adj = scrolled.hadjustment();

        let Some(clock) = scrolled.frame_clock() else {
            return;
        };

        let start = adj.value();
        let end = if R { start + 800.0 } else { start - 800.0 };

        let start_time = clock.frame_time();
        let end_time = start_time + 1000 * 400;

        scrolled.add_tick_callback(move |_view, clock| {
            let now = clock.frame_time();
            if now < end_time && adj.value() != end {
                let mut t = (now - start_time) as f64 / (end_time - start_time) as f64;
                t = Self::ease_in_out_cubic(t);
                adj.set_value(start + t * (end - start));
                glib::ControlFlow::Continue
            } else {
                adj.set_value(end);
                glib::ControlFlow::Break
            }
        });
    }

    fn ease_in_out_cubic(t: f64) -> f64 {
        if t < 0.5 {
            4.0 * t * t * t
        } else {
            let t = 2.0 * t - 2.0;
            0.5 * t * t * t + 1.0
        }
    }
}
