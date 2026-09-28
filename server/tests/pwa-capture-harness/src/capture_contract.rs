//! Real browser integration between the production PWA response and semantic writer adapter.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

#[tokio::test]
async fn semantic_writer_reads_real_capture_response_and_rejects_manifest_alias() {
    let root = std::env::temp_dir().join(format!("elon-capture-contract-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let mut request = [0u8; 8192];
            let _ = stream.read(&mut request).await;
            let html = r#"<!doctype html><main id="loginView"><form><input type="password"><button id="submit">Sign in</button></form></main>"#;
            let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}", html.len());
            let _ = stream.write_all(response.as_bytes()).await;
        }
    });
    let response = crate::node_agent_pwa_runtime::capture_tool(root.to_str(), json!({
        "url":url,"viewport":{"width":411,"height":842},
        "waitFor":{"selector":"#loginView"},
        "expectedPage":{"kind":"PUBLIC_LOGIN","pageId":"contract-test","path":"/","readySelector":"#loginView"},
        "evidence":{"sourceRevision":"test-capture-contract","routeRevision":"test-login"}
    })).await;
    server.abort();
    assert_eq!(
        response["ok"], true,
        "actual browser capture required: {response:#}"
    );
    let captured = crate::semantic_capture_result::read(&root, &response).unwrap();
    let tree: Value = serde_json::from_slice(&captured.tree).unwrap();
    let manifest: Value = serde_json::from_slice(&captured.manifest).unwrap();
    assert_eq!(tree["schema"], "elon.web.semantic-tree.v1");
    assert_eq!(tree["truncated"], false);
    assert!(tree["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|node| node["selector"] == "#submit"
            && node["role"] == "button"
            && node["label"] == "Sign in"
            && node["disabled"] == false));
    assert_eq!(
        manifest["semanticTree"]["sha256"],
        hex::encode(Sha256::digest(&captured.tree))
    );
    assert_eq!(
        manifest["artifact"]["sha256"],
        hex::encode(Sha256::digest(&captured.image))
    );
    assert!(image::load_from_memory(&captured.image).is_ok());
    let mut wrong_shape = response;
    let ui_tree = wrong_shape
        .as_object_mut()
        .unwrap()
        .remove("uiTree")
        .unwrap();
    wrong_shape["semanticTree"] = ui_tree;
    assert!(crate::semantic_capture_result::read(&root, &wrong_shape).is_err());
    std::fs::remove_dir_all(root).unwrap();
}
