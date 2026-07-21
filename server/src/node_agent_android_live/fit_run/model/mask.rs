use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use super::FitRect;
use crate::node_agent_android_live::visual_diff::{PixelRect, VisualMask};

const MAX_MASK_REGIONS: usize = 24;
const MAX_EXCLUDED_PERCENT: i128 = 25;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum FitMaskKind {
    DynamicContent,
    Annotation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitMaskRegion {
    pub(crate) kind: FitMaskKind,
    pub(crate) rect: FitRect,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitVisualMask {
    /// Rectangles are relative to the selected target crop, not screen space.
    #[serde(default)]
    pub(crate) regions: Vec<FitMaskRegion>,
}

impl FitVisualMask {
    pub(crate) fn validate(&self, target_rect: FitRect) -> Result<()> {
        if self.regions.len() > MAX_MASK_REGIONS {
            bail!("visualMask.regions 最多 {MAX_MASK_REGIONS} 个");
        }
        let width = i64::from(target_rect.right) - i64::from(target_rect.left);
        let height = i64::from(target_rect.bottom) - i64::from(target_rect.top);
        let target_area = i128::from(width) * i128::from(height);
        let mut excluded_area = 0_i128;
        for region in &self.regions {
            region.rect.validate("visualMask.regions[].rect")?;
            if region.rect.left < 0
                || region.rect.top < 0
                || i64::from(region.rect.right) > width
                || i64::from(region.rect.bottom) > height
                || region.reason.trim().is_empty()
                || region.reason.chars().count() > 240
            {
                bail!("visualMask 区域必须位于 TARGET_CROP 内并提供简短原因");
            }
            let region_width = i64::from(region.rect.right) - i64::from(region.rect.left);
            let region_height = i64::from(region.rect.bottom) - i64::from(region.rect.top);
            excluded_area += i128::from(region_width) * i128::from(region_height);
        }
        if target_area > 0 && excluded_area * 100 > target_area * MAX_EXCLUDED_PERCENT {
            bail!("visualMask 排除面积不能超过 target crop 的 25%");
        }
        Ok(())
    }

    pub(crate) fn visual_mask(&self) -> VisualMask {
        VisualMask {
            exclude_rects: self
                .regions
                .iter()
                .map(|region| PixelRect {
                    left: region.rect.left,
                    top: region.rect.top,
                    right: region.rect.right,
                    bottom: region.rect.bottom,
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_rejects_unbounded_or_excessive_exclusions() {
        let target = FitRect {
            left: 10,
            top: 20,
            right: 110,
            bottom: 120,
        };
        let mask = FitVisualMask {
            regions: vec![FitMaskRegion {
                kind: FitMaskKind::DynamicContent,
                rect: FitRect {
                    left: 0,
                    top: 0,
                    right: 60,
                    bottom: 60,
                },
                reason: "clock".into(),
            }],
        };
        assert!(mask.validate(target).is_err());
    }

    #[test]
    fn valid_mask_preserves_crop_relative_coordinates() {
        let target = FitRect {
            left: 100,
            top: 200,
            right: 300,
            bottom: 400,
        };
        let mask = FitVisualMask {
            regions: vec![FitMaskRegion {
                kind: FitMaskKind::Annotation,
                rect: FitRect {
                    left: 10,
                    top: 20,
                    right: 50,
                    bottom: 60,
                },
                reason: "reference annotation".into(),
            }],
        };
        mask.validate(target).unwrap();
        let visual = mask.visual_mask();
        assert_eq!(visual.exclude_rects.len(), 1);
        assert_eq!(visual.exclude_rects[0].left, 10);
        assert_eq!(visual.exclude_rects[0].top, 20);
    }
}
