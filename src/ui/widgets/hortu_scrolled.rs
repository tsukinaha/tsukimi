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
            fix::{
                ScrolledWindowFixExt,
                scroll_widget_to_row_center,
            },
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
        pub keyboard_focused: Cell<bool>,
        pub header_focused: Cell<bool>,
        pub selected_index: Cell<Option<usize>>,
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
                glib::clone!(
                    #[weak(rename_to = obj)]
                    self.obj(),
                    move |widget, tu_obj: &TuObject| {
                        let tu_item = widget
                            .downcast_ref::<TuListItem>()
                            .expect("LazyDiffView row must be a TuListItem");
                        let key = tu_obj.item().key();
                        let selected = obj
                            .imp()
                            .selected_index
                            .get()
                            .and_then(|index| obj.imp().diffview.key_at(index))
                            .as_deref()
                            == Some(key.as_str());
                        if tu_item.item().key() != key {
                            tu_item.set_item(tu_obj.item());
                        }
                        tu_item.set_poster_focused(selected);
                    }
                ),
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
        if self.imp().keyboard_focused.get() {
            return;
        }
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

    pub fn item_count(&self) -> usize {
        self.imp().diffview.len()
    }

    pub fn ensure_selection(&self) {
        if self.item_count() == 0 {
            return;
        }
        if self.imp().selected_index.get().is_none() {
            self.set_selection_index(0);
        }
    }

    pub fn clear_selection(&self) {
        let prev_key = self
            .imp()
            .selected_index
            .get()
            .and_then(|index| self.imp().diffview.key_at(index));
        self.imp().selected_index.set(None);
        self.imp().header_focused.set(false);
        self.clear_keyboard_focus();
        crate::tv::set_tv_focused(&self.imp().morebutton.get(), false);
        if let Some(key) = prev_key {
            self.set_focus_for_key(&key, false);
        }
    }

    pub fn clear_keyboard_focus(&self) {
        self.imp().keyboard_focused.set(false);
        self.on_leave_scroll_controls();
    }

    pub fn move_selection(&self, delta: i32) {
        let count = self.item_count();
        if count == 0 {
            return;
        }
        if self.imp().header_focused.get() {
            if delta > 0 {
                self.leave_header_focus();
            }
            return;
        }
        let current = self.imp().selected_index.get().unwrap_or(0);
        if delta < 0 && current == 0 && self.imp().morebutton.is_visible() {
            self.focus_header();
            return;
        }
        let next = (current as i32 + delta).clamp(0, count as i32 - 1) as usize;
        self.set_selection_index(next);
    }

    pub fn is_header_focused(&self) -> bool {
        self.imp().header_focused.get()
    }

    pub fn focus_header(&self) {
        if !self.imp().morebutton.is_visible() {
            return;
        }
        let prev_key = self
            .imp()
            .selected_index
            .get()
            .and_then(|index| self.imp().diffview.key_at(index));
        self.imp().selected_index.set(None);
        self.imp().header_focused.set(true);
        if let Some(key) = prev_key {
            self.set_focus_for_key(&key, false);
        }
        self.clear_keyboard_focus();
        crate::tv::set_tv_focused(&self.imp().morebutton.get(), true);
        self.imp().morebutton.grab_focus();
    }

    fn leave_header_focus(&self) {
        self.imp().header_focused.set(false);
        crate::tv::set_tv_focused(&self.imp().morebutton.get(), false);
        self.ensure_selection();
    }

    pub fn selection_at_start(&self) -> bool {
        self.imp().selected_index.get().unwrap_or(0) == 0
    }

    fn set_selection_index(&self, index: usize) {
        let count = self.item_count();
        if count == 0 {
            return;
        }
        let index = index.min(count - 1);
        let prev_key = self
            .imp()
            .selected_index
            .get()
            .and_then(|idx| self.imp().diffview.key_at(idx));
        let new_key = self.imp().diffview.key_at(index);
        self.imp().selected_index.set(Some(index));
        self.imp().diffview.scroll_to_index(index);
        self.update_selection_focus(prev_key.as_deref(), new_key.as_deref());
        self.show_scroll_controls_for_focus();
    }

    fn update_selection_focus(&self, prev_key: Option<&str>, new_key: Option<&str>) {
        if let Some(key) = prev_key.filter(|k| Some(*k) != new_key) {
            self.set_focus_for_key(key, false);
        }
        if let Some(key) = new_key {
            self.set_focus_for_key(key, true);
        }
    }

    fn set_focus_for_key(&self, key: &str, focused: bool) {
        if let Some(widget) = self.imp().diffview.row_widget_for_key(key)
            && let Some(item) = widget.downcast_ref::<TuListItem>()
        {
            item.set_poster_focused(focused);
        }
    }

    pub fn activate_selected(&self, window: &crate::Window) {
        let imp = self.imp();
        if imp.header_focused.get() {
            imp.morebutton.emit_clicked();
            return;
        }
        let Some(index) = imp.selected_index.get() else {
            return;
        };
        let items = imp.diffview.len();
        if index >= items {
            return;
        }
        if let Some(key) = imp
            .selected_index
            .get()
            .and_then(|idx| imp.diffview.key_at(idx))
            && let Some(obj) = imp.item_cache.borrow().get(&key)
        {
            obj.item().activate(window);
        }
    }

    pub fn show_scroll_controls_for_focus(&self) {
        self.imp().header_focused.set(false);
        crate::tv::set_tv_focused(&self.imp().morebutton.get(), false);
        self.imp().keyboard_focused.set(true);
        self.on_enter_scroll_controls();
    }

    pub fn scroll_into_parent_viewport(&self) {
        scroll_widget_to_row_center(self);
    }

    pub fn scroll_page_left(&self) {
        self.scroll_controls_anime::<false>();
    }

    pub fn scroll_page_right(&self) {
        self.scroll_controls_anime::<true>();
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
