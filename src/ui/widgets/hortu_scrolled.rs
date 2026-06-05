use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gtk::{
    CompositeTemplate,
    glib,
    template_callbacks,
};

use crate::{
    client::structs::SimpleListItem,
    ui::{
        provider::{
            tu_item::{
                PreferPoster,
                PreferSize,
            },
            tu_object::TuObject,
        },
        widgets::{
            fix::ScrolledWindowFixExt,
            lazy_diff_view::LazyDiffView,
            tu_list_item::{
                TuListItem,
                imp::PosterType,
            },
        },
    },
};

pub const SHOW_BUTTON_ANIMATION_DURATION: u32 = 500;

#[derive(Default, Hash, Eq, PartialEq, Clone, Copy, glib::Enum, Debug)]
#[repr(u32)]
#[enum_type(name = "UnifySize")]
pub enum UnifySize {
    #[default]
    Disable,
    Majority,
    ForceVideo,
    ForcePost,
}

pub fn resolve_prefer_size(unify_size: UnifySize, items: &[SimpleListItem]) -> PreferSize {
    match unify_size {
        UnifySize::Disable => PreferSize::Auto,
        UnifySize::ForceVideo => PreferSize::Video,
        UnifySize::ForcePost => PreferSize::Post,
        UnifySize::Majority => {
            let primary_ratio: Vec<_> = items
                .iter()
                .filter(|i| i.item_type != "Episode")
                .filter_map(|i| i.primary_image_aspect_ratio)
                .collect();
            if primary_ratio.is_empty() {
                return PreferSize::Auto;
            }
            let video_percentage = primary_ratio.iter().filter(|i| **i > 1.0).count() as f64
                / primary_ratio.len() as f64;
            match video_percentage {
                p if p > 0.8 => PreferSize::Video,
                p if p < 0.2 => PreferSize::Post,
                _ => PreferSize::Auto,
            }
        }
    }
}

mod imp {
    use std::{
        cell::{
            Cell,
            OnceCell,
            RefCell,
        },
        collections::HashMap,
    };

    use glib::subclass::InitializingObject;
    use gtk::prelude::Cast;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/hortu_scrolled.ui")]
    #[properties(wrapper_type = super::HortuScrolled)]
    pub struct HortuScrolled {
        #[property(get, set, construct_only, default_value = false)]
        pub is_resume: OnceCell<bool>,
        #[template_child]
        pub label: TemplateChild<gtk::Label>,
        #[template_child]
        pub diffview: TemplateChild<LazyDiffView>,
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

        #[property(get, set, builder(UnifySize::default()))]
        pub unify_size: RefCell<UnifySize>,

        #[property(get, set, builder(PreferPoster::default()))]
        pub prefer_poster: RefCell<PreferPoster>,

        pub show_left_animation: OnceCell<adw::TimedAnimation>,
        pub hide_left_animation: OnceCell<adw::TimedAnimation>,
        pub show_right_animation: OnceCell<adw::TimedAnimation>,
        pub hide_right_animation: OnceCell<adw::TimedAnimation>,
        pub is_hovering: Cell<bool>,
        pub item_cache: RefCell<HashMap<String, TuObject>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HortuScrolled {
        const NAME: &'static str = "HortuScrolled";
        type Type = super::HortuScrolled;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            LazyDiffView::ensure_type();
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
            self.parent_constructed();

            self.diffview.set_orientation(gtk::Orientation::Horizontal);
            self.diffview
                .scroll()
                .fix()
                .set_hscrollbar_policy(gtk::PolicyType::Never);
            self.diffview.configure(
                |tu_obj: &TuObject| tu_obj.item().key(),
                |_tu_obj: &TuObject| {
                    let tu_item = TuListItem::default();
                    tu_item.set_poster_type(PosterType::default());

                    let gesture = gtk::GestureClick::new();
                    gesture.set_button(1);
                    gesture.connect_released(glib::clone!(
                        #[weak]
                        tu_item,
                        move |gesture, _, _, _| {
                            gesture.set_state(gtk::EventSequenceState::Claimed);
                            tu_item.item().activate(&tu_item);
                        }
                    ));
                    tu_item.add_controller(gesture);

                    tu_item.upcast::<gtk::Widget>()
                },
                |widget, tu_obj: &TuObject| {
                    let tu_item = widget
                        .downcast_ref::<TuListItem>()
                        .expect("LazyDiffView row must be a TuListItem");
                    tu_item.set_item(tu_obj.item());
                },
            );

            let adj = self.diffview.scroll().hadjustment();
            adj.connect_value_changed(glib::clone!(
                #[weak(rename_to = obj)]
                self.obj(),
                move |_| {
                    obj.update_left_button(true);
                    obj.update_right_button(true);
                }
            ));
        }
    }

    impl WidgetImpl for HortuScrolled {}

    impl BinImpl for HortuScrolled {}
}

