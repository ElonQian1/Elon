use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};

#[test]
fn square_owned_images_video_and_immutable_media_selection() {
    let s = super::tests::fixture();
    let mut p = super::tests::selection(&s);
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::RgbImage::new(8, 8)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    let encoded = STANDARD.encode(bytes.into_inner());
    let media = s.upload_article_media("a", &encoded).unwrap();
    assert_eq!(media.id, s.upload_article_media("a", &encoded).unwrap().id);
    p.mode = Mode::Images;
    p.media_ids = vec![media.id.clone()];
    let preview = s.square_preview("a", p.clone()).unwrap();
    assert!(preview.media.contains_key(&media.id));
    let job = super::tests::enqueue(&s, &p);
    let work = s.square_claim().unwrap().unwrap();
    assert_eq!(work.payload.selection.media_ids, vec![media.id.clone()]);
    assert!(s.square_media_bytes("b", &media.id, false).is_err());
    assert!(s.square_media_bytes("a", &media.id, false).unwrap().1.len() > 12);
    s.square_cancel("a", &job.id).unwrap();
    p.media_ids.push(media.id.clone());
    assert!(s.square_preview("a", p.clone()).is_err());
    let mut mp4 = vec![0; 24];
    mp4[4..8].copy_from_slice(b"ftyp");
    assert!(s.square_video("b", mp4.clone(), 1.5, &media.id).is_err());
    assert!(s
        .square_video("a", mp4.clone(), f64::NAN, &media.id)
        .is_err());
    let video = s.square_video("a", mp4.clone(), 1.5, &media.id).unwrap();
    assert_eq!(video, s.square_video("a", mp4, 1.5, &media.id).unwrap());
    p.mode = Mode::Video;
    p.media_ids.clear();
    p.video_id = Some(video["id"].as_str().unwrap().into());
    let preview = s.square_preview("a", p.clone()).unwrap();
    assert!(preview.media.contains_key(&media.id));
    super::tests::enqueue(&s, &p);
    let w = s.square_claim().unwrap().unwrap();
    let id = w.payload.selection.video_id.unwrap();
    assert_eq!(s.square_video_metadata("a", &id).unwrap(), (1.5, media.id));
    assert_eq!(s.square_media_bytes("a", &id, true).unwrap().0, "video/mp4");
    assert!(s.square_media_bytes("b", &id, true).is_err());
}

#[test]
fn square_rebinding_cannot_exceed_pending_limit() {
    let s = super::tests::fixture();
    let p = super::tests::selection(&s);
    let original = super::tests::enqueue(&s, &p);
    s.square_unbind("a").unwrap();
    s.square_bind("a", "重新绑定", "test-square-key-0123456789")
        .unwrap();
    for _ in 0..100 {
        super::tests::enqueue(&s, &super::tests::selection(&s));
    }
    let preview = s.square_preview("a", p.clone()).unwrap();
    let result = s.square_enqueue(
        "a",
        Enqueue {
            selection: p,
            preview_hash: preview.preview_hash,
            request_key: uuid::Uuid::new_v4().to_string(),
            public_confirmed: true,
            conversion_confirmed: true,
            scheduled_at: None,
        },
    );
    assert_eq!(
        result
            .err()
            .unwrap()
            .downcast_ref::<ArticleFault>()
            .unwrap()
            .0,
        429
    );
    assert_eq!(s.square_job("a", &original.id).unwrap().status, "cancelled");
}
