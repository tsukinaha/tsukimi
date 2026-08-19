use std::cell::{
    Cell,
    RefCell,
};

use gtk::{
    gdk,
    glib,
    graphene,
    gsk,
    gsk::prelude::IsRenderNode,
    prelude::*,
    subclass::prelude::*,
};

use super::cover_palette::CoverPalette;

const SIZE_BUCKET: i32 = 64;
const CROSSFADE_DURATION_US: i64 = 520_000;

const BALANCED_FRAME_INTERVAL_US: i64 = 33_333;
const MOTION_PERIOD_US: i64 = 28_000_000;
const PHASE_FRAME_COUNT: usize = 10;
const PHASE_DOWNSAMPLE: f64 = 4.0;
const PHASE_TEXTURE_BUDGET_BYTES: usize = 8 * 1024 * 1024;
const MAX_CACHED_SCENES: usize = 2;
const TEXTURE_BYTES_PER_PIXEL: usize = 4;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "LycoricBackgroundQuality")]
pub enum BackgroundQuality {
    Eco,
    #[default]
    Balanced,
    High,
}

#[derive(Clone)]
struct CachedCover {
    generation: u64,
    size: (i32, i32),
    scale_factor: i32,
    node: gsk::RenderNode,
}

#[derive(Clone)]
struct LiveScene {
    size: (i32, i32),
    base: gsk::RenderNode,
    glow_a: gsk::RenderNode,
    glow_b: gsk::RenderNode,
    glow_c: Option<gsk::RenderNode>,
    scrim: gsk::RenderNode,
}

#[derive(Clone)]
struct CachedPhaseFrames {
    nodes: Vec<gsk::RenderNode>,
    textured: bool,
}

impl CachedPhaseFrames {
    fn node(&self, phase: f32) -> gsk::RenderNode {
        let sample = phase_sample(phase, self.nodes.len());
        let from = &self.nodes[sample.from];
        if sample.from == sample.to || sample.mix <= f32::EPSILON {
            return from.clone();
        }
        gsk::CrossFadeNode::new(from, &self.nodes[sample.to], sample.mix).upcast()
    }
}

#[derive(Clone)]
struct CachedScene {
    size: (i32, i32),
    scale_factor: i32,
    frames: CachedPhaseFrames,
    live: Option<LiveScene>,
}

impl CachedScene {
    fn cached_node(&self, phase: f32) -> gsk::RenderNode {
        self.frames.node(phase)
    }

    fn steady_node(&self, phase: f32, quality: BackgroundQuality) -> gsk::RenderNode {
        self.live
            .as_ref()
            .filter(|_| quality == BackgroundQuality::High && self.frames.textured)
            .map_or_else(
                || self.cached_node(phase),
                |scene| compose_live_scene(scene, phase, quality),
            )
    }
}

struct PaintableHandlers {
    contents: glib::SignalHandlerId,
    size: glib::SignalHandlerId,
}

