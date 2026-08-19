use std::ops::Range;

use gtk::{
    graphene,
    pango,
};

#[derive(Clone, Debug)]
struct ClusterGeometry {
    byte_range: Range<usize>,
    rect: graphene::Rect,
    rtl: bool,
}

#[derive(Clone, Debug)]
enum SegmentMask {
    Clusters(Range<usize>),
    Bounds(graphene::Rect),
    Empty,
}

#[derive(Clone, Copy, Debug)]
pub struct RevealPart {
    pub rect: graphene::Rect,
    pub feather: bool,
    pub rtl: bool,
    pub fraction: f32,
}

#[derive(Clone, Debug, Default)]
pub struct HighlightGeometry {
    clusters: Vec<ClusterGeometry>,
    segments: Vec<SegmentMask>,
    bounds: Option<graphene::Rect>,
}

impl HighlightGeometry {
    pub fn new(layout: &pango::Layout, text: &str, segments: &[Range<usize>]) -> Self {
        let bounds = layout_bounds(layout);
        let clusters = collect_clusters(layout, text.len(), bounds.as_ref());
        let masks = segments
            .iter()
            .map(|range| segment_mask(range, &clusters, bounds.as_ref()))
            .collect();
        Self {
            clusters,
            segments: masks,
            bounds,
        }
    }

    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }

    pub fn visit_reveal(
        &self, segment_index: usize, progress: f32, mut visitor: impl FnMut(&graphene::Rect),
    ) {
        let Some(mask) = self.segments.get(segment_index) else {
            return;
        };
        let progress = finite_progress(progress);
        match mask {
            SegmentMask::Clusters(range) => {
                visit_clusters(&self.clusters[range.clone()], progress, &mut visitor)
            }
            SegmentMask::Bounds(bounds) if progress > 0.0 => visitor(bounds),
            SegmentMask::Bounds(_) | SegmentMask::Empty => {}
        }
    }

    pub fn visit_gradient_reveal(
        &self, segment_index: usize, progress: f32, mut visitor: impl FnMut(RevealPart),
    ) {
        let Some(mask) = self.segments.get(segment_index) else {
            return;
        };
        let progress = finite_progress(progress);
        match mask {
            SegmentMask::Clusters(range) => {
                visit_cluster_parts(&self.clusters[range.clone()], progress, &mut visitor)
            }
            SegmentMask::Bounds(bounds) if progress > 0.0 => {
                let width = bounds.width() * progress;
                if width > 0.0 {
                    visitor(RevealPart {
                        rect: graphene::Rect::new(bounds.x(), bounds.y(), width, bounds.height()),
                        feather: progress < 1.0,
                        rtl: false,
                        fraction: progress,
                    });
                }
            }
            SegmentMask::Bounds(_) | SegmentMask::Empty => {}
        }
    }

    pub fn bounds(&self) -> Option<&graphene::Rect> {
        self.bounds.as_ref()
    }
}

fn collect_clusters(
    layout: &pango::Layout, text_len: usize, bounds: Option<&graphene::Rect>,
) -> Vec<ClusterGeometry> {
    if text_len == 0 {
        return Vec::new();
    }
    let mut raw = Vec::new();
    let mut iter = layout.iter();
    loop {
        let start = usize::try_from(iter.index().max(0))
            .unwrap_or(text_len)
            .min(text_len);
        let (ink, logical) = iter.cluster_extents();
        let rect = pango_rect_union(&ink, &logical);
        if let Some(rect) = clamp_rect(rect, bounds) {
            let rtl = layout
                .index_to_pos(start.min(i32::MAX as usize) as i32)
                .width()
                < 0;
            merge_raw_cluster(&mut raw, start, rect, rtl);
        }
        if !iter.next_cluster() {
            break;
        }
    }
    raw.sort_by_key(|cluster| cluster.0);
    raw.dedup_by(|right, left| {
        if left.0 != right.0 {
            return false;
        }
        left.1 = rect_union(&left.1, &right.1);
        left.2 |= right.2;
        true
    });

    let mut clusters = Vec::with_capacity(raw.len());
    for (index, (start, rect, rtl)) in raw.iter().enumerate() {
        let end = raw
            .get(index + 1)
            .map(|next| next.0)
            .unwrap_or(text_len)
            .max(*start)
            .min(text_len);
        if *start < end {
            clusters.push(ClusterGeometry {
                byte_range: *start..end,
                rect: *rect,
                rtl: *rtl,
            });
        }
    }
    clusters
}

