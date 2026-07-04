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
            hor_controls::HorControlsExt,
            lazy_diff_view::LazyDiffView,
            tu_list_item::{
                TuListItem,
                imp::PosterType,
            },
        },
    },
};

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

            self.obj().connect_scroll_controls();
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

    #[template_callback]
    fn on_rightbutton_clicked(&self) {
        self.scroll_controls_anime::<true>();
    }

    #[template_callback]
    fn on_enter_focus(&self) {
        self.on_enter_scroll_controls();
    }

    #[template_callback]
    fn on_leave_focus(&self) {
        self.on_leave_scroll_controls();
    }

    #[template_callback]
    fn on_leftbutton_clicked(&self) {
        self.scroll_controls_anime::<false>();
    }

    pub fn connect_morebutton<F>(&self, cb: F)
    where
        F: Fn(&gtk::Button) + 'static,
    {
        self.imp().morebutton.connect_clicked(cb);
    }
}

impl HorControlsExt for HortuScrolled {
    fn scroll_widget(&self) -> gtk::ScrolledWindow {
        self.imp().diffview.scroll()
    }

    fn left_button(&self) -> gtk::Button {
        self.imp().left_button.get()
    }

    fn right_button(&self) -> gtk::Button {
        self.imp().right_button.get()
    }

    fn show_left_animation_cell(&self) -> &std::cell::OnceCell<adw::TimedAnimation> {
        &self.imp().show_left_animation
    }

    fn hide_left_animation_cell(&self) -> &std::cell::OnceCell<adw::TimedAnimation> {
        &self.imp().hide_left_animation
    }

    fn show_right_animation_cell(&self) -> &std::cell::OnceCell<adw::TimedAnimation> {
        &self.imp().show_right_animation
    }

    fn hide_right_animation_cell(&self) -> &std::cell::OnceCell<adw::TimedAnimation> {
        &self.imp().hide_right_animation
    }

    fn is_hovering(&self) -> &std::cell::Cell<bool> {
        &self.imp().is_hovering
    }
}
