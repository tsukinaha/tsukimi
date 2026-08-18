use crate::{
    Color,
    DanmakwRenderer,
};
use gtk::{
    gdk,
    graphene,
    gsk,
    prelude::*,
};

const SHADOW_ALPHA: f32 = 0.65;
const OUTLINE_ALPHA: f32 = 0.95;

fn ring_samples(outline_px: f32) -> usize {
    ((outline_px * std::f32::consts::TAU).ceil() as usize).clamp(8, 24)
}

struct Style {
    ring: Vec<graphene::Point>,
    outline: gdk::RGBA,
    shadow: Option<graphene::Point>,
    shadow_color: gdk::RGBA,
}

const ATLAS_LIMIT: f32 = 4096.0;

struct Baked {
    atlas: gdk::Texture,
    slot: graphene::Point,
    dest: graphene::Rect,
    scale: f64,
}

pub struct DanmakuVisual {
    node: gsk::RenderNode,
    baked: Option<Baked>,
}

impl DanmakuVisual {
    pub fn new(
        layout: &pango::Layout, outline_px: f64, shadow_offset: f64, color: Color,
    ) -> Option<Self> {
        let style = Style::new(outline_px, shadow_offset);
        let snapshot = gtk::Snapshot::new();
        style.append(&snapshot, layout, &color.into());
        Some(Self {
            node: snapshot.to_node()?,
            baked: None,
        })
    }

    pub fn needs_bake(&self, scale: f64) -> bool {
        !self.baked.as_ref().is_some_and(|b| b.scale == scale)
    }

    fn slot_size(&self, scale: f32) -> graphene::Rect {
        let b = self.node.bounds();
        let x = (b.x() * scale).floor();
        let y = (b.y() * scale).floor();
        graphene::Rect::new(
            x,
            y,
            ((b.x() + b.width()) * scale).ceil() - x,
            ((b.y() + b.height()) * scale).ceil() - y,
        )
    }

    fn append(&self, snapshot: &gtk::Snapshot, x: f32, y: f32) {
        let Some(baked) = &self.baked else {
            snapshot.save();
            snapshot.translate(&graphene::Point::new(x, y));
            snapshot.append_node(&self.node);
            snapshot.restore();
            return;
        };

        let d = &baked.dest;
        let dest = graphene::Rect::new(x + d.x(), y + d.y(), d.width(), d.height());
        let s = baked.scale as f32;

        snapshot.push_clip(&dest);
        snapshot.append_texture(
            &baked.atlas,
            &graphene::Rect::new(
                dest.x() - baked.slot.x(),
                dest.y() - baked.slot.y(),
                baked.atlas.width() as f32 / s,
                baked.atlas.height() as f32 / s,
            ),
        );
        snapshot.pop();
    }
}

pub fn bake_batch(renderer: &gsk::Renderer, scale: f64, visuals: &mut [&mut DanmakuVisual]) {
    let mut rest = visuals;
    while !rest.is_empty() {
        let done = bake_one_atlas(renderer, scale, rest);
        rest = &mut rest[done..];
    }
}

