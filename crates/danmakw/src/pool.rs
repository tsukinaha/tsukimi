use crate::{
    Color,
    Danmaku,
    DanmakuMode,
    DanmakwRenderer,
    Intensity,
};
use gtk::{
    gdk::FrameClock,
    glib,
    prelude::*,
    subclass::prelude::*,
};
use std::cell::{
    Cell,
    RefCell,
};

mod imp {
    use crate::{
        DanmakuClock,
        DanmakwRenderer,
        DanmakwSnapshotExt,
    };

    use super::*;
    use gtk::TickCallbackId;

    #[derive(glib::Properties)]
    #[properties(wrapper_type = super::Danmakw)]
    pub struct Danmakw {
        #[property(get, set = Self::set_speed_factor, default_value = 1.0)]
        pub speed_factor: Cell<f64>,

        #[property(get, set = Self::set_font_size, default_value = 32.0)]
        pub font_size: Cell<f64>,

        #[property(get, set = Self::set_font_weight, default_value = 5u32)]
        pub font_weight: Cell<u32>,

        #[property(get, set = Self::set_intensity, builder(Intensity::default()))]
        pub intensity: Cell<Intensity>,

        #[property(get, set = Self::set_spacing_factor, default_value = 1.2)]
        pub spacing_factor: Cell<f64>,

        #[property(get, set = Self::set_outline_px, default_value = 1.0)]
        pub outline_px: Cell<f64>,

        #[property(get, set = Self::set_shadow_offset, default_value = 1.0)]
        pub shadow_offset: Cell<f64>,

        pub renderer: RefCell<DanmakwRenderer>,
        pub clock: RefCell<Option<DanmakuClock>>,
        pub tick_callback_id: RefCell<Option<TickCallbackId>>,
        pub pending_seek: Cell<Option<f64>>,
        pub allocation: Cell<(i32, i32)>,
    }

