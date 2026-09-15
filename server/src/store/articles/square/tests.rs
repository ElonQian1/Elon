use super::*;
pub(super) fn fixture() -> Store {
    static ENV: std::sync::Once = std::sync::Once::new();
    ENV.call_once(|| {
        if std::env::var("USER_API_KEY_SECRET").is_err() && std::env::var("SECRET_KEY").is_err() {
            std::env::set_var("USER_API_KEY_SECRET", "square-test-only-encryption-root");
        }
    });
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE users(id TEXT PRIMARY KEY,nickname TEXT); INSERT INTO users VALUES('a','作者'),('b','他人'); CREATE TABLE friend_groups(id TEXT PRIMARY KEY); CREATE TABLE friend_group_messages(id TEXT PRIMARY KEY);").unwrap();
    super::super::migration::migrate(&conn).unwrap();
    migration::migrate(&conn).unwrap();
    migration::migrate(&conn).unwrap();
    let s = Store {
        conn: std::sync::Mutex::new(conn),
    };
    s.square_bind("a", "我的广场", "test-square-key-0123456789")
        .unwrap();
    s
}
pub(super) fn selection(s: &Store) -> Selection {
    let a = s
        .create_article(
            "a",
            ArticleDocument {
                title: "研究记录".into(),
                blocks: vec![ArticleBlock::Paragraph {
                    text: "正文内容".into(),
                }],
                ..Default::default()
            },
        )
        .unwrap();
    Selection {
        article_id: a.card.id,
        version: 1,
        mode: Mode::Article,
        media_ids: vec![],
        cover_id: None,
        video_id: None,
    }
}
pub(super) fn enqueue(s: &Store, p: &Selection) -> Job {
    let preview = s.square_preview("a", p.clone()).unwrap();
    s.square_enqueue(
        "a",
        Enqueue {
            selection: p.clone(),
            preview_hash: preview.preview_hash,
            request_key: uuid::Uuid::new_v4().to_string(),
            public_confirmed: true,
            conversion_confirmed: true,
            scheduled_at: None,
        },
    )
    .unwrap()
}
#[test]
fn square_credentials_are_owner_bound_masked_and_cancellable() {
    let s = fixture();
    let p = selection(&s);
    let j = enqueue(&s, &p);
    let public = serde_json::to_string(&s.square_account("a").unwrap()).unwrap();
    assert!(!public.contains("test-square-key"));
    assert!(s.square_account("a").unwrap().verified_at.is_none());
    assert!(!s.square_account("b").unwrap().bound);
    let encrypted: String = s
        .conn()
        .unwrap()
        .query_row(
            "SELECT encrypted_key FROM article_square_accounts WHERE owner_id='a'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(!encrypted.contains("test-square-key"));
    assert!(reveal("b", &encrypted).is_err());
    s.square_unbind("a").unwrap();
    assert_eq!(s.square_job("a", &j.id).unwrap().status, "cancelled");
    assert!(s.square_claim().unwrap().is_none());
}
#[test]
fn square_duplicate_content_has_one_job_and_private_revision() {
    let s = fixture();
    let p = selection(&s);
    let a = enqueue(&s, &p);
    let b = enqueue(&s, &p);
    assert_eq!(a.id, b.id);
    assert!(s.square_preview("b", p.clone()).is_err());
    assert!(s.square_job("b", &a.id).is_err());
    assert!(s.square_retry("b", &a.id).is_err());
    assert!(s.read_article("b", &p.article_id, 1, false).is_err());
    let work = s.square_claim().unwrap().unwrap();
    assert!(s.square_claim().unwrap().is_none());
    s.square_cancel("a", &a.id).unwrap();
    assert!(s.square_submitting(&work).is_err());
}
#[test]
fn square_stale_preview_and_unconfirmed_publication_fail() {
    let s = fixture();
    let p = selection(&s);
    let preview = s.square_preview("a", p.clone()).unwrap();
    let mut input = Enqueue {
        selection: p.clone(),
        preview_hash: preview.preview_hash,
        request_key: uuid::Uuid::new_v4().to_string(),
        public_confirmed: false,
        conversion_confirmed: true,
        scheduled_at: None,
    };
    assert!(s.square_enqueue("a", input).is_err());
    let preview = s.square_preview("a", p.clone()).unwrap();
    s.square_bind("a", "另一个账号", "second-test-square-key-987654321")
        .unwrap();
    input = Enqueue {
        selection: p,
        preview_hash: preview.preview_hash,
        request_key: uuid::Uuid::new_v4().to_string(),
        public_confirmed: true,
        conversion_confirmed: true,
        scheduled_at: None,
    };
    assert!(s.square_enqueue("a", input).is_err());
}
#[test]
fn square_uncertain_requires_explicit_reconciliation() {
    let s = fixture();
    let p = selection(&s);
    let job = enqueue(&s, &p);
    let work = s.square_claim().unwrap().unwrap();
    s.square_submitting(&work).unwrap();
    assert!(s.square_cancel("a", &job.id).is_err());
    s.square_finish(&work, "uncertain", "回执丢失", None)
        .unwrap();
    assert!(s.square_retry("a", &job.id).is_err());
    assert!(s
        .square_resolve("a", &job.id, Some("javascript:secret"), false)
        .is_err());
    s.square_resolve("a", &job.id, None, true).unwrap();
    s.square_retry("a", &job.id).unwrap();
    let work = s.square_claim().unwrap().unwrap();
    s.square_submitting(&work).unwrap();
    s.square_finish(&work, "published", "成功", Some("123456789"))
        .unwrap();
    assert!(s
        .square_job("a", &job.id)
        .unwrap()
        .post_url
        .unwrap()
        .ends_with("123456789"));
    assert!(s.square_account("a").unwrap().verified_at.is_some());
    assert!(s.square_retry("a", &job.id).is_err());
}
#[test]
fn square_scheduled_jobs_do_not_run_early_and_restart_never_reposts() {
    let s = fixture();
    let p = selection(&s);
    let pre = s.square_preview("a", p.clone()).unwrap();
    let j = s
        .square_enqueue(
            "a",
            Enqueue {
                selection: p,
                preview_hash: pre.preview_hash,
                request_key: uuid::Uuid::new_v4().to_string(),
                public_confirmed: true,
                conversion_confirmed: true,
                scheduled_at: Some(epoch() + 3600),
            },
        )
        .unwrap();
    assert!(s.square_claim().unwrap().is_none());
    s.conn()
        .unwrap()
        .execute(
            "UPDATE article_square_jobs SET scheduled_at=0 WHERE id=?1",
            [&j.id],
        )
        .unwrap();
    let w = s.square_claim().unwrap().unwrap();
    s.square_submitting(&w).unwrap();
    s.conn()
        .unwrap()
        .execute(
            "UPDATE article_square_jobs SET updated_at=0 WHERE id=?1",
            [&j.id],
        )
        .unwrap();
    assert!(s.square_claim().unwrap().is_none());
    assert_eq!(s.square_job("a", &j.id).unwrap().status, "uncertain");
}
#[test]
fn square_withdrawal_and_rotation_fence_active_preparation() {
    let s = fixture();
    let p = selection(&s);
    let j = enqueue(&s, &p);
    let work = s.square_claim().unwrap().unwrap();
    s.withdraw_article("a", &p.article_id, 1).unwrap();
    assert!(s.square_submitting(&work).is_err());
    s.square_claim().unwrap();
    assert_eq!(s.square_job("a", &j.id).unwrap().status, "cancelled");
    let p = selection(&s);
    enqueue(&s, &p);
    let w = s.square_claim().unwrap().unwrap();
    s.square_bind("a", "换号", "third-square-test-key-1111111")
        .unwrap();
    assert!(s.square_work_key(&w).is_err());
}
#[test]
fn square_provider_never_confuses_timeout_or_unsafe_url_with_success() {
    assert!(matches!(
        provider::classify(504, &json!({})),
        provider::Outcome::Uncertain
    ));
    assert!(matches!(
        provider::classify(200, &json!({"code":"000000","data":{}})),
        provider::Outcome::Uncertain
    ));
    assert!(matches!(
        provider::classify(200, &json!({"code":"000000","data":{"id":"123"}})),
        provider::Outcome::Published(_)
    ));
    match provider::classify(400, &json!({"code":"220004","message":"secret-test-key"})) {
        provider::Outcome::Failed(m) => assert!(!m.contains("secret")),
        _ => panic!("expected rejection"),
    };
    for u in [
        "http://s3.amazonaws.com/x",
        "https://127.0.0.1/x",
        "https://binance.com.evil.test/x",
        "https://user:pass@s3.amazonaws.com/x",
        "https://s3.amazonaws.com:444/x",
    ] {
        assert!(!provider::trusted_media_url(u));
    }
    assert!(provider::trusted_media_url(
        "https://bucket.s3.amazonaws.com/x?signature=opaque"
    ));
}
#[test]
fn square_media_modes_and_limits_are_enforced() {
    let s = fixture();
    let mut p = selection(&s);
    p.mode = Mode::Images;
    assert!(s.square_preview("a", p.clone()).is_err());
    p.media_ids = vec!["foreign".into()];
    assert!(s.square_preview("a", p.clone()).is_err());
    p.mode = Mode::Text;
    assert!(s.square_preview("a", p.clone()).is_err());
    assert!(s.square_video("a", vec![0; 20], 1.0, "missing").is_err());
    let p = selection(&s);
    enqueue(&s, &p);
    let w = s.square_claim().unwrap().unwrap();
    for i in 0..100 {
        s.conn()
            .unwrap()
            .execute(
                "INSERT INTO article_square_submissions VALUES(?1,'a',?2)",
                params![format!("quota{i}"), epoch()],
            )
            .unwrap();
    }
    assert!(s.square_submitting(&w).is_err());
}

#[test]
fn square_lost_request_can_recover_after_draft_change_and_key_rotation() {
    let s = fixture();
    let p = selection(&s);
    let pre = s.square_preview("a", p.clone()).unwrap();
    let key = uuid::Uuid::new_v4().to_string();
    let input = || Enqueue {
        selection: p.clone(),
        preview_hash: pre.preview_hash.clone(),
        request_key: key.clone(),
        public_confirmed: true,
        conversion_confirmed: true,
        scheduled_at: None,
    };
    let first = s.square_enqueue("a", input()).unwrap();
    s.conn()
        .unwrap()
        .execute(
            "UPDATE social_contents SET edit_version=2 WHERE id=?1",
            [&p.article_id],
        )
        .unwrap();
    s.square_unbind("a").unwrap();
    assert_eq!(s.square_enqueue("a", input()).unwrap().id, first.id);
    let mut changed = input();
    changed.scheduled_at = Some(epoch() + 3600);
    assert!(s.square_enqueue("a", changed).is_err());
}

#[test]
fn square_rebinding_same_key_requires_new_preview_and_preserves_deduplication() {
    let s = fixture();
    let p = selection(&s);
    let first = enqueue(&s, &p);
    s.square_unbind("a").unwrap();
    s.square_bind("a", "重新绑定", "test-square-key-0123456789")
        .unwrap();
    let next = enqueue(&s, &p);
    assert_eq!(first.id, next.id);
    assert_eq!(next.status, "queued");
    let work = s.square_claim().unwrap().unwrap();
    assert!(s.square_work_key(&work).is_ok());
}
