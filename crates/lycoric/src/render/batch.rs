use gtk::{
    gdk,
    graphene,
    gsk,
    gsk::prelude::IsRenderNode,
    prelude::*,
};

use crate::render::{
    layout::DocumentLayout,
    visual::LineVisual,
};

const TEXTURE_BYTES_PER_PIXEL: usize = 4;
const MAX_TEXTURE_DIMENSION: usize = 4096;
pub(crate) const CURRENT_LINE_TEXTURE_BUDGET: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BatchKey {
    pub layout_generation: u64,
    pub scale_generation: u64,
    pub scale_bits: u64,
    pub visible_start: usize,
    pub visible_end: usize,
    pub current_line: Option<usize>,
    pub manual_scroll: bool,
}

impl BatchKey {
    pub fn new(
        layout_generation: u64, scale_generation: u64, scale: f64, visible: std::ops::Range<usize>,
        current_line: Option<usize>, manual_scroll: bool,
    ) -> Self {
        Self {
            layout_generation,
            scale_generation,
            scale_bits: scale.to_bits(),
            visible_start: visible.start,
            visible_end: visible.end,
            current_line,
            manual_scroll,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AtlasPolicy {
    pub min_viewport_ratio: f64,
    pub max_viewport_ratio: f64,
    pub max_pixel_area: f64,
    pub max_pixel_dimension: f64,
}

impl AtlasPolicy {
    pub fn should_bake(
        self, scene_bounds: &graphene::Rect, viewport: &graphene::Rect, scale: f64,
    ) -> bool {
        if !valid_rect(scene_bounds) || !valid_rect(viewport) || !valid_scale(scale) {
            return false;
        }
        let scene_area = rect_area(scene_bounds) * scale * scale;
        let viewport_area = rect_area(viewport) * scale * scale;
        let ratio = scene_area / viewport_area.max(1.0);
        let pixel_width = scene_bounds.width() as f64 * scale;
        let pixel_height = scene_bounds.height() as f64 * scale;
        ratio >= self.min_viewport_ratio
            && ratio <= self.max_viewport_ratio
            && scene_area <= self.max_pixel_area
            && pixel_width <= self.max_pixel_dimension
            && pixel_height <= self.max_pixel_dimension
    }
}

impl Default for AtlasPolicy {
    fn default() -> Self {
        Self {
            min_viewport_ratio: 0.6,
            max_viewport_ratio: 6.0,
            max_pixel_area: 16_777_216.0,
            max_pixel_dimension: 4096.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct RasterPlan {
    physical_bounds: graphene::Rect,
    logical_destination: graphene::Rect,
    pixel_width: usize,
    pixel_height: usize,
}

impl RasterPlan {
    fn new(bounds: &graphene::Rect, scale: f64, max_dimension: usize) -> Option<Self> {
        if !valid_rect(bounds) || !valid_scale(scale) || max_dimension == 0 {
            return None;
        }
        let (left, pixel_width) = aligned_axis(bounds.x(), bounds.width(), scale, max_dimension)?;
        let (top, pixel_height) = aligned_axis(bounds.y(), bounds.height(), scale, max_dimension)?;
        let physical_bounds = rect_from_pixels(left, top, pixel_width, pixel_height)?;
        let logical_destination = graphene::Rect::new(
            (left / scale) as f32,
            (top / scale) as f32,
            (pixel_width as f64 / scale) as f32,
            (pixel_height as f64 / scale) as f32,
        );
        valid_rect(&logical_destination).then_some(Self {
            physical_bounds,
            logical_destination,
            pixel_width,
            pixel_height,
        })
    }

    fn byte_size(self) -> Option<usize> {
        self.pixel_width
            .checked_mul(self.pixel_height)?
            .checked_mul(TEXTURE_BYTES_PER_PIXEL)
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct TextureBudget {
    limit: usize,
    used: usize,
    max_dimension: usize,
}

impl TextureBudget {
    pub(crate) fn current_line() -> Self {
        Self {
            limit: CURRENT_LINE_TEXTURE_BUDGET,
            used: 0,
            max_dimension: MAX_TEXTURE_DIMENSION,
        }
    }

    fn reserve(&mut self, plan: RasterPlan) -> Option<usize> {
        let bytes = plan.byte_size()?;
        let used = self.used.checked_add(bytes)?;
        if used > self.limit {
            return None;
        }
        self.used = used;
        Some(bytes)
    }

    fn release(&mut self, bytes: usize) {
        self.used = self.used.saturating_sub(bytes);
    }
}

pub(crate) fn bake_texture_node(
    renderer: &gsk::Renderer, node: &gsk::RenderNode, scale: f64, budget: &mut TextureBudget,
) -> Option<gsk::RenderNode> {
    let plan = RasterPlan::new(&node.bounds(), scale, budget.max_dimension)?;
    let reservation = budget.reserve(plan)?;
    let Some(texture) = render_texture(renderer, node, scale, plan) else {
        budget.release(reservation);
        return None;
    };
    Some(gsk::TextureNode::new(&texture, &plan.logical_destination).upcast())
}

#[derive(Clone)]
pub struct SharedTextureAtlas {
    node: gsk::RenderNode,
    scale_bits: u64,
}

impl SharedTextureAtlas {
    pub fn bake(
        renderer: &gsk::Renderer, node: &gsk::RenderNode, bounds: &graphene::Rect, scale: f64,
    ) -> Option<Self> {
        let plan = RasterPlan::new(bounds, scale, MAX_TEXTURE_DIMENSION)?;
        let texture = render_texture(renderer, node, scale, plan)?;
        let node = gsk::TextureNode::new(&texture, &plan.logical_destination).upcast();
        Some(Self {
            node,
            scale_bits: scale.to_bits(),
        })
    }

    pub fn node(&self, scale: f64) -> Option<gsk::RenderNode> {
        (self.scale_bits == scale.to_bits()).then(|| self.node.clone())
    }
}

#[derive(Clone)]
struct BatchEntry {
    key: BatchKey,
    fallback: Option<gsk::RenderNode>,
    atlas: Option<SharedTextureAtlas>,
}

#[derive(Default)]
pub struct BatchCache {
    entry: Option<BatchEntry>,
    policy: AtlasPolicy,
}

impl BatchCache {
    pub fn matches(&self, key: BatchKey) -> bool {
        self.entry.as_ref().is_some_and(|entry| entry.key == key)
    }

    pub fn replace(
        &mut self, key: BatchKey, fallback: Option<gsk::RenderNode>, viewport: &graphene::Rect,
        renderer: Option<&gsk::Renderer>,
    ) {
        let scale = f64::from_bits(key.scale_bits);
        let atlas = fallback.as_ref().and_then(|node| {
            let bounds = node.bounds();
            if !self.policy.should_bake(&bounds, viewport, scale) {
                return None;
            }
            renderer.and_then(|renderer| SharedTextureAtlas::bake(renderer, node, &bounds, scale))
        });
        self.entry = Some(BatchEntry {
            key,
            fallback,
            atlas,
        });
    }

    pub fn node(&self, key: BatchKey) -> Option<gsk::RenderNode> {
        let entry = self.entry.as_ref().filter(|entry| entry.key == key)?;
        entry
            .atlas
            .as_ref()
            .and_then(|atlas| atlas.node(f64::from_bits(key.scale_bits)))
            .or_else(|| entry.fallback.clone())
    }

    pub fn has_atlas(&self) -> bool {
        self.entry
            .as_ref()
            .is_some_and(|entry| entry.atlas.is_some())
    }

    pub fn invalidate(&mut self) {
        self.entry = None;
    }
}

pub fn compose_static_scene(
    layout: &DocumentLayout, visuals: &[Option<LineVisual>], visible: std::ops::Range<usize>,
    current_line: Option<usize>, manual_scroll: bool,
) -> Option<gsk::RenderNode> {
    let snapshot = gtk::Snapshot::new();
    for index in visible {
        if Some(index) == current_line {
            continue;
        }
        let Some((line, visual)) = layout
            .line(index)
            .zip(visuals.get(index).and_then(Option::as_ref))
        else {
            continue;
        };
        let blur = line_blur(layout, index, current_line, manual_scroll);
        if blur > 0.0 {
            snapshot.push_blur(blur);
        }
        visual.append_normal(&snapshot, line);
        if blur > 0.0 {
            snapshot.pop();
        }
    }
    snapshot.to_node()
}

fn line_blur(
    layout: &DocumentLayout, index: usize, current_line: Option<usize>, manual_scroll: bool,
) -> f64 {
    if manual_scroll {
        return 0.0;
    }
    let Some(current_center) = current_line.and_then(|current| layout.line_center(current)) else {
        return 0.0;
    };
    let Some(line_center) = layout.line_center(index) else {
        return 0.0;
    };
    blur_for_distance((line_center - current_center).abs())
}

pub(crate) fn blur_for_distance(distance: f32) -> f64 {
    (distance.max(0.0) as f64 / 80.0).min(10.0)
}

fn render_texture(
    renderer: &gsk::Renderer, node: &gsk::RenderNode, scale: f64, plan: RasterPlan,
) -> Option<gdk::Texture> {
    let transform = gsk::Transform::new().scale(scale as f32, scale as f32);
    let scaled_node = gsk::TransformNode::new(node, Some(&transform)).upcast();
    let texture = renderer.render_texture(&scaled_node, Some(&plan.physical_bounds));
    dimensions_match(&texture, plan).then_some(texture)
}

fn dimensions_match(texture: &gdk::Texture, plan: RasterPlan) -> bool {
    usize::try_from(texture.width()).ok() == Some(plan.pixel_width)
        && usize::try_from(texture.height()).ok() == Some(plan.pixel_height)
}

fn aligned_axis(
    origin: f32, extent: f32, scale: f64, max_dimension: usize,
) -> Option<(f64, usize)> {
    let start = (origin as f64 * scale).floor();
    let end = ((origin as f64 + extent as f64) * scale).ceil();
    let pixels = end - start;
    if !start.is_finite()
        || !end.is_finite()
        || pixels < 1.0
        || pixels > max_dimension as f64
        || start.abs() > f32::MAX as f64
        || end.abs() > f32::MAX as f64
    {
        return None;
    }
    Some((start, pixels as usize))
}

fn rect_from_pixels(left: f64, top: f64, width: usize, height: usize) -> Option<graphene::Rect> {
    let rect = graphene::Rect::new(left as f32, top as f32, width as f32, height as f32);
    valid_rect(&rect).then_some(rect)
}

fn valid_scale(scale: f64) -> bool {
    scale.is_finite() && scale > 0.0 && scale <= f32::MAX as f64
}

fn rect_area(rect: &graphene::Rect) -> f64 {
    rect.width() as f64 * rect.height() as f64
}

fn valid_rect(rect: &graphene::Rect) -> bool {
    rect.x().is_finite()
        && rect.y().is_finite()
        && rect.width().is_finite()
        && rect.height().is_finite()
        && rect.width() > 0.0
        && rect.height() > 0.0
        && (rect.x() + rect.width()).is_finite()
        && (rect.y() + rect.height()).is_finite()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_policy_is_driven_by_visible_area_and_pixel_limits() {
        let policy = AtlasPolicy::default();
        let viewport = graphene::Rect::new(0.0, 0.0, 100.0, 100.0);
        let useful_scene = graphene::Rect::new(0.0, 0.0, 120.0, 100.0);
        let tiny_scene = graphene::Rect::new(0.0, 0.0, 10.0, 10.0);
        let oversized_scene = graphene::Rect::new(0.0, 0.0, 5_000.0, 100.0);

        assert!(policy.should_bake(&useful_scene, &viewport, 1.0));
        assert!(!policy.should_bake(&tiny_scene, &viewport, 1.0));
        assert!(!policy.should_bake(&oversized_scene, &viewport, 1.0));
    }

    #[test]
    fn raster_plan_aligns_negative_bounds_and_restores_logical_destination() {
        let logical = graphene::Rect::new(-1.25, -2.25, 2.5, 1.5);
        let plan = RasterPlan::new(&logical, 2.0, MAX_TEXTURE_DIMENSION).unwrap();

        assert_eq!(
            plan.physical_bounds,
            graphene::Rect::new(-3.0, -5.0, 6.0, 4.0)
        );
        assert_eq!(
            plan.logical_destination,
            graphene::Rect::new(-1.5, -2.5, 3.0, 2.0)
        );
    }

    #[test]
    fn current_line_budget_enforces_eight_mib_and_max_dimension() {
        let mut budget = TextureBudget::current_line();
        let full_budget = RasterPlan::new(
            &graphene::Rect::new(0.0, 0.0, 4096.0, 512.0),
            1.0,
            budget.max_dimension,
        )
        .unwrap();
        assert_eq!(
            budget.reserve(full_budget),
            Some(CURRENT_LINE_TEXTURE_BUDGET)
        );

        let extra = RasterPlan::new(
            &graphene::Rect::new(0.0, 0.0, 1.0, 1.0),
            1.0,
            budget.max_dimension,
        )
        .unwrap();
        assert_eq!(budget.reserve(extra), None);
        assert!(
            RasterPlan::new(
                &graphene::Rect::new(0.0, 0.0, 4097.0, 1.0),
                1.0,
                MAX_TEXTURE_DIMENSION,
            )
            .is_none()
        );
    }

    #[test]
    fn invalid_scale_never_becomes_a_unit_scale_batch() {
        let invalid = BatchKey::new(7, 1, f64::NAN, 2..8, Some(4), false);
        let unit = BatchKey::new(7, 1, 1.0, 2..8, Some(4), false);
        assert_ne!(invalid, unit);
    }

    #[test]
    fn manual_scroll_invalidates_the_blurred_static_batch() {
        let automatic = BatchKey::new(7, 1, 1.0, 2..8, Some(4), false);
        let manual = BatchKey::new(7, 1, 1.0, 2..8, Some(4), true);
        assert_ne!(automatic, manual);
        assert_eq!(blur_for_distance(0.0), 0.0);
        assert!(blur_for_distance(320.0) > blur_for_distance(80.0));
    }

    #[test]
    fn batch_key_invalidates_on_physical_scale_mismatch() {
        let first = BatchKey::new(7, 1, 1.0, 2..8, Some(4), false);
        let second = BatchKey::new(7, 1, 2.0, 2..8, Some(4), false);
        assert_ne!(first, second);
    }
}
