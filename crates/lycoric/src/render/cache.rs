use std::ops::Range;

use gtk::{
    graphene,
    gsk,
    pango,
};

use crate::{
    model::LyricsDocument,
    render::{
        batch::{
            BatchCache,
            BatchKey,
            compose_static_scene,
        },
        layout::{
            DocumentLayout,
            LaneVisibility,
            ViewportAnchor,
        },
        style::{
            LyricsStyle,
            Overscan,
        },
        visual::{
            GapVisual,
            LineVisual,
            VisualSignature,
            visual_signature,
        },
    },
    time::LyricTime,
    timeline::Timeline,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CacheStamp {
    pub document_generation: u64,
    pub width_generation: u64,
    pub style_generation: u64,
    pub visibility_generation: u64,
    pub environment_generation: u64,
}

#[derive(Clone, Copy)]
pub struct CacheBuild<'a> {
    pub document: &'a LyricsDocument,
    pub timeline: &'a Timeline,
    pub context: &'a pango::Context,
    pub width: f32,
    pub style: &'a LyricsStyle,
    pub visibility: LaneVisibility,
    pub stamp: CacheStamp,
}

#[derive(Clone)]
pub struct BatchPrepare<'a> {
    pub visible: Range<usize>,
    pub current_line: Option<usize>,
    pub scale_generation: u64,
    pub scale_factor: f64,
    pub manual_scroll: bool,
    pub allow_texture_bake: bool,
    pub viewport: &'a graphene::Rect,
    pub renderer: Option<&'a gsk::Renderer>,
}

#[derive(Clone, Debug, Default)]
pub struct ViewportUpdate {
    pub visible: Range<usize>,
    pub scroll_correction: f32,
    pub changed: bool,
}

#[derive(Default)]
pub struct RenderCache {
    generation: u64,
    stamp: CacheStamp,
    layout: Option<DocumentLayout>,
    visuals: Vec<Option<LineVisual>>,
    visual_range: Range<usize>,
    batch: BatchCache,
    gap: GapVisual,
    baked_current_line: Option<usize>,
}

impl RenderCache {
    /// Resets only the lightweight geometry index. Pango layouts and render
    /// nodes are created later by `ensure_viewport` for the requested window.
    pub fn rebuild(&mut self, build: CacheBuild<'_>) {
        self.generation = self.generation.wrapping_add(1);
        self.stamp = build.stamp;
        let layout = DocumentLayout::new(
            build.document,
            build.timeline,
            build.width,
            build.style,
            build.visibility,
        );
        self.visuals = vec![None; layout.len()];
        self.visual_range = 0..0;
        self.layout = Some(layout);
        self.gap = GapVisual::build(build.style);
        self.baked_current_line = None;
        self.batch.invalidate();
    }

    pub fn ensure_viewport(
        &mut self, build: CacheBuild<'_>, scroll_offset: f32, viewport_height: f32,
        overscan: Overscan, anchor: ViewportAnchor,
    ) -> ViewportUpdate {
        if self.layout.is_none() || self.stamp != build.stamp {
            self.rebuild(build);
        }
        let mut corrected_scroll = scroll_offset;
        let mut total_correction = 0.0;
        let mut changed = false;
        let mut visible = 0..0;

        for _ in 0..3 {
            let requested = self
                .layout
                .as_ref()
                .map(|layout| layout.visible_range(corrected_scroll, viewport_height, overscan))
                .unwrap_or(0..0);
            let update = self.layout.as_mut().map(|layout| {
                layout.ensure_range(
                    build.document,
                    build.context,
                    build.style,
                    build.visibility,
                    requested.clone(),
                    anchor,
                )
            });
            let Some(update) = update else {
                break;
            };
            self.retain_visuals(update.range.clone());
            for index in update.rebuilt.iter().copied() {
                let visual = self
                    .layout
                    .as_ref()
                    .and_then(|layout| layout.line(index))
                    .map(|line| LineVisual::build(line, build.style));
                if let Some(slot) = self.visuals.get_mut(index) {
                    *slot = visual;
                }
            }
            changed |= update.changed;
            corrected_scroll += update.scroll_correction;
            total_correction += update.scroll_correction;
            visible = update.range;
            let stable = self.layout.as_ref().is_some_and(|layout| {
                layout.visible_range(corrected_scroll, viewport_height, overscan) == visible
            });
            if stable {
                break;
            }
        }

        if changed {
            self.generation = self.generation.wrapping_add(1);
            self.batch.invalidate();
        }
        ViewportUpdate {
            visible,
            scroll_correction: finite_delta(total_correction),
            changed,
        }
    }