mod imp {
    use super::*;
    use gtk::TickCallbackId;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::AnimatedBackdrop)]
    pub struct AnimatedBackdrop {
        #[property(get, set = Self::set_cover, explicit_notify, nullable)]
        pub(super) cover: RefCell<Option<gdk::Paintable>>,
        pub(super) cover_handlers: RefCell<Option<PaintableHandlers>>,
        pub(super) cover_generation: Cell<u64>,
        pub(super) cover_cache: RefCell<Option<CachedCover>>,
        pub(super) palette: RefCell<CoverPalette>,
        #[property(get, set = Self::set_quality, explicit_notify, builder(BackgroundQuality::default()))]
        pub(super) quality: Cell<BackgroundQuality>,
        #[property(get, set = Self::set_reduced_motion, explicit_notify)]
        pub(super) reduced_motion: Cell<bool>,
        #[property(get, set = Self::set_motion_active, explicit_notify)]
        pub(super) motion_active: Cell<bool>,
        pub(super) current_scene: RefCell<Option<CachedScene>>,
        pub(super) previous_scene: RefCell<Option<CachedScene>>,
        pub(super) crossfade_start_us: Cell<i64>,
        pub(super) crossfade_progress: Cell<f32>,
        pub(super) animation_origin_us: Cell<i64>,
        pub(super) phase: Cell<f32>,
        pub(super) last_frame_draw_us: Cell<i64>,
        pub(super) tick_callback: RefCell<Option<TickCallbackId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AnimatedBackdrop {
        const NAME: &'static str = "LycoricAnimatedBackdrop";
        type Type = super::AnimatedBackdrop;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("lycoric-animated-backdrop");
            klass.set_accessible_role(gtk::AccessibleRole::Presentation);
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for AnimatedBackdrop {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_hexpand(true);
            obj.set_vexpand(true);
            obj.set_overflow(gtk::Overflow::Hidden);
            gtk::prelude::WidgetExt::connect_scale_factor_notify(&*obj, |obj| {
                obj.handle_scale_change()
            });
        }

        fn dispose(&self) {
            self.stop_animation_sources();
            self.obj().disconnect_cover_handlers();
            self.cover.borrow_mut().take();
            self.cover_cache.borrow_mut().take();
            self.current_scene.borrow_mut().take();
            self.previous_scene.borrow_mut().take();
        }
    }

    impl WidgetImpl for AnimatedBackdrop {
        fn realize(&self) {
            self.parent_realize();
            self.cover_cache.borrow_mut().take();
            self.obj().rebuild_scene(false);
        }

        fn unrealize(&self) {
            self.stop_animation_sources();
            self.cover_cache.borrow_mut().take();
            self.current_scene.borrow_mut().take();
            self.previous_scene.borrow_mut().take();
            self.parent_unrealize();
        }

        fn map(&self) {
            self.parent_map();
            self.obj().ensure_animation_source();
        }

        fn unmap(&self) {
            self.stop_animation_sources();
            self.obj().finish_crossfade();
            self.parent_unmap();
        }

        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            self.parent_size_allocate(width, height, baseline);
            let size = bucketed_size(width, height);
            let cache_key = self
                .current_scene
                .borrow()
                .as_ref()
                .map(|scene| (scene.size, scene.scale_factor));
            let scale_factor = gtk::prelude::WidgetExt::scale_factor(&*self.obj()).max(1);
            if size.0 > 0 && size.1 > 0 && cache_key != Some((size, scale_factor)) {
                self.obj().rebuild_scene(false);
            }
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let obj = self.obj();
            let current = self.current_scene.borrow();
            let Some(current) = current.as_ref() else {
                return;
            };

            let bounds = graphene::Rect::new(0.0, 0.0, obj.width() as f32, obj.height() as f32);
            snapshot.push_clip(&bounds);

            let phase = self.phase.get();
            let previous = self.previous_scene.borrow();
            if let Some(previous) = previous.as_ref() {
                let old_node = previous.cached_node(phase);
                let new_node = current.cached_node(phase);
                let crossfade =
                    gsk::CrossFadeNode::new(&old_node, &new_node, self.crossfade_progress.get());
                snapshot.append_node(&crossfade);
            } else {
                snapshot.append_node(current.steady_node(phase, self.quality.get()));
            }

            snapshot.pop();
        }
    }

    impl AnimatedBackdrop {
        fn set_cover(&self, cover: Option<gdk::Paintable>) {
            let obj = self.obj();
            if !obj.replace_cover(cover.as_ref()) {
                return;
            }
            obj.invalidate_cover_cache();
            obj.rebuild_scene(true);
            obj.notify_cover();
        }

        fn set_quality(&self, quality: BackgroundQuality) {
            if self.quality.replace(quality) == quality {
                return;
            }
            self.cover_cache.borrow_mut().take();
            let obj = self.obj();
            obj.rebuild_scene(false);
            obj.ensure_animation_source();
            obj.notify_quality();
        }

        fn set_reduced_motion(&self, reduced: bool) {
            if self.reduced_motion.replace(reduced) == reduced {
                return;
            }
            let obj = self.obj();
            if reduced {
                obj.finish_crossfade();
                self.phase.set(0.0);
            }
            obj.ensure_animation_source();
            obj.queue_draw();
            obj.notify_reduced_motion();
        }

        fn set_motion_active(&self, active: bool) {
            if self.motion_active.replace(active) == active {
                return;
            }
            let obj = self.obj();
            obj.ensure_animation_source();
            obj.notify_motion_active();
        }

        pub(super) fn stop_animation_sources(&self) {
            if let Some(callback) = self.tick_callback.borrow_mut().take() {
                callback.remove();
            }
        }
    }
}

