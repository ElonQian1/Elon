use super::*;

async fn mark_real_runtime(session: &LiveUiSession) {
    let mut state = session.state.write().await;
    state.connected = true;
    state.nodes.push(serde_json::from_value(json!({
        "runtimeNodeId":"node-1", "definitionId":"project_plaza", "screenId":"marketplacePage",
        "kind":"view", "className":"View",
        "geometry":{"boundsInDisplayPx":{"left":1,"top":2,"right":3,"bottom":4,"width":2,"height":2},"density":1.0,"fontScale":1.0,"rotation":0,"visible":true},
        "properties":{}, "capabilities":{}
    })).unwrap());
}

#[tokio::test]
async fn real_runtime_selection_fails_closed_for_missing_pseudo_or_ambiguous_sessions() {
    let broker = LiveUiBroker::new();
    let root =
        std::env::temp_dir().join(format!("runtime-select-{}", uuid::Uuid::new_v4().simple()));
    std::fs::create_dir_all(&root).unwrap();
    let root = root.to_string_lossy().to_string();
    assert!(broker
        .unique_connected_runtime_for_project(&root)
        .await
        .is_err());
    let pseudo = broker
        .create_session(
            "ui-design-bootstrap".into(),
            "ui.design.bootstrap".into(),
            Some(root.clone()),
            1,
        )
        .await;
    mark_real_runtime(&pseudo).await;
    assert!(broker
        .unique_connected_runtime_for_project(&root)
        .await
        .is_err());
    let first = broker
        .create_session(
            "device-1".into(),
            "com.elon.app.uitest".into(),
            Some(root.clone()),
            1,
        )
        .await;
    mark_real_runtime(&first).await;
    assert_eq!(
        broker
            .unique_connected_runtime_for_project(&root)
            .await
            .unwrap()
            .id,
        first.id
    );
    let second = broker
        .create_session(
            "device-2".into(),
            "com.elon.app.uitest".into(),
            Some(root.clone()),
            1,
        )
        .await;
    mark_real_runtime(&second).await;
    let error = match broker.unique_connected_runtime_for_project(&root).await {
        Ok(_) => panic!("ambiguous runtimes must fail closed"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("身份不唯一"));
}

#[tokio::test]
async fn frame_request_retries_once_on_replacement_connection() {
    let broker = LiveUiBroker::new();
    let session = broker
        .create_session(
            "device-1".into(),
            "com.elon.app.uitest".into(),
            Some(".".into()),
            1,
        )
        .await;
    let (first_tx, mut first_rx) = mpsc::unbounded_channel();
    *session.runtime_tx.write().await = Some(first_tx);
    let request = tokio::spawn({
        let session = session.clone();
        async move { session.request_frame().await }
    });
    let _ = first_rx.recv().await.unwrap();
    let (replacement_tx, mut replacement_rx) = mpsc::unbounded_channel();
    *session.runtime_tx.write().await = Some(replacement_tx);
    let replacement = tokio::time::timeout(Duration::from_secs(5), replacement_rx.recv())
        .await
        .unwrap()
        .unwrap();
    let Message::Text(text) = replacement else {
        panic!("frame request must be text")
    };
    let request_id = serde_json::from_str::<Value>(&text).unwrap()["requestId"]
        .as_str()
        .unwrap()
        .to_string();
    session.handle_runtime_text(&json!({"messageType":"frame.snapshot","requestId":request_id,"dataUrl":"data:image/webp;base64,UklGRg==","width":1,"height":1}).to_string()).await.unwrap();
    assert_eq!(request.await.unwrap().unwrap()["requestId"], request_id);
}