glib::wrapper! {
    /// A scrolled list of items.
    pub struct HortuScrolled(ObjectSubclass<imp::HortuScrolled>)
        @extends gtk::Widget, adw::Bin, @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
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

    pub fn set_items(&self, items: Vec<SimpleListItem>) {
        let imp = self.imp();

        if items.is_empty() {
            imp.diffview.set_items(Vec::<TuObject>::new());
            self.set_visible(false);
            return;
        }

        self.set_visible(true);

        let prefer_size = resolve_prefer_size(self.unify_size(), &items);
        let visible_ids = items
            .iter()
            .map(|item| item.id.as_str())
            .collect::<std::collections::HashSet<_>>();
        imp.item_cache
            .borrow_mut()
            .retain(|id, _| visible_ids.contains(id.as_str()));

        let items = items
            .into_iter()
            .map(|item| {
                let mut cache = imp.item_cache.borrow_mut();
                let object = if let Some(object) = cache.get(&item.id) {
                    object.clone()
                } else {
                    let object = TuObject::from_simple(item.to_owned());
                    cache.insert(object.item().key(), object.clone());
                    object
                };
                let tu_item = object.item();
                tu_item.update_user_data(&item.user_data);
                tu_item.set_is_resume(self.is_resume());
                tu_item.set_prefer_size(prefer_size);
                tu_item.set_prefer_poster(self.prefer_poster());
                object
            })
            .collect::<Vec<_>>();

        imp.diffview.set_items(items);

        imp.revealer.set_reveal_child(true);
    }

    fn set_left_opacity(&self, opacity: f64) {
        let btn = &self.imp().left_button;
        btn.set_visible(opacity > 0.);
        btn.set_opacity(opacity);
    }

    fn set_right_opacity(&self, opacity: f64) {
        let btn = &self.imp().right_button;
        btn.set_visible(opacity > 0.);
        btn.set_opacity(opacity);
    }

    fn left_opacity(&self) -> f64 {
        self.imp().left_button.opacity()
    }

    fn right_opacity(&self) -> f64 {
        self.imp().right_button.opacity()
    }

    fn is_at_lower(&self) -> bool {
        let adj = self.imp().diffview.scroll().hadjustment();
        adj.value() <= adj.lower() + f64::EPSILON
    }

    fn is_at_upper(&self) -> bool {
        let adj = self.imp().diffview.scroll().hadjustment();
        let max_value = adj.upper() - adj.page_size();
        adj.value() >= max_value - f64::EPSILON
    }

    fn show_left_animation(&self) -> &adw::TimedAnimation {
        self.imp().show_left_animation.get_or_init(|| {
            let target = adw::CallbackAnimationTarget::new(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |opacity| obj.set_left_opacity(opacity)
            ));
            adw::TimedAnimation::builder()
                .duration(SHOW_BUTTON_ANIMATION_DURATION)
                .widget(&self.imp().diffview.get())
                .target(&target)
                .value_to(0.7)
                .build()
        })
    }

    fn hide_left_animation(&self) -> &adw::TimedAnimation {
        self.imp().hide_left_animation.get_or_init(|| {
            let target = adw::CallbackAnimationTarget::new(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |opacity| obj.set_left_opacity(opacity)
            ));
            adw::TimedAnimation::builder()
                .duration(SHOW_BUTTON_ANIMATION_DURATION)
                .widget(&self.imp().diffview.get())
                .target(&target)
                .value_to(0.)
                .build()
        })
    }

    fn show_right_animation(&self) -> &adw::TimedAnimation {
        self.imp().show_right_animation.get_or_init(|| {
            let target = adw::CallbackAnimationTarget::new(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |opacity| obj.set_right_opacity(opacity)
            ));
            adw::TimedAnimation::builder()
                .duration(SHOW_BUTTON_ANIMATION_DURATION)
                .widget(&self.imp().diffview.get())
                .target(&target)
                .value_to(0.7)
                .build()
        })
    }

    fn hide_right_animation(&self) -> &adw::TimedAnimation {
        self.imp().hide_right_animation.get_or_init(|| {
            let target = adw::CallbackAnimationTarget::new(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |opacity| obj.set_right_opacity(opacity)
            ));
            adw::TimedAnimation::builder()
                .duration(SHOW_BUTTON_ANIMATION_DURATION)
                .widget(&self.imp().diffview.get())
                .target(&target)
                .value_to(0.)
                .build()
        })
    }

    fn update_left_button(&self, animate: bool) {
        let should_show = self.imp().is_hovering.get() && !self.is_at_lower();
        let current = self.left_opacity();
        if should_show && current < 0.7 {
            self.hide_left_animation().pause();
            self.show_left_animation().set_value_from(current);
            self.show_left_animation().play();
        } else if !should_show && current > 0. {
            if animate {
                self.show_left_animation().pause();
                self.hide_left_animation().set_value_from(current);
                self.hide_left_animation().play();
            } else {
                self.show_left_animation().pause();
                self.set_left_opacity(0.);
            }
        }
    }

    fn update_right_button(&self, animate: bool) {
        let should_show = self.imp().is_hovering.get() && !self.is_at_upper();
        let current = self.right_opacity();
        if should_show && current < 0.7 {
            self.hide_right_animation().pause();
            self.show_right_animation().set_value_from(current);
            self.show_right_animation().play();
        } else if !should_show && current > 0. {
            if animate {
                self.show_right_animation().pause();
                self.hide_right_animation().set_value_from(current);
                self.hide_right_animation().play();
            } else {
                self.show_right_animation().pause();
                self.set_right_opacity(0.);
            }
        }
    }

    #[template_callback]
    fn on_rightbutton_clicked(&self) {
        self.anime::<true>();
    }

    #[template_callback]
    fn on_enter_focus(&self) {
        self.imp().is_hovering.set(true);
        self.update_left_button(true);
        self.update_right_button(true);
    }

    #[template_callback]
    fn on_leave_focus(&self) {
        self.imp().is_hovering.set(false);
        let left = self.left_opacity();
        if left > 0. {
            self.show_left_animation().pause();
            self.hide_left_animation().set_value_from(left);
            self.hide_left_animation().play();
        }
        let right = self.right_opacity();
        if right > 0. {
            self.show_right_animation().pause();
            self.hide_right_animation().set_value_from(right);
            self.hide_right_animation().play();
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
        let scrolled = self.imp().diffview.scroll();
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
