use image::{DynamicImage, Rgba, RgbaImage};

use super::visual_diff::{
    compare_dynamic_images, compare_dynamic_images_with_projection, AdaptiveIconMask,
    AdaptiveIconMaskShape, PixelRect, VisualMask, VisualScoreProfile,
};

fn solid(width: u32, height: u32, color: [u8; 4]) -> DynamicImage {
    DynamicImage::ImageRgba8(RgbaImage::from_pixel(width, height, Rgba(color)))
}

#[test]
fn identical_images_pass_every_hard_gate() {
    let image = solid(80, 24, [20, 40, 60, 255]);
    let result = compare_dynamic_images(&image, &image, None, None).unwrap();
    assert_eq!(result.visual_loss, 0.0);
    assert!(result.score_report.target_gate.passed);
    assert_eq!(result.score_report.comparison_width, 80);
    assert_eq!(result.score_report.comparison_height, 24);
}

#[test]
fn transparent_target_compares_only_effective_alpha_pixels() {
    let mut target = RgbaImage::from_pixel(40, 40, Rgba([255, 255, 255, 0]));
    let mut current = RgbaImage::from_pixel(40, 40, Rgba([16, 24, 32, 255]));
    for y in 10..30 {
        for x in 10..30 {
            target.put_pixel(x, y, Rgba([40, 120, 220, 255]));
            current.put_pixel(x, y, Rgba([40, 120, 220, 255]));
        }
    }

    let result = compare_dynamic_images(
        &DynamicImage::ImageRgba8(target),
        &DynamicImage::ImageRgba8(current),
        None,
        None,
    )
    .unwrap();

    assert_eq!(result.mean_absolute_color_error, 0.0);
    assert_eq!(result.edge_error, 0.0);
    assert_eq!(result.score_report.coverage.eligible_pixels, 400);
    assert_eq!(result.score_report.coverage.compared_pixels, 400);
    assert_eq!(result.score_report.coverage.ratio, 1.0);
    assert!(result.score_report.target_gate.passed);
}

#[test]
fn transparent_target_keeps_coverage_failure_for_missing_effective_pixels() {
    let mut target = RgbaImage::from_pixel(20, 20, Rgba([0, 0, 0, 0]));
    for y in 5..15 {
        for x in 5..15 {
            target.put_pixel(x, y, Rgba([80, 160, 240, 255]));
        }
    }
    let current = RgbaImage::from_pixel(20, 20, Rgba([0, 0, 0, 0]));

    let result = compare_dynamic_images(
        &DynamicImage::ImageRgba8(target),
        &DynamicImage::ImageRgba8(current),
        None,
        None,
    )
    .unwrap();

    assert_eq!(result.score_report.coverage.eligible_pixels, 100);
    assert_eq!(result.score_report.coverage.compared_pixels, 0);
    assert_eq!(result.score_report.coverage.ratio, 0.0);
    assert!(!result.score_report.target_gate.coverage_passed);
    assert!(!result.score_report.target_gate.passed);
}

#[test]
fn antialiased_target_pixels_accept_a_valid_background_composite() {
    let background = [24_u8, 32_u8, 40_u8];
    let mut target = RgbaImage::from_pixel(20, 20, Rgba([0, 0, 0, 0]));
    let mut current = RgbaImage::from_pixel(
        20,
        20,
        Rgba([background[0], background[1], background[2], 255]),
    );
    for y in 5..15 {
        for x in 5..15 {
            let alpha = if matches!(x, 5 | 14) || matches!(y, 5 | 14) {
                128
            } else {
                255
            };
            let foreground = [80_u8, 160_u8, 240_u8];
            target.put_pixel(
                x,
                y,
                Rgba([foreground[0], foreground[1], foreground[2], alpha]),
            );
            let blend = |channel: usize| {
                let value = u16::from(foreground[channel]) * u16::from(alpha)
                    + u16::from(background[channel]) * u16::from(255 - alpha);
                ((value + 127) / 255) as u8
            };
            current.put_pixel(x, y, Rgba([blend(0), blend(1), blend(2), 255]));
        }
    }

    let result = compare_dynamic_images(
        &DynamicImage::ImageRgba8(target),
        &DynamicImage::ImageRgba8(current),
        None,
        None,
    )
    .unwrap();

    assert_eq!(result.visual_loss, 0.0);
    assert_eq!(result.score_report.coverage.eligible_pixels, 100);
    assert_eq!(result.score_report.coverage.ratio, 1.0);
    assert!(result.score_report.target_gate.passed);
}

#[test]
fn letterbox_preserves_aspect_ratio_instead_of_stretching_to_a_square() {
    let target = solid(200, 40, [40, 80, 120, 255]);
    let current = solid(100, 40, [40, 80, 120, 255]);
    let result = compare_dynamic_images_with_projection(
        &target,
        &current,
        None,
        Some(PixelRect {
            left: 0,
            top: 0,
            right: 100,
            bottom: 40,
        }),
        Some(PixelRect {
            left: 0,
            top: 0,
            right: 200,
            bottom: 40,
        }),
        &VisualMask::default(),
        VisualScoreProfile::default(),
    )
    .unwrap();
    assert_eq!(result.score_report.comparison_width, 200);
    assert_eq!(result.score_report.comparison_height, 40);
    assert!(result.score_report.geometry.aspect_error_ratio > 0.4);
    assert!(!result.score_report.target_gate.geometry_passed);
}

