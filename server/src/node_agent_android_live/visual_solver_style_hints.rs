use std::collections::BTreeMap;

use anyhow::{Context, Result};
use image::{GenericImageView, Pixel};
use serde_json::json;

use super::protocol::{LivePatchOperation, LivePropertyValue, LiveUiNode};
use super::visual_diff::PixelRect;

const COLOR_PROPERTIES: [&str; 3] = ["backgroundColor", "contentColor", "borderColor"];

pub(super) fn target_color_operations(
    target_path: &str,
    rect: PixelRect,
    node: &LiveUiNode,
    requested: &[String],
) -> Result<Vec<LivePatchOperation>> {
    let requested = COLOR_PROPERTIES
        .iter()
        .filter(|property| requested.iter().any(|value| value == **property))
        .filter(|property| {
            node.properties.get(**property).is_some_and(|snapshot| {
                snapshot.change_level == "LIVE" && snapshot.commit_mode != "READ_ONLY"
            })
        })
        .copied()
        .collect::<Vec<_>>();
    if requested.is_empty() {
        return Ok(Vec::new());
    }
    let image = image::open(target_path).context("视觉求解无法读取目标设计图颜色")?;
    let bounds = clamped_rect(rect, image.width(), image.height());
    if bounds.right <= bounds.left || bounds.bottom <= bounds.top {
        return Ok(Vec::new());
    }
    let samples = color_samples(&image, bounds);
    let background = dominant_color(&samples.interior).or_else(|| dominant_color(&samples.all));
    let content = background.and_then(|background| contrasting_color(&samples.all, background));
    let border = dominant_color(&samples.border);
    let mut values = BTreeMap::new();
    if let Some(value) = background {
        values.insert("backgroundColor", value);
    }
    if let Some(value) = content {
        values.insert("contentColor", value);
    }
    if let Some(value) = border {
        values.insert("borderColor", value);
    }
    Ok(requested
        .into_iter()
        .filter_map(|property| {
            let color = values.get(property).copied()?;
            let value = argb(color);
            let unchanged = node
                .properties
                .get(property)
                .and_then(|snapshot| snapshot.effective.as_ref())
                .and_then(|current| current.value.as_str())
                .is_some_and(|current| current.eq_ignore_ascii_case(&value));
            (!unchanged).then(|| LivePatchOperation {
                property: property.to_string(),
                value: LivePropertyValue {
                    value_type: "argb".to_string(),
                    value: json!(value),
                },
            })
        })
        .collect())
}

struct ColorSamples {
    all: Vec<[u8; 3]>,
    interior: Vec<[u8; 3]>,
    border: Vec<[u8; 3]>,
}

fn color_samples(image: &image::DynamicImage, rect: PixelRect) -> ColorSamples {
    let width = (rect.right - rect.left).max(1);
    let height = (rect.bottom - rect.top).max(1);
    let inset_x = (width / 8).clamp(1, 12);
    let inset_y = (height / 8).clamp(1, 12);
    let border_width = ((width.min(height)) / 12).clamp(1, 8);
    let mut samples = ColorSamples {
        all: Vec::new(),
        interior: Vec::new(),
        border: Vec::new(),
    };
    for y in rect.top..rect.bottom {
        for x in rect.left..rect.right {
            let rgba = image.get_pixel(x as u32, y as u32).to_rgba().0;
            if rgba[3] < 128 {
                continue;
            }
            let color = [rgba[0], rgba[1], rgba[2]];
            samples.all.push(color);
            if x >= rect.left + inset_x
                && x < rect.right - inset_x
                && y >= rect.top + inset_y
                && y < rect.bottom - inset_y
            {
                samples.interior.push(color);
            }
            if x < rect.left + border_width
                || x >= rect.right - border_width
                || y < rect.top + border_width
                || y >= rect.bottom - border_width
            {
                samples.border.push(color);
            }
        }
    }
    samples
}

fn dominant_color(samples: &[[u8; 3]]) -> Option<[u8; 3]> {
    let histogram = histogram(samples);
    let (bucket, count) = histogram.into_iter().max_by_key(|(_, count)| *count)?;
    if count < samples.len().max(1) / 80 {
        return None;
    }
    Some(bucket_color(bucket))
}

fn contrasting_color(samples: &[[u8; 3]], background: [u8; 3]) -> Option<[u8; 3]> {
    let minimum = samples.len().max(1) / 250;
    histogram(samples)
        .into_iter()
        .filter(|(_, count)| *count >= minimum.max(2))
        .filter_map(|(bucket, count)| {
            let color = bucket_color(bucket);
            let contrast = color_distance(color, background);
            (contrast >= 72.0).then_some((color, contrast * count as f64))
        })
        .max_by(|left, right| left.1.total_cmp(&right.1))
        .map(|(color, _)| color)
}

fn histogram(samples: &[[u8; 3]]) -> BTreeMap<u16, usize> {
    let mut result = BTreeMap::new();
    for [red, green, blue] in samples {
        let bucket =
            (u16::from(*red >> 4) << 8) | (u16::from(*green >> 4) << 4) | u16::from(*blue >> 4);
        *result.entry(bucket).or_insert(0) += 1;
    }
    result
}

fn bucket_color(bucket: u16) -> [u8; 3] {
    [
        (((bucket >> 8) & 0x0f) as u8) * 17,
        (((bucket >> 4) & 0x0f) as u8) * 17,
        ((bucket & 0x0f) as u8) * 17,
    ]
}

fn color_distance(left: [u8; 3], right: [u8; 3]) -> f64 {
    let red = f64::from(left[0]) - f64::from(right[0]);
    let green = f64::from(left[1]) - f64::from(right[1]);
    let blue = f64::from(left[2]) - f64::from(right[2]);
    (red * red + green * green + blue * blue).sqrt()
}

fn argb([red, green, blue]: [u8; 3]) -> String {
    format!("#FF{red:02X}{green:02X}{blue:02X}")
}

fn clamped_rect(rect: PixelRect, width: u32, height: u32) -> PixelRect {
    PixelRect {
        left: rect.left.clamp(0, width as i32),
        top: rect.top.clamp(0, height as i32),
        right: rect.right.clamp(0, width as i32),
        bottom: rect.bottom.clamp(0, height as i32),
    }
}
