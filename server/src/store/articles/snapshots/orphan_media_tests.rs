use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::json;
use std::io::Cursor;
use tests::{document, fixture};

fn upload(store: &Store, group: &str) -> SnapshotAsset {
    let mut png = Cursor::new(Vec::new());
    image::RgbImage::from_pixel(3, 2, image::Rgb([11, 22, 33]))
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    store
        .upload_ai_snapshot_asset("author", group, &STANDARD.encode(png.into_inner()))
        .unwrap()
}
fn has_media(store: &Store, asset: &str) -> bool {
    store
        .conn()
        .unwrap()
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM social_content_media WHERE id=?1)",
            [asset],
            |r| r.get(0),
        )
        .unwrap()
}
fn later() -> i64 {
    chrono::Utc::now().timestamp() + 25 * 60 * 60
}

#[test]
fn orphan_cleanup_waits_at_least_twenty_four_hours_and_is_idempotent() {
    let store = fixture();
    let asset = upload(&store, "g1");
    assert_eq!(
        orphan_media::cleanup(&store, "author", chrono::Utc::now().timestamp() + 23 * 3600)
            .unwrap(),
        0
    );
    assert!(has_media(&store, &asset.asset_id));
    assert_eq!(orphan_media::cleanup(&store, "author", later()).unwrap(), 1);
    assert!(!has_media(&store, &asset.asset_id));
    assert_eq!(orphan_media::cleanup(&store, "author", later()).unwrap(), 0);
}

#[test]
fn failed_grant_insert_rolls_back_media_and_provenance() {
    let store = fixture();
    store
        .conn()
        .unwrap()
        .execute_batch(
            "CREATE TRIGGER deny_grant BEFORE INSERT ON social_snapshot_asset_grants
        BEGIN SELECT RAISE(ABORT,'fixture failure'); END;",
        )
        .unwrap();
    let mut bytes = Cursor::new(Vec::new());
    image::RgbImage::new(2, 2)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    assert!(store
        .upload_ai_snapshot_asset("author", "g1", &STANDARD.encode(bytes.into_inner()))
        .is_err());
    for table in [
        "social_content_media",
        "social_snapshot_orphan_media",
        "social_snapshot_asset_grants",
    ] {
        let count: i64 = store
            .conn()
            .unwrap()
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}

#[test]
fn ordinary_article_uploads_and_article_adoption_never_expire() {
    for article_first in [false, true] {
        let store = fixture();
        let mut bytes = Cursor::new(Vec::new());
        image::RgbImage::new(2, 2)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let encoded = STANDARD.encode(bytes.into_inner());
        if article_first {
            store.upload_article_media("author", &encoded).unwrap();
        }
        let asset = store
            .upload_ai_snapshot_asset("author", "g1", &encoded)
            .unwrap();
        if !article_first {
            let clean: Vec<u8> = store
                .conn()
                .unwrap()
                .query_row(
                    "SELECT bytes FROM social_content_media WHERE id=?1",
                    [&asset.asset_id],
                    |r| r.get(0),
                )
                .unwrap();
            store
                .upload_article_media("author", &STANDARD.encode(clean))
                .unwrap();
        }
        assert_eq!(orphan_media::cleanup(&store, "author", later()).unwrap(), 0);
        assert!(has_media(&store, &asset.asset_id));
    }
}

#[test]
fn drafts_published_and_revoked_revisions_preserve_assets_and_replay() {
    for state in ["draft", "published", "revoked"] {
        let store = fixture();
        let asset = upload(&store, "g1");
        let mut value = serde_json::to_value(document()).unwrap();
        value["messages"][0]["parts"] =
            json!([{"type":"image","label":"Selected","asset_id":asset.asset_id}]);
        let doc: SnapshotDocument = serde_json::from_value(value).unwrap();
        let created = if state == "draft" {
            store
                .create_article(
                    "author",
                    super::super::ArticleDocument {
                        title: "Draft".into(),
                        cover: Some(asset.asset_id.clone()),
                        ..Default::default()
                    },
                )
                .unwrap();
            None
        } else {
            Some(
                store
                    .create_ai_snapshot("author", "g1", "operation-1", doc.clone())
                    .unwrap(),
            )
        };
        if state == "revoked" {
            store
                .revoke_ai_snapshot("author", "g1", &created.as_ref().unwrap().snapshot_id)
                .unwrap();
        }
        assert_eq!(orphan_media::cleanup(&store, "author", later()).unwrap(), 0);
        assert!(has_media(&store, &asset.asset_id));
        if state == "published" {
            assert!(
                store
                    .create_ai_snapshot("author", "g1", "operation-1", doc)
                    .unwrap()
                    .replayed
            );
        }
    }
}

#[test]
fn other_grants_and_unknown_provenance_are_preserved() {
    let store = fixture();
    let asset = upload(&store, "g1");
    assert_eq!(upload(&store, "g2").asset_id, asset.asset_id);
    store
        .conn()
        .unwrap()
        .execute(
            "UPDATE social_snapshot_asset_grants SET uploaded_at=?1 WHERE group_id='g2'",
            [later()],
        )
        .unwrap();
    assert_eq!(orphan_media::cleanup(&store, "author", later()).unwrap(), 0);
    assert!(has_media(&store, &asset.asset_id));
    store
        .conn()
        .unwrap()
        .execute("DELETE FROM social_snapshot_orphan_media", [])
        .unwrap();
    assert_eq!(
        orphan_media::cleanup(&store, "author", later() + 25 * 3600).unwrap(),
        0
    );
    assert!(has_media(&store, &asset.asset_id));
}

#[test]
fn unknown_foreign_key_reference_aborts_cleanup_without_losing_grants() {
    let store = fixture();
    let asset = upload(&store, "g1");
    store
        .conn()
        .unwrap()
        .execute_batch(
            "CREATE TABLE extra_publication(asset_id TEXT REFERENCES social_content_media(id));",
        )
        .unwrap();
    store
        .conn()
        .unwrap()
        .execute(
            "INSERT INTO extra_publication VALUES (?1)",
            [&asset.asset_id],
        )
        .unwrap();
    assert!(orphan_media::cleanup(&store, "author", later()).is_err());
    assert!(has_media(&store, &asset.asset_id));
    let retained: bool = store
        .conn()
        .unwrap()
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM social_snapshot_asset_grants WHERE media_id=?1)",
            [&asset.asset_id],
            |r| r.get(0),
        )
        .unwrap();
    assert!(retained);
}
