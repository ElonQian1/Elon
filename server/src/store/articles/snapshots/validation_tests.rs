use super::*;
use serde_json::json;
use tests::{document, fixture};

fn parsed(value: serde_json::Value) -> SnapshotDocument {
    serde_json::from_value(value).unwrap()
}

#[test]
fn document_schema_allowlist_rejects_transport_and_unknown_rich_fields() {
    let baseline = serde_json::to_value(document()).unwrap();
    for key in [
        "url",
        "cookies",
        "token",
        "conversation_id",
        "selected_ids",
        "raw",
    ] {
        let mut value = baseline.clone();
        value[key] = json!("private");
        assert!(serde_json::from_value::<SnapshotDocument>(value).is_err());
    }
    let mut value = baseline.clone();
    value["messages"][0]["provider_id"] = json!("vendor-id");
    assert!(serde_json::from_value::<SnapshotDocument>(value).is_err());
    let mut value = baseline;
    value["messages"][0]["parts"] = json!([{"type":"interactive","label":"unsupported"}]);
    assert!(parsed(value).validate().is_err());
}

#[test]
fn complete_text_blocks_are_preserved_without_vendor_identifiers() {
    let store = fixture();
    let mut value = serde_json::to_value(document()).unwrap();
    value["messages"][1]["parts"] = json!([{"type":"code","label":"Example","language":"rust",
        "text_block":{"version":1,"id":"old-block-id","kind":"code","title":"Example","language":"rust",
            "content":"fn main() { println!(\"hello\"); }","complete":true}}]);
    let created = store
        .create_ai_snapshot("author", "g1", "operation-1", parsed(value.clone()))
        .unwrap();
    let view = store
        .read_ai_snapshot("reader", "g1", &created.snapshot_id)
        .unwrap();
    let block = view.document.messages[1].parts[0]
        .text_block
        .as_ref()
        .unwrap();
    assert_eq!(block.content, "fn main() { println!(\"hello\"); }");
    assert!(block.id.starts_with("shared_block_"));
    value["messages"][1]["parts"][0]["text_block"]["complete"] = json!(false);
    assert!(parsed(value.clone()).validate().is_err());
    value["messages"][1]["parts"][0]["text_block"]["sourceMessageId"] = json!("vendor-message");
    assert!(serde_json::from_value::<SnapshotDocument>(value).is_err());
}

#[test]
fn finance_and_chart_accept_only_bounded_finite_native_structure() {
    let mut value = serde_json::to_value(document()).unwrap();
    value["messages"][0]["parts"] = json!([{"type":"rich_card","label":"Chart","caption":"Selected chart",
        "card":{"kind":"chart","title":"Example","series":[{"key":"value","label":"Value"}],
            "points":[{"label":"A","values":[1.0]},{"label":"B","values":[2.0]}]}}]);
    assert!(parsed(value.clone()).validate().is_ok());
    value["messages"][0]["parts"][0]["card"]["points"][0]["values"] = json!([1, 2]);
    assert!(parsed(value.clone()).validate().is_err());
    value["messages"][0]["parts"][0]["card"] = json!({"kind":"finance","title":"Example","primary_value":"10.00",
        "trend":"positive","series":[{"key":"value","label":"Value"}],"points":[{"label":"A","values":[10.0]}]});
    assert!(parsed(value.clone()).validate().is_ok());
    value["messages"][0]["parts"][0]["card"]["url"] = json!("https://private.invalid");
    assert!(serde_json::from_value::<SnapshotDocument>(value).is_err());
}