#[test]
fn projected_position_is_checked_independently_from_design_crop_coordinates() {
    let target = solid(60, 20, [80, 100, 120, 255]);
    let current_frame = solid(240, 150, [80, 100, 120, 255]);
    let result = compare_dynamic_images_with_projection(
        &target,
        &current_frame,
        None,
        Some(PixelRect {
            left: 120,
            top: 80,
            right: 180,
            bottom: 100,
        }),
        Some(PixelRect {
            left: 100,
            top: 70,
            right: 160,
            bottom: 90,
        }),
        &VisualMask::default(),
        VisualScoreProfile::default(),
    )
    .unwrap();
    assert_eq!(result.mean_absolute_color_error, 0.0);
    assert_eq!(result.score_report.position.left_error_px, 20.0);
    assert_eq!(result.score_report.position.top_error_px, 10.0);
    assert!(!result.score_report.target_gate.position_passed);
    assert!(!result.score_report.target_gate.passed);
}

#[test]
fn excluded_region_does_not_pollute_color_score() {
    let target = solid(20, 10, [0, 0, 0, 255]);
    let mut current = RgbaImage::from_pixel(20, 10, Rgba([0, 0, 0, 255]));
    for y in 0..10 {
        for x in 10..20 {
            current.put_pixel(x, y, Rgba([255, 255, 255, 255]));
        }
    }
    let result = compare_dynamic_images_with_projection(
        &target,
        &DynamicImage::ImageRgba8(current),
        None,
        None,
        None,
        &VisualMask {
            exclude_rects: vec![PixelRect {
                left: 10,
                top: 0,
                right: 20,
                bottom: 10,
            }],
            adaptive_icon_mask: None,
        },
        VisualScoreProfile::default(),
    )
    .unwrap();
    assert_eq!(result.score_report.color.mean_absolute_error, 0.0);
    assert_eq!(result.score_report.coverage.compared_pixels, 100);
    assert!(result.score_report.target_gate.passed);
}

#[test]
fn adaptive_circle_mask_ignores_launcher_corner_pixels() {
    let target = solid(40, 40, [0, 0, 0, 255]);
    let mut current = RgbaImage::from_pixel(40, 40, Rgba([0, 0, 0, 255]));
    for y in 0..4 {
        for x in 0..4 {
            current.put_pixel(x, y, Rgba([255, 255, 255, 255]));
        }
    }
    let result = compare_dynamic_images_with_projection(
        &target,
        &DynamicImage::ImageRgba8(current),
        None,
        None,
        None,
        &VisualMask {
            exclude_rects: Vec::new(),
            adaptive_icon_mask: Some(AdaptiveIconMask {
                shape: AdaptiveIconMaskShape::Circle,
                safe_zone_inset_fraction: 0.0,
            }),
        },
        VisualScoreProfile::default(),
    )
    .unwrap();
    assert_eq!(result.score_report.color.mean_absolute_error, 0.0);
    assert!(result.score_report.coverage.compared_pixels < 1_600);
}

#[test]
fn low_aggregate_score_cannot_override_a_failed_geometry_gate() {
    let target = solid(100, 20, [25, 25, 25, 255]);
    let current = solid(99, 20, [25, 25, 25, 255]);
    let result = compare_dynamic_images_with_projection(
        &target,
        &current,
        None,
        Some(PixelRect {
            left: 0,
            top: 0,
            right: 99,
            bottom: 20,
        }),
        Some(PixelRect {
            left: 0,
            top: 0,
            right: 100,
            bottom: 20,
        }),
        &VisualMask::default(),
        VisualScoreProfile {
            max_size_error_ratio: 0.001,
            ..VisualScoreProfile::default()
        },
    )
    .unwrap();
    assert!(result.visual_loss < 0.05);
    assert!(!result.score_report.target_gate.geometry_passed);
    assert!(!result.score_report.target_gate.passed);
}

#[test]
fn color_metric_reports_perceptual_lab_distance() {
    let dark = solid(30, 20, [20, 20, 20, 255]);
    let light = solid(30, 20, [230, 230, 230, 255]);
    let result = compare_dynamic_images(&dark, &light, None, None).unwrap();
    assert!(result.score_report.color.mean_delta_e > 70.0);
    assert_eq!(
        result.score_report.color.mean_delta_e,
        result.score_report.color.p95_delta_e
    );
    assert!(!result.score_report.target_gate.color_passed);
}

#[test]
fn grayscale_and_same_luminance_subpixel_text_edges_are_normalized() {
    let grayscale = solid(30, 20, [128, 128, 128, 255]);
    let subpixel = solid(30, 20, [145, 123, 132, 255]);
    let result = compare_dynamic_images(&grayscale, &subpixel, None, None).unwrap();
    assert!(result.score_report.color.mean_absolute_error < 0.01);
    assert!(result.score_report.color.mean_delta_e < 3.0);
    assert!(result.score_report.target_gate.color_passed);
}

#[test]
fn saturated_real_color_difference_is_not_treated_as_text_antialiasing() {
    let grayscale = solid(30, 20, [128, 128, 128, 255]);
    let saturated = solid(30, 20, [240, 20, 20, 255]);
    let result = compare_dynamic_images(&grayscale, &saturated, None, None).unwrap();
    assert!(result.score_report.color.mean_absolute_error > 0.2);
    assert!(!result.score_report.target_gate.color_passed);
}
