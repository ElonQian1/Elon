use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};

fn fixture() -> Store {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON;
        CREATE TABLE users(id TEXT PRIMARY KEY,nickname TEXT,email TEXT,phone TEXT);
        CREATE TABLE friend_groups(id TEXT PRIMARY KEY,updated_at TEXT);
        CREATE TABLE friend_group_members(group_id TEXT,user_id TEXT,last_read_at TEXT);
        CREATE TABLE friend_group_messages(id TEXT PRIMARY KEY,group_id TEXT,sender_user_id TEXT,content TEXT,attachments_json TEXT,created_at TEXT,recalled_at TEXT);
        INSERT INTO users VALUES('author','作者',NULL,NULL),('reader','读者',NULL,NULL),('other','外部用户',NULL,NULL);
        INSERT INTO friend_groups VALUES('g1',''),('g2','');
        INSERT INTO friend_group_members VALUES('g1','author',''),('g1','reader',''),('g2','author','');").unwrap();
    migration::migrate(&conn).unwrap();
    migration::migrate(&conn).unwrap();
    Store {
        conn: std::sync::Mutex::new(conn),
    }
}
fn doc(title: &str) -> ArticleDocument {
    ArticleDocument {
        title: title.into(),
        blocks: vec![ArticleBlock::Paragraph {
            text: "第一段正文".into(),
        }],
        ..Default::default()
    }
}
fn publish(store: &Store, id: &str, version: i64) -> ArticlePublishResult {
    store
        .publish_article("author", id, version, &["g1".into()])
        .unwrap()
}
#[test]
fn draft_is_private_and_published_revision_is_immutable() {
    let s = fixture();
    let a = s.create_article("author", doc("原始标题")).unwrap();
    assert!(s.article_draft("reader", &a.card.id).is_err());
    assert!(s.read_article("reader", &a.card.id, 1, false).is_err());
    publish(&s, &a.card.id, 1);
    s.save_article("author", &a.card.id, 1, doc("尚未发布的修改"))
        .unwrap();
    assert_eq!(
        s.read_article("reader", &a.card.id, 1, false)
            .unwrap()
            .document
            .title,
        "原始标题"
    );
    assert!(s.read_article("reader", &a.card.id, 2, false).is_err());
    assert!(s
        .save_article("author", &a.card.id, 1, doc("过期设备"))
        .is_err());
    let c = s.conn().unwrap();
    assert!(c
        .execute("UPDATE social_content_revisions SET document_json='{}'", [])
        .is_err());
}
#[test]
fn duplicate_publish_is_idempotent_and_can_add_another_group() {
    let s = fixture();
    let a = s.create_article("author", doc("文章")).unwrap();
    assert_eq!(publish(&s, &a.card.id, 1).messages.len(), 1);
    assert!(publish(&s, &a.card.id, 1).messages.is_empty());
    let next = s
        .publish_article("author", &a.card.id, 1, &["g1".into(), "g2".into()])
        .unwrap();
    assert_eq!(next.messages.len(), 1);
    assert_eq!(next.already_shared_groups, vec!["g1"]);
    assert_eq!(
        s.conn()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM friend_group_messages", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        2
    );
}

