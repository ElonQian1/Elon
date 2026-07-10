use image::{DynamicImage, Rgba, RgbaImage};

use super::visual_diff::{compare_dynamic_images, PixelRect};

#[test]
fn identical_images_have_zero_visual_loss() {
    let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(10, 12, Rgba([20, 40, 60, 255])));
    let result = compare_dynamic_images(&image, &image, None, None).unwrap();
    assert_eq!(result.visual_loss, 0.0);
}

#[test]
fn visual_loss_detects_color_and_geometry_changes() {
    let left = DynamicImage::ImageRgba8(RgbaImage::from_pixel(20, 20, Rgba([0, 0, 0, 255])));
    let right = DynamicImage::ImageRgba8(RgbaImage::from_pixel(10, 12, Rgba([255, 255, 255, 255])));
    let result = compare_dynamic_images(
        &left,
        &right,
        Some(PixelRect {
            left: 0,
            top: 0,
            right: 18,
            bottom: 16,
        }),
        None,
    )
    .unwrap();
    assert!(result.mean_absolute_color_error > 0.99);
    assert!(result.geometry_error > 0.0);
    assert!(result.visual_loss > 0.5);
}