glib::wrapper! {
    pub struct AnimatedBackdrop(ObjectSubclass<imp::AnimatedBackdrop>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl AnimatedBackdrop {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_cover_and_palette(&self, cover: Option<&gdk::Paintable>, palette: CoverPalette) {
        let cover_changed = self.replace_cover(cover);
        let palette_changed = *self.imp().palette.borrow() != palette;
        if !cover_changed && !palette_changed {
            return;
        }

        if cover_changed {
            self.invalidate_cover_cache();
            self.notify_cover();
        }
        if palette_changed {
            self.imp().palette.replace(palette);
        }
        self.rebuild_scene(true);
    }

    pub fn set_palette(&self, palette: CoverPalette) {
        if *self.imp().palette.borrow() == palette {
            return;
        }
        self.imp().palette.replace(palette);
        self.rebuild_scene(true);
    }

    pub fn palette(&self) -> CoverPalette {
        self.imp().palette.borrow().clone()
    }

    fn replace_cover(&self, cover: Option<&gdk::Paintable>) -> bool {
        if same_paintable(self.imp().cover.borrow().as_ref(), cover) {
            return false;
        }

        self.disconnect_cover_handlers();
        self.imp().cover.replace(cover.cloned());
        self.connect_cover_handlers();
        true
    }

    fn connect_cover_handlers(&self) {
        let Some(cover) = self.imp().cover.borrow().clone() else {
            return;
        };

        let weak = self.downgrade();
        let contents = cover.connect_invalidate_contents(move |_| {
            if let Some(obj) = weak.upgrade() {
                obj.handle_cover_invalidation();
            }
        });
        let weak = self.downgrade();
        let size = cover.connect_invalidate_size(move |_| {
            if let Some(obj) = weak.upgrade() {
                obj.handle_cover_invalidation();
            }
        });
        self.imp()
            .cover_handlers
            .replace(Some(PaintableHandlers { contents, size }));
    }

    fn disconnect_cover_handlers(&self) {
        let handlers = self.imp().cover_handlers.borrow_mut().take();
        let cover = self.imp().cover.borrow().clone();
        if let (Some(cover), Some(handlers)) = (cover, handlers) {
            cover.disconnect(handlers.contents);
            cover.disconnect(handlers.size);
        }
    }

    fn handle_cover_invalidation(&self) {
        self.invalidate_cover_cache();
        self.rebuild_scene(false);
    }

    fn handle_scale_change(&self) {
        self.imp().cover_cache.borrow_mut().take();
        self.rebuild_scene(false);
    }

    fn invalidate_cover_cache(&self) {
        let generation = self.imp().cover_generation.get().wrapping_add(1);
        self.imp().cover_generation.set(generation);
        self.imp().cover_cache.borrow_mut().take();
    }

    fn rebuild_scene(&self, transition: bool) {
        let size = bucketed_size(self.width(), self.height());
        if size.0 <= 0 || size.1 <= 0 {
            return;
        }

        self.imp().previous_scene.borrow_mut().take();
        if transition && let Some(current) = self.imp().current_scene.borrow_mut().as_mut() {
            current.live.take();
        }
        let scene = self.build_scene(size);
        let can_crossfade = transition
            && self.imp().current_scene.borrow().is_some()
            && self.is_mapped()
            && !self.reduced_motion();

        if can_crossfade {
            let old_scene = self.imp().current_scene.borrow_mut().take();
            self.imp().previous_scene.replace(old_scene);
            self.imp().crossfade_start_us.set(glib::monotonic_time());
            self.imp().crossfade_progress.set(0.0);
        } else {
            self.imp().previous_scene.borrow_mut().take();
            self.imp().crossfade_progress.set(1.0);
        }

        self.imp().current_scene.replace(Some(scene));
        self.queue_draw();
        self.ensure_animation_source();
    }

    fn build_scene(&self, size: (i32, i32)) -> CachedScene {
        let scale_factor = gtk::prelude::WidgetExt::scale_factor(self).max(1);
        let bounds = scene_bounds(size);
        let palette = self.palette();
        let base = build_base_node(
            self.cached_cover_node(size, scale_factor),
            &palette,
            &bounds,
        );
        let quality = self.quality();
        let live = LiveScene {
            size,
            base,
            glow_a: radial_node(&bounds, &palette.accent, (0.16, 0.22), 0.90),
            glow_b: radial_node(&bounds, &palette.secondary, (0.84, 0.72), 0.82),
            glow_c: (quality == BackgroundQuality::High)
                .then(|| conic_node(&bounds, &palette.dominant, &palette.accent)),
            scrim: scrim_node(&bounds, &palette),
        };
        let renderer = self.native().and_then(|native| native.renderer());
        let frames = prepare_phase_frames(&live, quality, scale_factor, renderer.as_ref());
        CachedScene {
            size,
            scale_factor,
            frames,
            live: (quality == BackgroundQuality::High).then_some(live),
        }
    }

    fn cached_cover_node(&self, size: (i32, i32), scale_factor: i32) -> Option<gsk::RenderNode> {
        let generation = self.imp().cover_generation.get();
        if let Some(cache) = self.imp().cover_cache.borrow().as_ref()
            && cache.generation == generation
            && cache.size == size
            && cache.scale_factor == scale_factor
        {
            return Some(cache.node.clone());
        }

        let cover = self.imp().cover.borrow().clone()?;
        let source = cover_crop_node(&cover.current_image(), size)?;
        let node = self.bake_blurred_cover(&source, size, scale_factor);
        self.imp().cover_cache.replace(Some(CachedCover {
            generation,
            size,
            scale_factor,
            node: node.clone(),
        }));
        Some(node)
    }

    fn bake_blurred_cover(
        &self, source: &gsk::RenderNode, size: (i32, i32), scale_factor: i32,
    ) -> gsk::RenderNode {
        match self.quality() {
            BackgroundQuality::Eco => return source.clone(),
            BackgroundQuality::Balanced => return gsk::BlurNode::new(source, 28.0).upcast(),
            BackgroundQuality::High => {}
        }

        let radius = 42.0;
        let scale = scale_factor.max(1) as f32;
        let transform = gsk::Transform::new().scale(scale, scale);
        let scaled = gsk::TransformNode::new(source, Some(&transform));
        let blurred = gsk::BlurNode::new(&scaled, radius * scale);
        let Some(renderer) = self.native().and_then(|native| native.renderer()) else {
            return source.clone();
        };

        let logical_bounds = scene_bounds(size);
        let physical_bounds = graphene::Rect::new(
            0.0,
            0.0,
            logical_bounds.width() * scale,
            logical_bounds.height() * scale,
        );
        let texture = renderer.render_texture(&blurred, Some(&physical_bounds));
        let snapshot = gtk::Snapshot::new();
        snapshot.append_scaled_texture(&texture, gsk::ScalingFilter::Linear, &logical_bounds);
        snapshot.to_node().unwrap_or_else(|| source.clone())
    }

    fn ensure_animation_source(&self) {
        if !self.is_mapped() || self.reduced_motion() {
            self.stop_animation_sources();
            return;
        }

        if self.needs_frame_clock() {
            self.ensure_frame_tick();
        } else {
            self.stop_frame_tick();
        }
    }

    fn needs_frame_clock(&self) -> bool {
        self.imp().previous_scene.borrow().is_some() || self.continuous_motion()
    }

    fn ensure_frame_tick(&self) {
        if self.imp().tick_callback.borrow().is_some() {
            return;
        }
        let callback = self.add_tick_callback(|obj, clock| obj.frame_tick(clock));
        self.imp().tick_callback.replace(Some(callback));
    }

    fn stop_animation_sources(&self) {
        self.stop_frame_tick();
    }

    fn stop_frame_tick(&self) {
        if let Some(callback) = self.imp().tick_callback.borrow_mut().take() {
            callback.remove();
        }
    }

    fn frame_tick(&self, clock: &gdk::FrameClock) -> glib::ControlFlow {
        let now = clock.frame_time();
        let crossfading = self.advance_crossfade(now);
        let continuous = self.continuous_motion();
        let balanced_due =
            now.saturating_sub(self.imp().last_frame_draw_us.get()) >= BALANCED_FRAME_INTERVAL_US;
        let draw_due = crossfading
            || (continuous && (self.quality() == BackgroundQuality::High || balanced_due));
        if draw_due {
            if continuous {
                self.advance_phase(now);
            }
            self.imp().last_frame_draw_us.set(now);
            self.queue_draw();
        }

        if crossfading || continuous {
            glib::ControlFlow::Continue
        } else {
            self.imp().tick_callback.borrow_mut().take();
            glib::ControlFlow::Break
        }
    }

    fn continuous_motion(&self) -> bool {
        self.motion_active() && self.quality() != BackgroundQuality::Eco && !self.reduced_motion()
    }

    fn advance_phase(&self, now: i64) {
        let origin = self.imp().animation_origin_us.get();
        let origin = if origin == 0 {
            self.imp().animation_origin_us.set(now);
            now
        } else {
            origin
        };
        let elapsed = now.saturating_sub(origin).rem_euclid(MOTION_PERIOD_US);
        self.imp()
            .phase
            .set(elapsed as f32 / MOTION_PERIOD_US as f32);
    }

    fn advance_crossfade(&self, now: i64) -> bool {
        if self.imp().previous_scene.borrow().is_none() {
            return false;
        }
        let elapsed = now.saturating_sub(self.imp().crossfade_start_us.get());
        let linear = (elapsed as f32 / CROSSFADE_DURATION_US as f32).clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - linear).powi(3);
        self.imp().crossfade_progress.set(eased);
        if linear >= 1.0 {
            self.finish_crossfade();
            false
        } else {
            true
        }
    }

    fn finish_crossfade(&self) {
        self.imp().previous_scene.borrow_mut().take();
        self.imp().crossfade_progress.set(1.0);
    }
}

impl Default for AnimatedBackdrop {
    fn default() -> Self {
        Self::new()
    }
}

pub(super) fn same_paintable(
    left: Option<&gdk::Paintable>, right: Option<&gdk::Paintable>,
) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left.as_ptr() == right.as_ptr(),
        (None, None) => true,
        _ => false,
    }
}