#[test]
fn private_provider_urls_and_credentials_fail_closed_in_every_text_surface() {
    for private in [
        "https://chatgpt.com/c/secret",
        "https://files.oaiusercontent.com/file",
        "Authorization: Bearer SyntheticCredentialValue123456789",
        "__Secure-next-auth.session-token=SyntheticCookieValue123456789",
        "Cookie: session=SyntheticCookieValue123456789",
        "access_token=SyntheticCredentialValue123456789",
        "https://example.org/image?token=secret",
        "https://user:password@example.org/a",
        "http://127.0.0.1/a",
    ] {
        let mut doc = document();
        doc.messages[0].content = private.into();
        assert!(doc.validate().is_err(), "accepted {private}");
        let mut doc = document();
        doc.summary = private.into();
        assert!(doc.validate().is_err(), "accepted summary {private}");
    }
    let mut doc = document();
    doc.messages[0].content = "Public citation [paper](https://example.org/paper) and $x^2$".into();
    assert!(doc.validate().is_ok());
}

#[test]
fn scheme_examples_are_preserved_as_inert_selected_text_and_code() {
    let store = fixture();
    let code = "const u='file:///tmp/example';\nconst s='sandbox:/mnt/data/example';";
    let content = format!("```javascript\n{code}\n```\njavascript:alert(1)");
    let mut value = serde_json::to_value(document()).unwrap();
    value["messages"][0]["content"] = json!(content);
    value["messages"][0]["parts"] = json!([{"type":"code","label":"URI examples",
        "text_block":{"version":1,"id":"public-example","kind":"code","title":"URI examples",
            "language":"javascript","content":code,"complete":true}}]);
    let created = store
        .create_ai_snapshot("author", "g1", "uri-examples", parsed(value))
        .unwrap();
    let read = store
        .read_ai_snapshot("reader", "g1", &created.snapshot_id)
        .unwrap();
    assert_eq!(read.document.messages[0].content, content);
    assert_eq!(
        read.document.messages[0].parts[0]
            .text_block
            .as_ref()
            .unwrap()
            .content,
        code
    );
    for example in [
        "file:///tmp/example",
        "sandbox:/mnt/data/example",
        "javascript:alert(1)",
        "vbscript:MsgBox(1)",
        "data:text/plain,example",
        "blob:https://example.org/example",
    ] {
        let mut doc = document();
        doc.messages[0].content = example.into();
        assert!(
            doc.validate().is_ok(),
            "rejected inert scheme example: {example}"
        );
    }
    let mut resource = serde_json::to_value(document()).unwrap();
    resource["messages"][0]["parts"] = json!([{"type":"image","label":"Not an uploaded asset",
        "asset_id":"file:///tmp/example"}]);
    assert!(parsed(resource.clone()).validate().is_err());
    resource["messages"][0]["parts"][0]["url"] = json!("javascript:alert(1)");
    assert!(serde_json::from_value::<SnapshotDocument>(resource).is_err());
}

#[test]
fn ordinary_code_and_public_docs_with_queries_or_fragments_are_preserved() {
    for code in [
        "// An api_key and access_token are not embedded credentials. Cookie documentation.",
        "const api_key = \"example\";",
        "<img src=\"https://example.org/image.png\">",
        "![image](https://example.org/image.png)",
        "[Docs](https://platform.openai.com/docs?language=python#authentication)",
        "[Help](https://help.openai.com/en/articles/example)",
        "chatgpt.com is a provider",
    ] {
        let mut doc = document();
        doc.messages[0].content = code.into();
        assert!(doc.validate().is_ok(), "rejected authorized prose: {code}");
    }
}

#[test]
fn empty_oversized_duplicate_and_partial_messages_are_not_silently_truncated() {
    let mut doc = document();
    doc.messages.clear();
    assert!(doc.validate().is_err());
    let mut doc = document();
    doc.messages[1].id = doc.messages[0].id.clone();
    assert!(doc.validate().is_err());
    let mut doc = document();
    doc.messages[0].content = "word ".repeat(24_001);
    assert!(doc.validate().is_err());
    let mut doc = document();
    doc.messages[0].role = "system".into();
    assert!(doc.validate().is_err());
    let mut doc = document();
    let template = doc.messages[0].clone();
    doc.messages = (0..201)
        .map(|i| SnapshotMessage {
            id: format!("public-{i}"),
            ..template.clone()
        })
        .collect();
    assert!(doc.validate().is_err());
    doc.messages.truncate(200);
    assert!(doc.validate().is_ok());
}
