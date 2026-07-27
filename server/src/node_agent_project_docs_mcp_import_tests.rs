use super::*;

#[tokio::test]
async fn directly_imports_a_git_backed_low_authority_conversation() {
    let root = workspace();
    let imported_source = handle_request(
        &root,
        request(json!({
            "jsonrpc":"2.0","id":601,"method":"tools/call",
            "params":{"name":"project_discussions_import_source","arguments":{
                "title":"商户数据讨论",
                "content":"用户：AI 为什么不能读取商户数据？\n\n助手：需要身份、授权和审计。",
                "source_reference":"chat://merchant-data",
                "suggested_filename":"merchant-data.md",
                "authorization_mode":"git_backed_full"
            }}
        })),
    )
    .await
    .unwrap();
    assert_eq!(
        imported_source["result"]["structuredContent"]["status"],
        "imported"
    );
    assert_eq!(
        imported_source["result"]["structuredContent"]["git_document_transaction_complete"],
        true
    );
    let imported_path = imported_source["result"]["structuredContent"]["path"]
        .as_str()
        .unwrap();
    assert!(imported_path.starts_with("docs/inbox/conversations/"));
    let imported_content = fs::read_to_string(root.join(imported_path)).unwrap();
    assert!(imported_content.contains("lifecycle: source_material"));
    assert!(imported_content.contains("authority: none"));
    fs::remove_dir_all(root).unwrap();
}
