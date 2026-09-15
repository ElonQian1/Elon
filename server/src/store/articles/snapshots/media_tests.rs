use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::json;
use std::io::Cursor;
use tests::{document, fixture};

fn png() -> Vec<u8> {
    let mut output = Cursor::new(Vec::new());
    image::RgbImage::from_pixel(3, 2, image::Rgb([31, 72, 150]))
        .write_to(&mut output, image::ImageFormat::Png)
        .unwrap();
    output.into_inner()
}
fn with_image(asset: &str) -> SnapshotDocument {
    let mut value = serde_json::to_value(document()).unwrap();
    value["cover_asset_id"] = json!(asset);
    value["messages"][1]["parts"] = json!([{"type":"image","label":"Selected image",
        "asset_id":asset,"media_type":"image/png","caption":"Preserved caption"}]);
    serde_json::from_value(value).unwrap()
}

#[test]
fn images_strip_original_metadata_and_require_snapshot_group_permission() {
    let store = fixture();
    let mut input = png();
    input.extend_from_slice(b"provider_cookie=SyntheticPrivateBytes123456789");
    let asset = store
        .upload_ai_snapshot_asset("author", "g1", &STANDARD.encode(input))
        .unwrap();
    assert_eq!(asset.mime_type, "image/png");
    let doc = with_image(&asset.asset_id);
    let created = store
        .create_ai_snapshot("author", "g1", "operation-1", doc.clone())
        .unwrap();
    let (_, bytes) = store
        .read_ai_snapshot_asset("reader", "g1", &created.snapshot_id, &asset.asset_id)
        .unwrap();
    assert!(!bytes.windows(15).any(|part| part == b"provider_cookie"));
    assert_eq!(
        image::load_from_memory(&bytes).unwrap().to_rgb8(),
        image::load_from_memory(&png()).unwrap().to_rgb8()
    );
    assert!(store
        .read_ai_snapshot_asset("other", "g1", &created.snapshot_id, &asset.asset_id)
        .is_err());
    assert!(store
        .read_ai_snapshot_asset("author", "g2", &created.snapshot_id, &asset.asset_id)
        .is_err());
    assert!(store
        .create_ai_snapshot("author", "g2", "operation-1", doc.clone())
        .is_err());
    assert!(store
        .create_ai_snapshot("reader", "g1", "operation-1", doc)
        .is_err());
    let read = store
        .read_ai_snapshot("reader", "g1", &created.snapshot_id)
        .unwrap();
    assert_eq!(
        read.document.cover_asset_id.as_deref(),
        Some(asset.asset_id.as_str())
    );
    let card: SnapshotCard =
        serde_json::from_str(created.message.content.strip_prefix(CARD_PREFIX).unwrap()).unwrap();
    assert_eq!(
        card.cover_asset_id.as_deref(),
        Some(asset.asset_id.as_str())
    );
    assert_eq!(
        read.document.messages[1].parts[0].caption.as_deref(),
        Some("Preserved caption")
    );
    store
        .revoke_ai_snapshot("author", "g1", &created.snapshot_id)
        .unwrap();
    assert!(store
        .read_ai_snapshot_asset("reader", "g1", &created.snapshot_id, &asset.asset_id)
        .is_err());
}

#[test]
fn same_owner_upload_deduplicates_but_unselected_and_unscoped_media_are_denied() {
    let store = fixture();
    let encoded = STANDARD.encode(png());
    assert!(store
        .upload_ai_snapshot_asset("other", "g1", &encoded)
        .is_err());
    let ordinary = store.upload_article_media("author", &encoded).unwrap();
    assert!(store
        .create_ai_snapshot("author", "g1", "operation-1", with_image(&ordinary.id))
        .is_err());
    let first = store
        .upload_ai_snapshot_asset("author", "g1", &encoded)
        .unwrap();
    let next = store
        .upload_ai_snapshot_asset("author", "g1", &encoded)
        .unwrap();
    assert_eq!(first.asset_id, next.asset_id);
    let created = store
        .create_ai_snapshot("author", "g1", "operation-1", document())
        .unwrap();
    assert!(store
        .read_ai_snapshot_asset("reader", "g1", &created.snapshot_id, &first.asset_id)
        .is_err());
    let mut invalid = document();
    invalid.cover_asset_id = Some(first.asset_id);
    assert!(invalid.validate().is_err());
}

#[test]
fn departed_members_and_recalled_messages_lose_asset_access() {
    let store = fixture();
    let asset = store
        .upload_ai_snapshot_asset("author", "g1", &STANDARD.encode(png()))
        .unwrap();
    let created = store
        .create_ai_snapshot("author", "g1", "operation-1", with_image(&asset.asset_id))
        .unwrap();
    store
        .conn()
        .unwrap()
        .execute(
            "DELETE FROM friend_group_members WHERE user_id='reader'",
            [],
        )
        .unwrap();
    assert!(store
        .read_ai_snapshot_asset("reader", "g1", &created.snapshot_id, &asset.asset_id)
        .is_err());
    store
        .conn()
        .unwrap()
        .execute("UPDATE friend_group_messages SET recalled_at='now'", [])
        .unwrap();
    assert!(store
        .read_ai_snapshot_asset("author", "g1", &created.snapshot_id, &asset.asset_id)
        .is_err());
}

#[test]
fn full_resolution_images_above_article_limit_do_not_get_downscaled() {
    let store = fixture();
    let mut seed = 1u32;
    let image = image::RgbImage::from_fn(512, 512, |_, _| {
        let mut pixel = [0; 3];
        for channel in &mut pixel {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            *channel = (seed >> 24) as u8;
        }
        image::Rgb(pixel)
    });
    let mut output = Cursor::new(Vec::new());
    image
        .write_to(&mut output, image::ImageFormat::Png)
        .unwrap();
    assert!(output.get_ref().len() > 512 * 1024);
    let encoded = STANDARD.encode(output.into_inner());
    assert!(store.upload_article_media("author", &encoded).is_err());
    let asset = store
        .upload_ai_snapshot_asset("author", "g1", &encoded)
        .unwrap();
    let created = store
        .create_ai_snapshot("author", "g1", "operation-1", with_image(&asset.asset_id))
        .unwrap();
    let (_, bytes) = store
        .read_ai_snapshot_asset("reader", "g1", &created.snapshot_id, &asset.asset_id)
        .unwrap();
    assert_eq!(image::load_from_memory(&bytes).unwrap().to_rgb8(), image);
}

#[test]
fn sanitizer_preserves_sixteen_bit_pixels_and_rejects_non_images() {
    let store = fixture();
    for encoded in ["not-base64", "PHNjcmlwdD4=", ""] {
        assert!(store
            .upload_ai_snapshot_asset("author", "g1", encoded)
            .is_err());
    }
    let original = image::ImageBuffer::from_pixel(2, 2, image::Rgb([1025u16, 30001, 65001]));
    let mut output = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb16(original.clone())
        .write_to(&mut output, image::ImageFormat::Png)
        .unwrap();
    let asset = store
        .upload_ai_snapshot_asset("author", "g1", &STANDARD.encode(output.into_inner()))
        .unwrap();
    let created = store
        .create_ai_snapshot("author", "g1", "operation-1", with_image(&asset.asset_id))
        .unwrap();
    let (_, bytes) = store
        .read_ai_snapshot_asset("reader", "g1", &created.snapshot_id, &asset.asset_id)
        .unwrap();
    assert_eq!(
        image::load_from_memory(&bytes).unwrap().to_rgb16(),
        original
    );
}