#[test]
fn conversation_preview_contains_article_title_instead_of_protocol() {
    let s = fixture();
    let a = s.create_article("author", doc("群内周报")).unwrap();
    let p = publish(&s, &a.card.id, 1);
    assert_eq!(
        message_preview(&p.messages[0].content).as_deref(),
        Some("[文章] 群内周报")
    );
    assert_eq!(message_preview("普通消息"), None);
    assert_eq!(message_preview("【一龙文章】\n{\"schema\":9}"), None);
}
#[test]
fn group_validation_and_message_failure_roll_back_whole_publication() {
    let s = fixture();
    let a = s.create_article("author", doc("文章")).unwrap();
    assert!(s
        .publish_article("author", &a.card.id, 1, &["g1".into(), "missing".into()])
        .is_err());
    s.conn().unwrap().execute_batch("CREATE TRIGGER fail_second BEFORE INSERT ON friend_group_messages WHEN NEW.group_id='g2' BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    assert!(s
        .publish_article("author", &a.card.id, 1, &["g1".into(), "g2".into()])
        .is_err());
    let c = s.conn().unwrap();
    for table in [
        "social_content_revisions",
        "social_content_distributions",
        "friend_group_messages",
    ] {
        assert_eq!(
            c.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}
#[test]
fn outsiders_departed_members_recall_and_withdrawal_lose_access() {
    let s = fixture();
    let a = s.create_article("author", doc("文章")).unwrap();
    let p = publish(&s, &a.card.id, 1);
    assert!(s.read_article("other", &a.card.id, 1, false).is_err());
    assert_eq!(
        s.list_articles("reader", Some("g1"), 0)
            .unwrap()
            .items
            .len(),
        1
    );
    s.conn()
        .unwrap()
        .execute(
            "DELETE FROM friend_group_members WHERE user_id='reader'",
            [],
        )
        .unwrap();
    assert!(s.read_article("reader", &a.card.id, 1, false).is_err());
    s.conn()
        .unwrap()
        .execute_batch("INSERT INTO friend_group_members VALUES('g1','reader','')")
        .unwrap();
    s.conn()
        .unwrap()
        .execute(
            "UPDATE friend_group_messages SET recalled_at='now' WHERE id=?1",
            [&p.messages[0].id],
        )
        .unwrap();
    assert!(s.read_article("reader", &a.card.id, 1, false).is_err());
    assert!(s.withdraw_article("other", &a.card.id, 1).is_err());
    s.withdraw_article("author", &a.card.id, 1).unwrap();
    assert!(s.read_article("author", &a.card.id, 1, false).is_err());
    assert!(s
        .publish_article("author", &a.card.id, 1, &["g1".into()])
        .is_err());
}
#[test]
fn deleting_chat_message_releases_distribution_without_deleting_article() {
    let s = fixture();
    let a = s.create_article("author", doc("文章")).unwrap();
    let p = publish(&s, &a.card.id, 1);
    s.conn()
        .unwrap()
        .execute(
            "DELETE FROM friend_group_messages WHERE id=?1",
            [&p.messages[0].id],
        )
        .unwrap();
    assert!(s
        .list_articles("reader", Some("g1"), 0)
        .unwrap()
        .items
        .is_empty());
    assert!(s.article_draft("author", &a.card.id).is_ok());
}
#[test]
fn images_are_validated_owned_and_available_only_with_document_permission() {
    let s = fixture();
    assert!(s.upload_article_media("author", "PHNjcmlwdD4=").is_err());
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::RgbImage::new(2, 2)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    let encoded = STANDARD.encode(bytes.into_inner());
    let m = s.upload_article_media("author", &encoded).unwrap();
    assert_eq!(s.upload_article_media("author", &encoded).unwrap().id, m.id);
    let repeated = ArticleDocument {
        title: "重复图片".into(),
        blocks: vec![
            ArticleBlock::Image {
                media_id: m.id.clone(),
                caption: String::new()
            };
            13
        ],
        ..Default::default()
    };
    assert!(s.create_article("author", repeated).is_err());
    let mut document = doc("图文");
    document.cover = Some(m.id.clone());
    assert!(s.create_article("other", document.clone()).is_err());
    let a = s.create_article("author", document).unwrap();
    publish(&s, &a.card.id, 1);
    let full = s.read_article("reader", &a.card.id, 1, false).unwrap();
    assert_eq!(full.media.get(&m.id), Some(&m.data_url));
    let compact = s.read_article("reader", &a.card.id, 1, true).unwrap();
    assert!(compact.media.is_empty());
    assert!(compact.document.blocks.is_empty());
    assert!(compact.card.cover_data_url.is_some());
}
#[test]
fn empty_body_and_unsupported_blocks_are_rejected() {
    let s = fixture();
    let a = s
        .create_article("author", ArticleDocument::default())
        .unwrap();
    assert!(s
        .publish_article("author", &a.card.id, 1, &["g1".into()])
        .is_err());
    assert!(serde_json::from_str::<ArticleDocument>(
        r#"{"blocks":[{"type":"html","text":"<script>"}]}"#
    )
    .is_err());
}