    impl Default for Danmakw {
        fn default() -> Self {
            Self {
                speed_factor: Cell::new(1.0),
                font_size: Cell::new(32.0),
                font_weight: Cell::new(5),
                intensity: Cell::new(Intensity::default()),
                spacing_factor: Cell::new(1.2),
                outline_px: Cell::new(1.0),
                shadow_offset: Cell::new(1.0),
                renderer: Default::default(),
                clock: Default::default(),
                tick_callback_id: Default::default(),
                pending_seek: Default::default(),
                allocation: Default::default(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Danmakw {
        const NAME: &'static str = "Danmakw";
        type Type = super::Danmakw;
        type ParentType = gtk::Widget;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Danmakw {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for Danmakw {
        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            self.parent_size_allocate(width, height, baseline);

            let previous = self.allocation.replace((width, height));
            if width <= 0 || height <= 0 {
                return;
            }

            let pending_seek = self.pending_seek.take();
            if previous == (width, height) && pending_seek.is_none() {
                return;
            }

            let time_milis = pending_seek.unwrap_or_else(|| {
                self.clock
                    .borrow()
                    .as_ref()
                    .map(DanmakuClock::time_milis)
                    .unwrap_or_else(|| self.renderer.borrow().last_time)
            });
            self.obj()
                .rebuild_visible_state_at(width, height, time_milis);
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let obj = self.obj();
            let width = obj.width() as f32;
            let height = obj.height() as f32;
            let mut renderer = self.renderer.borrow_mut();
            obj.sync_gpu(&mut renderer);
            snapshot.render_danmakw(&mut renderer, width, height);
        }
    }

    impl Danmakw {
        pub fn start_clock(&self) {
            let mut clock = self.clock.borrow_mut();
            if let Some(c) = clock.as_mut() {
                c.resume();
            } else {
                *clock = Some(DanmakuClock::new(self.speed_factor.get()));
            }
        }

        pub fn pause_clock(&self) {
            if let Some(clock) = self.clock.borrow_mut().as_mut() {
                clock.pause();
            }
        }

        fn set_speed_factor(&self, v: f64) {
            self.speed_factor.set(v);
            self.renderer.borrow_mut().speed_factor = v;
            if let Some(clock) = self.clock.borrow_mut().as_mut() {
                clock.set_speed_factor(v);
            }
        }

        fn set_font_size(&self, v: f64) {
            self.font_size.set(v);
            self.renderer.borrow_mut().font_size = v;
        }

        fn set_font_weight(&self, v: u32) {
            self.font_weight.set(v);
            self.renderer.borrow_mut().set_font_weight_index(v);
        }

        fn set_intensity(&self, v: Intensity) {
            self.intensity.set(v);
            self.renderer.borrow_mut().set_intensity(v);
        }

        fn set_spacing_factor(&self, v: f64) {
            self.spacing_factor.set(v);
            self.renderer.borrow_mut().spacing_factor = v as f32;
        }

        fn set_outline_px(&self, v: f64) {
            self.outline_px.set(v);
            self.renderer.borrow_mut().outline_px = v;
        }

        fn set_shadow_offset(&self, v: f64) {
            self.shadow_offset.set(v);
            self.renderer.borrow_mut().shadow_offset = v;
        }
    }
}

glib::wrapper! {
    pub struct Danmakw(ObjectSubclass<imp::Danmakw>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for Danmakw {
    fn default() -> Self {
        Self::new()
    }
}

impl Danmakw {
    pub fn new() -> Self {
        glib::Object::new()
    }

    fn sync_gpu(&self, renderer: &mut DanmakwRenderer) {
        renderer.scale_factor = self.scale_factor() as f64;
        renderer.gsk_renderer = self.native().and_then(|n| n.renderer());
    }

    fn rebuild_visible_state_at(&self, width: i32, height: i32, time_milis: f64) {
        let mut renderer = self.imp().renderer.borrow_mut();
        self.sync_gpu(&mut renderer);
        if renderer.screen_height != height as f32 {
            renderer.screen_height = height as f32;
            renderer.recompute_max_rows();
        }
        renderer.rebuild_visible_state_at(&self.pango_context(), width as f32, time_milis);
        self.queue_draw();
    }

    pub fn clear_danmaku(&self) {
        let mut renderer = self.imp().renderer.borrow_mut();
        renderer.clear_danmaku();
    }

    pub fn add_danmaku(&self, text: &str) {
        self.add_danmaku_full(text, Color::default(), DanmakuMode::Scroll);
    }

    pub fn add_danmaku_full(&self, text: &str, color: Color, mode: DanmakuMode) {
        let width = self.width() as f32;
        let danmaku = Danmaku {
            content: text.to_string(),
            start: 0.0,
            color,
            mode,
        };
        let mut renderer = self.imp().renderer.borrow_mut();
        self.sync_gpu(&mut renderer);
        renderer.add_danmaku(&self.pango_context(), width, danmaku);
    }

    pub fn load_danmaku(&self, danmaku: Vec<Danmaku>) {
        let mut renderer = self.imp().renderer.borrow_mut();
        renderer.danmaku_queue.init(danmaku, 0.0);
        renderer.clear_danmaku();
    }

    pub fn start_rendering(&self) {
        self.start_clock();
        let id = self.add_tick_callback(Self::cb);
        self.imp().tick_callback_id.replace(Some(id));
    }

    pub fn stop_rendering(&self) {
        if let Some(id) = self.imp().tick_callback_id.borrow_mut().take() {
            id.remove();
        }
    }

    pub fn start_clock(&self) {
        self.imp().start_clock();
    }

    pub fn pause_clock(&self) {
        self.imp().pause_clock();
    }

    pub fn set_paused(&self, paused: bool) {
        if paused {
            self.pause_clock();
            self.stop_rendering();
        } else {
            self.start_rendering();
        }
    }

    pub fn update(&self, time_milis: f64) {
        let imp = self.imp();
        let width = self.width() as f32;
        if width <= 0.0 || self.height() <= 0 {
            imp.pending_seek.set(Some(time_milis));
            return;
        }
        let mut renderer = imp.renderer.borrow_mut();
        self.sync_gpu(&mut renderer);
        renderer.update(&self.pango_context(), width, time_milis);
    }

    pub fn preroll_seek(&self, time_milis: f64) {
        let imp = self.imp();
        if let Some(c) = imp.clock.borrow_mut().as_mut() {
            c.seek(time_milis)
        }

        let width = self.width();
        let height = self.height();
        if width <= 0 || height <= 0 {
            imp.pending_seek.set(Some(time_milis));
            return;
        }

        imp.pending_seek.set(None);
        self.rebuild_visible_state_at(width, height, time_milis);
    }

    fn cb(&self, frame_clock: &FrameClock) -> glib::ControlFlow {
        let imp = self.imp();
        let width = self.width() as f32;
        if width <= 0.0 || self.height() <= 0 {
            return glib::ControlFlow::Continue;
        }

        let time = {
            let clock = imp.clock.borrow();
            let Some(clock) = clock.as_ref() else {
                return glib::ControlFlow::Continue;
            };
            clock.time_milis_at(frame_clock.frame_time())
        };

        let mut renderer = imp.renderer.borrow_mut();
        self.sync_gpu(&mut renderer);
        renderer.update(&self.pango_context(), width, time);

        self.queue_draw();
        glib::ControlFlow::Continue
    }
}