fn merge_raw_cluster(
    clusters: &mut Vec<(usize, graphene::Rect, bool)>, start: usize, rect: graphene::Rect,
    rtl: bool,
) {
    if let Some(existing) = clusters.iter_mut().find(|cluster| cluster.0 == start) {
        existing.1 = rect_union(&existing.1, &rect);
        existing.2 |= rtl;
    } else {
        clusters.push((start, rect, rtl));
    }
}

fn segment_mask(
    range: &Range<usize>, clusters: &[ClusterGeometry], bounds: Option<&graphene::Rect>,
) -> SegmentMask {
    if range.start >= range.end {
        return SegmentMask::Empty;
    }
    if clusters.is_empty() {
        return bounds
            .copied()
            .map(SegmentMask::Bounds)
            .unwrap_or(SegmentMask::Empty);
    }
    let start = clusters
        .iter()
        .position(|cluster| ranges_overlap(&cluster.byte_range, range));
    let end = clusters
        .iter()
        .rposition(|cluster| ranges_overlap(&cluster.byte_range, range))
        .map(|index| index + 1);
    match (start, end) {
        (Some(start), Some(end)) if start < end => SegmentMask::Clusters(start..end),
        _ => SegmentMask::Empty,
    }
}

fn visit_cluster_parts(
    clusters: &[ClusterGeometry], progress: f32, visitor: &mut impl FnMut(RevealPart),
) {
    if progress <= 0.0 || clusters.is_empty() {
        return;
    }
    let total: f32 = clusters.iter().map(cluster_weight).sum();
    if !total.is_finite() || total <= 0.0 {
        return;
    }
    let mut remaining = total * progress;
    for cluster in clusters {
        let weight = cluster_weight(cluster);
        if remaining >= weight {
            visitor(RevealPart {
                rect: cluster.rect,
                feather: false,
                rtl: cluster.rtl,
                fraction: 1.0,
            });
            remaining -= weight;
        } else if remaining > 0.0 {
            if let Some(rect) = partial_rect(cluster, remaining / weight) {
                visitor(RevealPart {
                    rect,
                    feather: true,
                    rtl: cluster.rtl,
                    fraction: (remaining / weight).clamp(0.0, 1.0),
                });
            }
            break;
        } else {
            break;
        }
    }
}

fn visit_clusters(
    clusters: &[ClusterGeometry], progress: f32, visitor: &mut impl FnMut(&graphene::Rect),
) {
    if progress <= 0.0 || clusters.is_empty() {
        return;
    }
    if progress >= 1.0 {
        for cluster in clusters {
            visitor(&cluster.rect);
        }
        return;
    }

    let total: f32 = clusters.iter().map(cluster_weight).sum();
    if !total.is_finite() || total <= 0.0 {
        visit_cluster_step(clusters, progress, visitor);
        return;
    }
    let mut remaining = total * progress;
    for cluster in clusters {
        let weight = cluster_weight(cluster);
        if remaining >= weight {
            visitor(&cluster.rect);
            remaining -= weight;
        } else if remaining > 0.0 {
            if let Some(rect) = partial_rect(cluster, remaining / weight) {
                visitor(&rect);
            }
            break;
        } else {
            break;
        }
    }
}

fn visit_cluster_step(
    clusters: &[ClusterGeometry], progress: f32, visitor: &mut impl FnMut(&graphene::Rect),
) {
    let count = ((clusters.len() as f32 * progress).ceil() as usize).min(clusters.len());
    for cluster in &clusters[..count] {
        visitor(&cluster.rect);
    }
}

fn partial_rect(cluster: &ClusterGeometry, fraction: f32) -> Option<graphene::Rect> {
    let rect = cluster.rect;
    let fraction = finite_progress(fraction);
    let width = rect.width() * fraction;
    if width <= 0.0 || rect.height() <= 0.0 {
        return None;
    }
    let x = if cluster.rtl {
        rect.x() + rect.width() - width
    } else {
        rect.x()
    };
    Some(graphene::Rect::new(x, rect.y(), width, rect.height()))
}