    pub fn prepare_batch(&mut self, prepare: BatchPrepare<'_>) {
        let renderer = if prepare.allow_texture_bake {
            prepare.renderer
        } else {
            None
        };
        self.prepare_current_bake(
            prepare.current_line,
            prepare.scale_generation,
            prepare.scale_factor,
            renderer,
        );
        let key = BatchKey::new(
            self.generation,
            prepare.scale_generation,
            prepare.scale_factor,
            prepare.visible.clone(),
            prepare.current_line,
            prepare.manual_scroll,
        );
        if self.batch.matches(key)
            && (!prepare.allow_texture_bake || self.batch.has_atlas() || renderer.is_none())
        {
            return;
        }
        let fallback = self.layout.as_ref().and_then(|layout| {
            compose_static_scene(
                layout,
                &self.visuals,
                prepare.visible,
                prepare.current_line,
                prepare.manual_scroll,
            )
        });
        self.batch
            .replace(key, fallback, prepare.viewport, renderer);
    }

    pub fn unbatched_static_node(
        &self, visible: Range<usize>, current_line: Option<usize>, manual_scroll: bool,
    ) -> Option<gsk::RenderNode> {
        compose_static_scene(
            self.layout.as_ref()?,
            &self.visuals,
            visible,
            current_line,
            manual_scroll,
        )
    }

    pub fn static_node(
        &self, visible: Range<usize>, current_line: Option<usize>, scale_generation: u64,
        scale_factor: f64, manual_scroll: bool,
    ) -> Option<gsk::RenderNode> {
        let key = BatchKey::new(
            self.generation,
            scale_generation,
            scale_factor,
            visible,
            current_line,
            manual_scroll,
        );
        self.batch.node(key)
    }

    pub fn visual_signature(
        &self, current_line: Option<usize>, position: LyricTime,
    ) -> VisualSignature {
        let line = current_line.and_then(|index| {
            self.layout
                .as_ref()
                .and_then(|layout| layout.line(index))
                .zip(self.visuals.get(index).and_then(Option::as_ref))
        });
        visual_signature(line, current_line, position)
    }

    pub fn line_scene(
        &self, index: usize,
    ) -> Option<(&crate::render::layout::LineLayout, &LineVisual)> {
        self.layout
            .as_ref()
            .and_then(|layout| layout.line(index))
            .zip(self.visuals.get(index).and_then(Option::as_ref))
    }

    pub fn set_gap_expansion(&mut self, gap: Option<usize>, expansion: f32) -> bool {
        let changed = self
            .layout
            .as_mut()
            .is_some_and(|layout| layout.set_gap_expansion(gap, expansion));
        if changed {
            self.generation = self.generation.wrapping_add(1);
            self.batch.invalidate();
        }
        changed
    }

    pub fn gap_node(&self) -> Option<&gsk::RenderNode> {
        self.gap.dot()
    }

    pub fn clear(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.layout = None;
        self.visuals.clear();
        self.visual_range = 0..0;
        self.gap = GapVisual::default();
        self.baked_current_line = None;
        self.batch.invalidate();
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn stamp(&self) -> CacheStamp {
        self.stamp
    }

    pub fn layout(&self) -> Option<&DocumentLayout> {
        self.layout.as_ref()
    }

    pub fn invalidate_batch(&mut self) {
        self.batch.invalidate();
    }

    pub fn invalidate_scale(&mut self) {
        self.switch_baked_current_line(None);
        self.batch.invalidate();
    }

    fn prepare_current_bake(
        &mut self, current_line: Option<usize>, scale_generation: u64, scale: f64,
        renderer: Option<&gsk::Renderer>,
    ) {
        self.switch_baked_current_line(current_line);
        let Some(index) = current_line else {
            return;
        };
        let Some(line) = self.layout.as_ref().and_then(|layout| layout.line(index)) else {
            return;
        };
        if let Some(visual) = self.visuals.get_mut(index).and_then(Option::as_mut) {
            visual.prepare_bake(line, scale_generation, scale, renderer);
        }
    }

    fn switch_baked_current_line(&mut self, current_line: Option<usize>) {
        if self.baked_current_line == current_line {
            return;
        }
        if let Some(index) = self.baked_current_line.take() {
            self.clear_line_bake(index);
        }
        self.baked_current_line = current_line;
    }

    fn clear_line_bake(&mut self, index: usize) {
        let Some(line) = self.layout.as_ref().and_then(|layout| layout.line(index)) else {
            return;
        };
        if let Some(visual) = self.visuals.get_mut(index).and_then(Option::as_mut) {
            visual.clear_bake(line);
        }
    }

    fn retain_visuals(&mut self, range: Range<usize>) {
        for index in self.visual_range.clone() {
            if !range.contains(&index)
                && let Some(visual) = self.visuals.get_mut(index)
            {
                *visual = None;
            }
        }
        self.visual_range = range;
    }
}

fn finite_delta(value: f32) -> f32 {
    if value.is_finite() && value.abs() > 0.01 {
        value
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_is_part_of_the_read_only_batch_lookup() {
        let cache = RenderCache::default();
        assert!(cache.static_node(0..0, None, 1, 1.0, false).is_none());
        assert!(cache.static_node(0..0, None, 1, 2.0, false).is_none());
    }
}
