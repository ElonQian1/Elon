use std::fs;
use std::io::Cursor;

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};

use super::broker::LiveUiBroker;
use super::ui_ir::{
    bind_ui_ir, persist_target_design, ui_ir_path, BindUiIrRequest, TargetDesignUpload,
};

#[test]
fn ui_ir_session_id_cannot_escape_artifact_root() {
    assert!(ui_ir_path(None, "../escape").is_err());
    assert!(ui_ir_path(None, "live_safe-01").is_ok());
}

#[tokio::test]
async fn persists_target_design_and_compact_ir() {
    let root =
        std::env::temp_dir().join(format!("elon-ui-ir-test-{}", uuid::Uuid::new_v4().simple()));
    fs::create_dir_all(&root).unwrap();
    let broker = LiveUiBroker::new();
    let session = broker
        .create_session(
            "device-1".to_string(),
            "com.example.debug".to_string(),
            Some(root.display().to_string()),
            38917,
        )
        .await;
    let mut png = Vec::new();
    DynamicImage::ImageRgba8(RgbaImage::from_pixel(2, 3, Rgba([1, 2, 3, 255])))
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .unwrap();
    let target = persist_target_design(
        &broker,
        &session.id,
        TargetDesignUpload {
            name: "设计图.png".to_string(),
            data_url: format!("data:image/png;base64,{}", B64.encode(&png)),
            figma_url: Some("https://www.figma.com/design/demo".to_string()),
        },
    )
    .await
    .unwrap();
    assert_eq!((target.width, target.height), (2, 3));
    let document = bind_ui_ir(
        &broker,
        &session.id,
        BindUiIrRequest {
            target_design: Some(target),
            clear_target_design: false,
            ..BindUiIrRequest::default()
        },
    )
    .await
    .unwrap();
    assert!(document.summary.has_target_design);
    assert_eq!(document.summary.node_count, 0);
    assert!(ui_ir_path(Some(root.to_str().unwrap()), &session.id)
        .unwrap()
        .is_file());
    fs::remove_dir_all(root).unwrap();
}