fn cluster_weight(cluster: &ClusterGeometry) -> f32 {
    cluster.rect.width().abs().max(1.0)
}

fn layout_bounds(layout: &pango::Layout) -> Option<graphene::Rect> {
    let (ink, logical) = layout.pixel_extents();
    let ink = pixel_rect(&ink);
    let logical = pixel_rect(&logical);
    match (ink, logical) {
        (Some(left), Some(right)) => Some(rect_union(&left, &right)),
        (Some(rect), None) | (None, Some(rect)) => Some(rect),
        (None, None) => None,
    }
}

fn pango_rect_union(ink: &pango::Rectangle, logical: &pango::Rectangle) -> graphene::Rect {
    let ink = unit_rect(ink);
    let logical = unit_rect(logical);
    rect_union(&ink, &logical)
}

fn unit_rect(rect: &pango::Rectangle) -> graphene::Rect {
    let scale = pango::SCALE as f32;
    graphene::Rect::new(
        rect.x() as f32 / scale,
        rect.y() as f32 / scale,
        rect.width().abs() as f32 / scale,
        rect.height().abs() as f32 / scale,
    )
}

fn pixel_rect(rect: &pango::Rectangle) -> Option<graphene::Rect> {
    let width = rect.width().abs() as f32;
    let height = rect.height().abs() as f32;
    (width > 0.0 && height > 0.0)
        .then(|| graphene::Rect::new(rect.x() as f32, rect.y() as f32, width, height))
}

fn clamp_rect(rect: graphene::Rect, bounds: Option<&graphene::Rect>) -> Option<graphene::Rect> {
    let Some(bounds) = bounds else {
        return valid_rect(rect).then_some(rect);
    };
    let x1 = rect.x().max(bounds.x());
    let y1 = rect.y().max(bounds.y());
    let x2 = (rect.x() + rect.width()).min(bounds.x() + bounds.width());
    let y2 = (rect.y() + rect.height()).min(bounds.y() + bounds.height());
    let clamped = graphene::Rect::new(x1, y1, (x2 - x1).max(0.0), (y2 - y1).max(0.0));
    valid_rect(clamped).then_some(clamped)
}

fn rect_union(left: &graphene::Rect, right: &graphene::Rect) -> graphene::Rect {
    let x1 = left.x().min(right.x());
    let y1 = left.y().min(right.y());
    let x2 = (left.x() + left.width()).max(right.x() + right.width());
    let y2 = (left.y() + left.height()).max(right.y() + right.height());
    graphene::Rect::new(x1, y1, x2 - x1, y2 - y1)
}

fn ranges_overlap(left: &Range<usize>, right: &Range<usize>) -> bool {
    left.start < right.end && right.start < left.end
}

fn valid_rect(rect: graphene::Rect) -> bool {
    rect.x().is_finite()
        && rect.y().is_finite()
        && rect.width().is_finite()
        && rect.height().is_finite()
        && rect.width() > 0.0
        && rect.height() > 0.0
}

fn finite_progress(progress: f32) -> f32 {
    if progress.is_finite() {
        progress.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradient_is_applied_only_to_the_moving_cluster_frontier() {
        let geometry = HighlightGeometry {
            clusters: vec![
                ClusterGeometry {
                    byte_range: 0..1,
                    rect: graphene::Rect::new(0.0, 0.0, 10.0, 20.0),
                    rtl: false,
                },
                ClusterGeometry {
                    byte_range: 1..2,
                    rect: graphene::Rect::new(10.0, 0.0, 10.0, 20.0),
                    rtl: false,
                },
                ClusterGeometry {
                    byte_range: 2..3,
                    rect: graphene::Rect::new(20.0, 0.0, 10.0, 20.0),
                    rtl: false,
                },
            ],
            segments: vec![SegmentMask::Clusters(0..3)],
            bounds: Some(graphene::Rect::new(0.0, 0.0, 30.0, 20.0)),
        };
        let mut parts = Vec::new();
        geometry.visit_gradient_reveal(0, 0.5, |part| parts.push(part));

        assert_eq!(parts.len(), 2);
        assert!(!parts[0].feather);
        assert!(parts[1].feather);
        assert_eq!(parts[1].rect.width(), 5.0);
    }
}