fn bucketed_size(width: i32, height: i32) -> (i32, i32) {
    (bucket(width), bucket(height))
}

fn bucket(value: i32) -> i32 {
    if value <= 0 {
        return 0;
    }
    ((value + SIZE_BUCKET - 1) / SIZE_BUCKET) * SIZE_BUCKET
}

fn scene_bounds(size: (i32, i32)) -> graphene::Rect {
    graphene::Rect::new(0.0, 0.0, size.0 as f32, size.1 as f32)
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PhaseSample {
    from: usize,
    to: usize,
    mix: f32,
}

fn phase_sample(phase: f32, frame_count: usize) -> PhaseSample {
    if frame_count <= 1 {
        return PhaseSample {
            from: 0,
            to: 0,
            mix: 0.0,
        };
    }
    let phase = if phase.is_finite() {
        phase.rem_euclid(1.0)
    } else {
        0.0
    };
    let position = phase * frame_count as f32;
    let from = (position.floor() as usize).min(frame_count - 1);
    PhaseSample {
        from,
        to: (from + 1) % frame_count,
        mix: position - from as f32,
    }
}

fn prepare_phase_frames(
    scene: &LiveScene, quality: BackgroundQuality, scale_factor: i32,
    renderer: Option<&gsk::Renderer>,
) -> CachedPhaseFrames {
    let frame_count = if quality == BackgroundQuality::Eco {
        1
    } else {
        PHASE_FRAME_COUNT
    };
    let fallback = (0..frame_count)
        .map(|index| compose_live_scene(scene, index as f32 / frame_count as f32, quality))
        .collect::<Vec<_>>();
    let Some(renderer) = renderer else {
        return CachedPhaseFrames {
            nodes: fallback,
            textured: false,
        };
    };

    let budget = phase_scene_texture_budget();
    let raster_size = phase_texture_size(scene.size, scale_factor, frame_count, budget);
    let Some(nodes) = bake_phase_frames(renderer, &fallback, scene.size, raster_size, budget)
    else {
        return CachedPhaseFrames {
            nodes: fallback,
            textured: false,
        };
    };
    CachedPhaseFrames {
        nodes,
        textured: true,
    }
}

fn bake_phase_frames(
    renderer: &gsk::Renderer, frames: &[gsk::RenderNode], logical_size: (i32, i32),
    raster_size: (i32, i32), budget: usize,
) -> Option<Vec<gsk::RenderNode>> {
    let mut used_bytes = 0usize;
    let mut baked = Vec::with_capacity(frames.len());
    for frame in frames {
        let (node, bytes) = bake_phase_frame(renderer, frame, logical_size, raster_size)?;
        used_bytes = used_bytes.checked_add(bytes)?;
        if used_bytes > budget {
            return None;
        }
        baked.push(node);
    }
    Some(baked)
}

fn bake_phase_frame(
    renderer: &gsk::Renderer, frame: &gsk::RenderNode, logical_size: (i32, i32),
    raster_size: (i32, i32),
) -> Option<(gsk::RenderNode, usize)> {
    if logical_size.0 <= 0 || logical_size.1 <= 0 || raster_size.0 <= 0 || raster_size.1 <= 0 {
        return None;
    }
    let transform = gsk::Transform::new().scale(
        raster_size.0 as f32 / logical_size.0 as f32,
        raster_size.1 as f32 / logical_size.1 as f32,
    );
    let scaled = gsk::TransformNode::new(frame, Some(&transform));
    let raster_bounds = graphene::Rect::new(0.0, 0.0, raster_size.0 as f32, raster_size.1 as f32);
    let texture = renderer.render_texture(&scaled, Some(&raster_bounds));
    let bytes = texture_byte_size((texture.width(), texture.height()), 1)?;
    let snapshot = gtk::Snapshot::new();
    snapshot.append_scaled_texture(
        &texture,
        gsk::ScalingFilter::Linear,
        &scene_bounds(logical_size),
    );
    Some((snapshot.to_node()?, bytes))
}

fn phase_texture_size(
    logical_size: (i32, i32), scale_factor: i32, frame_count: usize, budget: usize,
) -> (i32, i32) {
    if logical_size.0 <= 0 || logical_size.1 <= 0 || frame_count == 0 {
        return (0, 0);
    }
    let max_pixels = budget / frame_count / TEXTURE_BYTES_PER_PIXEL;
    if max_pixels == 0 {
        return (0, 0);
    }

    let scale = scale_factor.max(1) as f64 / PHASE_DOWNSAMPLE;
    let target_width = (logical_size.0 as f64 * scale).max(1.0);
    let target_height = (logical_size.1 as f64 * scale).max(1.0);
    let budget_scale = ((max_pixels as f64 / (target_width * target_height)).sqrt()).min(1.0);
    (
        (target_width * budget_scale).floor().max(1.0) as i32,
        (target_height * budget_scale).floor().max(1.0) as i32,
    )
}

fn phase_scene_texture_budget() -> usize {
    PHASE_TEXTURE_BUDGET_BYTES / MAX_CACHED_SCENES
}

fn texture_byte_size(size: (i32, i32), frame_count: usize) -> Option<usize> {
    usize::try_from(size.0)
        .ok()?
        .checked_mul(usize::try_from(size.1).ok()?)?
        .checked_mul(frame_count)?
        .checked_mul(TEXTURE_BYTES_PER_PIXEL)
}

fn cover_crop_node(cover: &gdk::Paintable, size: (i32, i32)) -> Option<gsk::RenderNode> {
    let width = size.0 as f64;
    let height = size.1 as f64;
    let intrinsic_width = cover.intrinsic_width().max(1) as f64;
    let intrinsic_height = cover.intrinsic_height().max(1) as f64;
    let scale = (width / intrinsic_width).max(height / intrinsic_height) * 1.08;
    let draw_width = intrinsic_width * scale;
    let draw_height = intrinsic_height * scale;

    let snapshot = gtk::Snapshot::new();
    snapshot.translate(&graphene::Point::new(
        ((width - draw_width) * 0.5) as f32,
        ((height - draw_height) * 0.5) as f32,
    ));
    cover.snapshot(&snapshot, draw_width, draw_height);
    snapshot.to_node()
}

fn build_base_node(
    cover: Option<gsk::RenderNode>, palette: &CoverPalette, bounds: &graphene::Rect,
) -> gsk::RenderNode {
    let snapshot = gtk::Snapshot::new();
    snapshot.append_color(&palette.dominant, bounds);
    if let Some(cover) = cover {
        snapshot.push_opacity(0.72);
        snapshot.append_node(&cover);
        snapshot.pop();
    }
    snapshot.append_linear_gradient(
        bounds,
        &graphene::Point::new(0.0, 0.0),
        &graphene::Point::new(bounds.width(), bounds.height()),
        &[
            gsk::ColorStop::new(0.0, with_alpha(&palette.dominant, 0.18)),
            gsk::ColorStop::new(0.54, with_alpha(&palette.accent, 0.20)),
            gsk::ColorStop::new(1.0, with_alpha(&palette.secondary, 0.26)),
        ],
    );
    snapshot
        .to_node()
        .expect("base scene always contains a color node")
}

fn radial_node(
    bounds: &graphene::Rect, color: &gdk::RGBA, center: (f32, f32), radius: f32,
) -> gsk::RenderNode {
    let snapshot = gtk::Snapshot::new();
    let max_dimension = bounds.width().max(bounds.height());
    snapshot.append_radial_gradient(
        bounds,
        &graphene::Point::new(bounds.width() * center.0, bounds.height() * center.1),
        max_dimension * radius,
        max_dimension * radius,
        0.0,
        1.0,
        &[
            gsk::ColorStop::new(0.0, with_alpha(color, 0.58)),
            gsk::ColorStop::new(0.52, with_alpha(color, 0.20)),
            gsk::ColorStop::new(1.0, with_alpha(color, 0.0)),
        ],
    );
    snapshot
        .to_node()
        .expect("radial gradient has render content")
}

fn conic_node(
    bounds: &graphene::Rect, dominant: &gdk::RGBA, accent: &gdk::RGBA,
) -> gsk::RenderNode {
    let snapshot = gtk::Snapshot::new();
    snapshot.append_conic_gradient(
        bounds,
        &graphene::Point::new(bounds.width() * 0.52, bounds.height() * 0.46),
        24.0,
        &[
            gsk::ColorStop::new(0.0, with_alpha(dominant, 0.0)),
            gsk::ColorStop::new(0.30, with_alpha(accent, 0.16)),
            gsk::ColorStop::new(0.68, with_alpha(dominant, 0.02)),
            gsk::ColorStop::new(1.0, with_alpha(accent, 0.0)),
        ],
    );
    snapshot
        .to_node()
        .expect("conic gradient has render content")
}

fn scrim_node(bounds: &graphene::Rect, palette: &CoverPalette) -> gsk::RenderNode {
    let snapshot = gtk::Snapshot::new();
    snapshot.append_color(&palette.scrim, bounds);
    snapshot.append_linear_gradient(
        bounds,
        &graphene::Point::new(bounds.width() * 0.5, 0.0),
        &graphene::Point::new(bounds.width() * 0.5, bounds.height()),
        &[
            gsk::ColorStop::new(0.0, gdk::RGBA::new(0.0, 0.0, 0.0, 0.16)),
            gsk::ColorStop::new(0.48, gdk::RGBA::new(0.0, 0.0, 0.0, 0.03)),
            gsk::ColorStop::new(1.0, gdk::RGBA::new(0.0, 0.0, 0.0, 0.30)),
        ],
    );
    snapshot.to_node().expect("scrim has render content")
}

fn compose_live_scene(
    scene: &LiveScene, phase: f32, quality: BackgroundQuality,
) -> gsk::RenderNode {
    let snapshot = gtk::Snapshot::new();
    snapshot.append_node(&scene.base);

    let angle = phase * std::f32::consts::TAU;
    let amplitude = if quality == BackgroundQuality::Eco {
        0.0
    } else {
        18.0
    };
    append_moving_layer(
        &snapshot,
        &scene.glow_a,
        scene.size,
        angle.sin() * amplitude,
        angle.cos() * amplitude * 0.7,
        1.02 + 0.025 * angle.cos(),
        0.84,
    );
    append_moving_layer(
        &snapshot,
        &scene.glow_b,
        scene.size,
        -angle.cos() * amplitude * 0.8,
        angle.sin() * amplitude,
        1.03 + 0.02 * angle.sin(),
        0.76,
    );
    if let Some(glow) = scene.glow_c.as_ref() {
        append_moving_layer(
            &snapshot,
            glow,
            scene.size,
            0.0,
            0.0,
            1.06 + 0.018 * (angle * 0.5).sin(),
            0.62,
        );
    }

    snapshot.append_node(&scene.scrim);
    snapshot
        .to_node()
        .expect("composed scene has render content")
}

fn append_moving_layer(
    snapshot: &gtk::Snapshot, layer: &gsk::RenderNode, size: (i32, i32), offset_x: f32,
    offset_y: f32, scale: f32, opacity: f32,
) {
    let center = graphene::Point::new(size.0 as f32 * 0.5, size.1 as f32 * 0.5);
    let transform = gsk::Transform::new()
        .translate(&graphene::Point::new(
            center.x() + offset_x,
            center.y() + offset_y,
        ))
        .scale(scale, scale)
        .translate(&graphene::Point::new(-center.x(), -center.y()));
    let transformed = gsk::TransformNode::new(layer, Some(&transform));
    let faded = gsk::OpacityNode::new(&transformed, opacity);
    snapshot.append_node(&faded);
}

fn with_alpha(color: &gdk::RGBA, alpha: f32) -> gdk::RGBA {
    let mut color = *color;
    color.set_alpha(alpha.clamp(0.0, 1.0));
    color
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paintable_equality_uses_object_identity() {
        let bytes = glib::Bytes::from_static(&[16, 32, 64, 255]);
        let first: gdk::Paintable =
            gdk::MemoryTexture::new(1, 1, gdk::MemoryFormat::R8g8b8a8, &bytes, 4).upcast();
        let alias = first.clone();
        let distinct: gdk::Paintable =
            gdk::MemoryTexture::new(1, 1, gdk::MemoryFormat::R8g8b8a8, &bytes, 4).upcast();

        assert!(same_paintable(None, None));
        assert!(same_paintable(Some(&first), Some(&alias)));
        assert!(!same_paintable(Some(&first), Some(&distinct)));
        assert!(!same_paintable(Some(&first), None));
    }

    #[test]
    fn phase_sample_interpolates_and_wraps() {
        assert_eq!(
            phase_sample(0.25, 10),
            PhaseSample {
                from: 2,
                to: 3,
                mix: 0.5,
            }
        );
        assert_eq!(
            phase_sample(0.95, 10),
            PhaseSample {
                from: 9,
                to: 0,
                mix: 0.5,
            }
        );
        assert_eq!(phase_sample(1.0, 10), phase_sample(0.0, 10));
        assert_eq!(phase_sample(f32::NAN, 10), phase_sample(0.0, 10));
    }

    #[test]
    fn phase_sample_handles_static_frame() {
        assert_eq!(
            phase_sample(0.73, 1),
            PhaseSample {
                from: 0,
                to: 0,
                mix: 0.0,
            }
        );
    }

    #[test]
    fn texture_size_tracks_physical_scale_before_budget_cap() {
        let budget = phase_scene_texture_budget();
        assert_eq!(phase_texture_size((400, 300), 1, 10, budget), (100, 75));
        assert_eq!(phase_texture_size((400, 300), 2, 10, budget), (200, 150));
    }

    #[test]
    fn texture_size_keeps_two_scenes_within_total_budget() {
        let scene_budget = phase_scene_texture_budget();
        let size = phase_texture_size((3840, 2160), 2, PHASE_FRAME_COUNT, scene_budget);
        let scene_bytes = texture_byte_size(size, PHASE_FRAME_COUNT).unwrap();
        assert!(scene_bytes <= scene_budget);
        assert!(scene_bytes * MAX_CACHED_SCENES <= PHASE_TEXTURE_BUDGET_BYTES);
    }
}
