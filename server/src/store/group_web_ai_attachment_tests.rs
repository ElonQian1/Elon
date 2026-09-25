use super::*;
use serde_json::json;

fn image() -> Value {
    json!({"attachment_id":"fixture", "display_name":"图.png", "mime_type":"image/png", "size_bytes":123,
        "url":"https://platform.example/api/user/u/chat-attachments/g/image.png"})
}

#[test]
fn ordered_files_have_unique_prompt_names_and_no_external_origin() {
    let mut files = Vec::new();
    append(&mut files, "m1", &json!([image()])).unwrap();
    append(&mut files, "m2", &json!([image()])).unwrap();
    assert_eq!(files[0].message_id, "m1");
    assert_eq!(files[1].message_id, "m2");
    assert_ne!(files[0].name, files[1].name);
    assert!(files[0].download_path.starts_with("/api/user/"));
    assert!(!serde_json::to_string(&files)
        .unwrap()
        .contains("platform.example"));
}

#[test]
fn malformed_unsupported_and_oversize_files_fail_closed() {
    for (field, value) in [
        ("url", json!("https://example.org/private")),
        ("mime_type", json!("audio/mpeg")),
        ("size_bytes", json!(9 * 1024 * 1024)),
        ("attachment_id", json!("")),
    ] {
        let mut item = image();
        item[field] = value;
        assert!(append(&mut Vec::new(), "m", &json!([item])).is_err());
    }
    assert!(download_path("/api/user/u/chat-attachments/g/%2fsecret").is_err());
    assert!(download_path("/api/user/u/chat-attachments/g/a?token=secret").is_err());
    assert!(append(&mut Vec::new(), "m", &json!(vec![image(); 10])).is_err());
}