fn bake_one_atlas(
    renderer: &gsk::Renderer, scale: f64, visuals: &mut [&mut DanmakuVisual],
) -> usize {
    let s = scale as f32;

    let mut placed: Vec<(graphene::Rect, f32, f32)> = Vec::new();
    let (mut shelf_x, mut shelf_y, mut shelf_h) = (0.0f32, 0.0f32, 0.0f32);
    let (mut used_w, mut used_h) = (0.0f32, 0.0f32);

    for visual in visuals.iter() {
        let src = visual.slot_size(s);
        if !placed.is_empty() && shelf_x + src.width() > ATLAS_LIMIT {
            shelf_x = 0.0;
            shelf_y += shelf_h;
            shelf_h = 0.0;
        }

        if !placed.is_empty() && shelf_y + src.height() > ATLAS_LIMIT {
            break;
        }
        placed.push((src, shelf_x, shelf_y));
        shelf_x += src.width();
        shelf_h = shelf_h.max(src.height());
        used_w = used_w.max(shelf_x);
        used_h = used_h.max(shelf_y + shelf_h);
    }

    let snapshot = gtk::Snapshot::new();
    snapshot.scale(s, s);
    for (visual, (src, ax, ay)) in visuals.iter().zip(placed.iter()) {
        snapshot.save();
        snapshot.translate(&graphene::Point::new(
            (ax - src.x()) / s,
            (ay - src.y()) / s,
        ));
        snapshot.append_node(&visual.node);
        snapshot.restore();
    }
    let Some(root) = snapshot.to_node() else {
        return placed.len().max(1);
    };
    let atlas =
        renderer.render_texture(&root, Some(&graphene::Rect::new(0.0, 0.0, used_w, used_h)));

    for (visual, (src, ax, ay)) in visuals.iter_mut().zip(placed.iter()) {
        visual.baked = Some(Baked {
            atlas: atlas.clone(),
            slot: graphene::Point::new(ax / s, ay / s),
            dest: graphene::Rect::new(src.x() / s, src.y() / s, src.width() / s, src.height() / s),
            scale,
        });
    }
    placed.len().max(1)
}

impl Style {
    fn new(outline_px: f64, shadow_offset: f64) -> Self {
        let outline_px = outline_px as f32;
        let ring = if outline_px > 0.0 {
            let samples = ring_samples(outline_px);
            (0..samples)
                .map(|i| {
                    let angle = std::f32::consts::TAU * i as f32 / samples as f32;
                    graphene::Point::new(angle.cos() * outline_px, angle.sin() * outline_px)
                })
                .collect()
        } else {
            Vec::new()
        };

        let shadow_offset = shadow_offset as f32;

        Self {
            ring,
            outline: gdk::RGBA::new(0.0, 0.0, 0.0, OUTLINE_ALPHA),
            shadow: (shadow_offset != 0.0)
                .then(|| graphene::Point::new(shadow_offset, shadow_offset)),
            shadow_color: gdk::RGBA::new(0.0, 0.0, 0.0, SHADOW_ALPHA),
        }
    }

    fn append(&self, snapshot: &gtk::Snapshot, layout: &pango::Layout, color: &gdk::RGBA) {
        if let Some(offset) = self.shadow {
            snapshot.save();
            snapshot.translate(&offset);
            snapshot.append_layout(layout, &self.shadow_color);
            snapshot.restore();
        }

        if !self.ring.is_empty() {
            let outline = gtk::Snapshot::new();
            outline.append_layout(layout, &self.outline);
            if let Some(outline) = outline.to_node() {
                for offset in &self.ring {
                    snapshot.save();
                    snapshot.translate(offset);
                    snapshot.append_node(&outline);
                    snapshot.restore();
                }
            }
        }

        snapshot.append_layout(layout, color);
    }
}

pub trait DanmakwSnapshotExt {
    fn render_danmakw(&self, renderer: &mut DanmakwRenderer, width: f32, height: f32);
}

impl DanmakwSnapshotExt for gtk::Snapshot {
    fn render_danmakw(&self, renderer: &mut DanmakwRenderer, width: f32, height: f32) {
        if renderer.screen_height != height {
            renderer.screen_height = height;
            renderer.recompute_max_rows();
        }

        for sd in renderer.scroll_danmaku.iter() {
            sd.visual
                .append(self, sd.x, renderer.scrolled_top_y(sd.row));
        }

        for cd in renderer.top_center_danmaku.iter() {
            let x = (width - cd.width) / 2.0;
            cd.visual.append(self, x, renderer.top_center_y(cd.row));
        }

        for cd in renderer.bottom_center_danmaku.iter() {
            let x = (width - cd.width) / 2.0;
            cd.visual
                .append(self, x, renderer.bottom_center_y(cd.row, height));
        }
    }
}
