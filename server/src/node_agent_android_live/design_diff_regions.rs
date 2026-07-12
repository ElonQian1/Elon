use std::collections::VecDeque;

use anyhow::{bail, Context, Result};
use image::{imageops::FilterType, DynamicImage, GenericImageView};
use serde::{Deserialize, Serialize};

use super::broker::LiveUiBroker;
use super::protocol::LiveUiNode;
use super::ui_ir::load_or_build_ui_ir;

const CELL_SIZE: u32 = 4;
const MAX_REGIONS: usize = 48;
const MAX_CANDIDATES: usize = 6;

#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct DesignDiffRegionRequest {
    pub(crate) channel_threshold: u8,
    pub(crate) minimum_region_area: u32,
    pub(crate) merge_gap_px: u32,
    pub(crate) maximum_regions: usize,
}

impl Default for DesignDiffRegionRequest {
    fn default() -> Self {
        Self {
            channel_threshold: 18,
            minimum_region_area: 96,
            merge_gap_px: 12,
            maximum_regions: 24,
        }
    }
}

impl DesignDiffRegionRequest {
    fn validate(&self) -> Result<()> {
        if self.channel_threshold == 0 {
            bail!("channelThreshold 必须大于 0");
        }
        if self.minimum_region_area == 0 || self.minimum_region_area > 4_000_000 {
            bail!("minimumRegionArea 必须在 1..4000000");
        }
        if self.merge_gap_px > 256 {
            bail!("mergeGapPx 不得超过 256");
        }
        if self.maximum_regions == 0 || self.maximum_regions > MAX_REGIONS {
            bail!("maximumRegions 必须在 1..{MAX_REGIONS}");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesignPixelRect {
    pub(crate) left: i32,
    pub(crate) top: i32,
    pub(crate) right: i32,
    pub(crate) bottom: i32,
}

impl DesignPixelRect {
    fn width(self) -> i32 {
        (self.right - self.left).max(0)
    }

    fn height(self) -> i32 {
        (self.bottom - self.top).max(0)
    }

    fn area(self) -> f64 {
        f64::from(self.width()) * f64::from(self.height())
    }

    fn union(self, other: Self) -> Self {
        Self {
            left: self.left.min(other.left),
            top: self.top.min(other.top),
            right: self.right.max(other.right),
            bottom: self.bottom.max(other.bottom),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesignNodeCandidate {
    pub(crate) runtime_node_id: String,
    pub(crate) definition_id: String,
    pub(crate) instance_key: Option<String>,
    pub(crate) kind: String,
    pub(crate) text: Option<String>,
    pub(crate) target_bounds: DesignPixelRect,
    pub(crate) score: f64,
    pub(crate) editable_property_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesignDiffRegion {
    pub(crate) id: String,
    pub(crate) target_rect: DesignPixelRect,
    pub(crate) changed_pixels: u32,
    pub(crate) candidates: Vec<DesignNodeCandidate>,
    pub(crate) recommended_runtime_node_id: Option<String>,
    pub(crate) confidence: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesignDiffRegionAnalysis {
    pub(crate) baseline_width: u32,
    pub(crate) baseline_height: u32,
    pub(crate) target_width: u32,
    pub(crate) target_height: u32,
    pub(crate) scale_x: f64,
    pub(crate) scale_y: f64,
    pub(crate) changed_pixel_ratio: f64,
    pub(crate) regions: Vec<DesignDiffRegion>,
}

#[derive(Debug, Clone, Copy)]
struct RawRegion {
    rect: DesignPixelRect,
    changed_pixels: u32,
}

pub(crate) async fn analyze_session_design_diff(
    broker: &LiveUiBroker,
    session_id: &str,
    request: DesignDiffRegionRequest,
) -> Result<DesignDiffRegionAnalysis> {
    request.validate()?;
    let ir = load_or_build_ui_ir(broker, session_id).await?;
    let snapshot = ir.snapshot.context("尚未绑定原始真机截图")?;
    let target = ir.target_design.context("尚未绑定目标设计图")?;
    let baseline = image::open(&snapshot.screenshot_path)
        .with_context(|| format!("无法读取原始真机截图: {}", snapshot.screenshot_path))?;
    let target_image = image::open(&target.path)
        .with_context(|| format!("无法读取目标设计图: {}", target.path))?;
    analyze_design_diff_images(&baseline, &target_image, &ir.nodes, &request)
}

pub(super) fn analyze_design_diff_images(
    baseline: &DynamicImage,
    target: &DynamicImage,
    nodes: &[LiveUiNode],
    request: &DesignDiffRegionRequest,
) -> Result<DesignDiffRegionAnalysis> {
    request.validate()?;
    let (baseline_width, baseline_height) = baseline.dimensions();
    let (target_width, target_height) = target.dimensions();
    if baseline_width == 0 || baseline_height == 0 || target_width == 0 || target_height == 0 {
        bail!("原始截图和目标设计图必须是非空图片");
    }
    let normalized = if (baseline_width, baseline_height) == (target_width, target_height) {
        baseline.to_rgba8()
    } else {
        baseline
            .resize_exact(target_width, target_height, FilterType::Triangle)
            .to_rgba8()
    };
    let target = target.to_rgba8();
    let (raw_regions, changed_pixels) = changed_regions(&normalized, &target, request);
    let scale_x = f64::from(target_width) / f64::from(baseline_width);
    let scale_y = f64::from(target_height) / f64::from(baseline_height);
    let mut regions = raw_regions
        .into_iter()
        .map(|raw| region_with_candidates(raw, nodes, scale_x, scale_y))
        .collect::<Vec<_>>();
    regions.sort_by(|left, right| {
        right
            .changed_pixels
            .cmp(&left.changed_pixels)
            .then_with(|| left.target_rect.top.cmp(&right.target_rect.top))
            .then_with(|| left.target_rect.left.cmp(&right.target_rect.left))
    });
    regions.truncate(request.maximum_regions);
    for (index, region) in regions.iter_mut().enumerate() {
        region.id = format!("diff_{:03}", index + 1);
    }
    let total = f64::from(target_width) * f64::from(target_height);
    Ok(DesignDiffRegionAnalysis {
        baseline_width,
        baseline_height,
        target_width,
        target_height,
        scale_x,
        scale_y,
        changed_pixel_ratio: f64::from(changed_pixels) / total.max(1.0),
        regions,
    })
}

fn changed_regions(
    baseline: &image::RgbaImage,
    target: &image::RgbaImage,
    request: &DesignDiffRegionRequest,
) -> (Vec<RawRegion>, u32) {
    let width = target.width();
    let height = target.height();
    let grid_width = width.div_ceil(CELL_SIZE);
    let grid_height = height.div_ceil(CELL_SIZE);
    let mut counts = vec![0u16; (grid_width * grid_height) as usize];
    let mut changed_pixels = 0u32;
    for y in 0..height {
        for x in 0..width {
            let before = baseline.get_pixel(x, y).0;
            let after = target.get_pixel(x, y).0;
            let delta = before
                .iter()
                .zip(after.iter())
                .map(|(left, right)| left.abs_diff(*right))
                .max()
                .unwrap_or_default();
            if delta >= request.channel_threshold {
                let index = ((y / CELL_SIZE) * grid_width + x / CELL_SIZE) as usize;
                counts[index] = counts[index].saturating_add(1);
                changed_pixels = changed_pixels.saturating_add(1);
            }
        }
    }
    let mut visited = vec![false; counts.len()];
    let mut regions = Vec::new();
    for cell_y in 0..grid_height {
        for cell_x in 0..grid_width {
            let start = (cell_y * grid_width + cell_x) as usize;
            if visited[start] || counts[start] < 2 {
                continue;
            }
            let mut queue = VecDeque::from([(cell_x, cell_y)]);
            visited[start] = true;
            let mut min_x = cell_x;
            let mut min_y = cell_y;
            let mut max_x = cell_x;
            let mut max_y = cell_y;
            let mut region_pixels = 0u32;
            while let Some((x, y)) = queue.pop_front() {
                let index = (y * grid_width + x) as usize;
                region_pixels = region_pixels.saturating_add(u32::from(counts[index]));
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let next_x = x as i32 + dx;
                        let next_y = y as i32 + dy;
                        if next_x < 0
                            || next_y < 0
                            || next_x >= grid_width as i32
                            || next_y >= grid_height as i32
                        {
                            continue;
                        }
                        let next = (next_y as u32 * grid_width + next_x as u32) as usize;
                        if !visited[next] && counts[next] >= 2 {
                            visited[next] = true;
                            queue.push_back((next_x as u32, next_y as u32));
                        }
                    }
                }
            }
            let rect = DesignPixelRect {
                left: (min_x * CELL_SIZE) as i32,
                top: (min_y * CELL_SIZE) as i32,
                right: ((max_x + 1) * CELL_SIZE).min(width) as i32,
                bottom: ((max_y + 1) * CELL_SIZE).min(height) as i32,
            };
            if rect.area() >= f64::from(request.minimum_region_area) {
                regions.push(RawRegion {
                    rect,
                    changed_pixels: region_pixels,
                });
            }
        }
    }
    (merge_regions(regions, request.merge_gap_px), changed_pixels)
}

fn merge_regions(mut source: Vec<RawRegion>, gap: u32) -> Vec<RawRegion> {
    let gap = gap as i32;
    let mut changed = true;
    while changed {
        changed = false;
        'outer: for left in 0..source.len() {
            for right in (left + 1)..source.len() {
                if boxes_near(source[left].rect, source[right].rect, gap) {
                    let merged = RawRegion {
                        rect: source[left].rect.union(source[right].rect),
                        changed_pixels: source[left]
                            .changed_pixels
                            .saturating_add(source[right].changed_pixels),
                    };
                    source[left] = merged;
                    source.remove(right);
                    changed = true;
                    break 'outer;
                }
            }
        }
    }
    source
}

fn boxes_near(left: DesignPixelRect, right: DesignPixelRect, gap: i32) -> bool {
    left.left - gap < right.right
        && left.right + gap > right.left
        && left.top - gap < right.bottom
        && left.bottom + gap > right.top
}

fn region_with_candidates(
    raw: RawRegion,
    nodes: &[LiveUiNode],
    scale_x: f64,
    scale_y: f64,
) -> DesignDiffRegion {
    let mut candidates = nodes
        .iter()
        .filter(|node| node.geometry.visible)
        .filter_map(|node| candidate(node, raw.rect, scale_x, scale_y))
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    candidates.truncate(MAX_CANDIDATES);
    let confidence = candidates.first().map(|value| value.score).unwrap_or(0.0);
    let recommended_runtime_node_id = candidates
        .first()
        .filter(|candidate| candidate.score >= 0.18)
        .map(|candidate| candidate.runtime_node_id.clone());
    DesignDiffRegion {
        id: String::new(),
        target_rect: raw.rect,
        changed_pixels: raw.changed_pixels,
        candidates,
        recommended_runtime_node_id,
        confidence,
    }
}

fn candidate(
    node: &LiveUiNode,
    region: DesignPixelRect,
    scale_x: f64,
    scale_y: f64,
) -> Option<DesignNodeCandidate> {
    let bounds = &node.geometry.bounds_in_display_px;
    let node_rect = DesignPixelRect {
        left: (f64::from(bounds.left) * scale_x).round() as i32,
        top: (f64::from(bounds.top) * scale_y).round() as i32,
        right: (f64::from(bounds.right) * scale_x).round() as i32,
        bottom: (f64::from(bounds.bottom) * scale_y).round() as i32,
    };
    let intersection = DesignPixelRect {
        left: region.left.max(node_rect.left),
        top: region.top.max(node_rect.top),
        right: region.right.min(node_rect.right),
        bottom: region.bottom.min(node_rect.bottom),
    };
    let intersection_area = intersection.area();
    if intersection_area <= 0.0 || region.area() <= 0.0 || node_rect.area() <= 0.0 {
        return None;
    }
    let region_coverage = intersection_area / region.area();
    let node_coverage = intersection_area / node_rect.area();
    let size_similarity = region.area().min(node_rect.area()) / region.area().max(node_rect.area());
    let editable_property_count = node
        .properties
        .values()
        .filter(|property| property.change_level == "LIVE" && property.commit_mode != "READ_ONLY")
        .count();
    let editable_bonus = if editable_property_count > 0 {
        0.1
    } else {
        0.0
    };
    let score =
        (region_coverage * 0.5 + node_coverage * 0.25 + size_similarity * 0.15 + editable_bonus)
            .clamp(0.0, 1.0);
    Some(DesignNodeCandidate {
        runtime_node_id: node.runtime_node_id.clone(),
        definition_id: node.definition_id.clone(),
        instance_key: node.instance_key.clone(),
        kind: node.kind.clone(),
        text: node.text.clone(),
        target_bounds: node_rect,
        score,
        editable_property_count,
    })
}
