use super::*;

#[test]
fn message_timeline_full_migration_ai_channel_equivalence_and_authorization() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    crate::store_schema::apply_migrations(&conn).unwrap();
    schema::migrate(&conn).unwrap();
    let store = Store {
        conn: std::sync::Mutex::new(conn),
    };
    let owner = store.ensure_device_user("timeline-a").unwrap();
    let other = store.ensure_device_user("timeline-b").unwrap();
    let project = store
        .create_project(&owner.id, "Timeline fixture", None, None)
        .unwrap()
        .project;
    store
        .ensure_conversation(&project.id, &owner.id, Some("test-ai"), None)
        .unwrap();
    store
        .add_message(
            &project.id,
            Some("test-ai"),
            None,
            Some(&owner.id),
            "user",
            "hello",
        )
        .unwrap();
    store
        .add_message(
            &project.id,
            Some("test-ai"),
            None,
            None,
            "assistant",
            "reply",
        )
        .unwrap();
    let mut request = TimelineRequest {
        kind: "ai".into(),
        id: "test-ai".into(),
        project: project.id.clone(),
        before: None,
        sync: None,
        limit: None,
    };
    let first = store.read_message_timeline(&owner.id, &request).unwrap();
    assert_eq!(first.messages.len(), 2);
    assert!(store.read_message_timeline(&other.id, &request).is_err());
    let expected = store
        .list_user_conversation_messages(&project.id, &owner.id, "test-ai", 50)
        .unwrap();
    for (mut actual, expected) in first.messages.into_iter().zip(expected) {
        actual.as_object_mut().unwrap().remove("timeline_cursor");
        assert_eq!(actual, serde_json::to_value(expected).unwrap());
    }
    // Legacy duplicate conversation names cannot become a cross-user timeline.
    store
        .ensure_conversation(&project.id, &other.id, Some("test-ai"), None)
        .unwrap();
    assert!(store.read_message_timeline(&owner.id, &request).is_err());
    let channels = store
        .list_project_space_channels(&owner.id, &project.id)
        .unwrap();
    let channel = channels.iter().find(|c| c.kind == "discussion").unwrap();
    let message = store
        .insert_project_channel_message(
            &project.id,
            &channel.id,
            Some(&owner.id),
            "text",
            "channel body",
            None,
            None,
        )
        .unwrap();
    request.kind = "channel".into();
    request.id = channel.id.clone();
    let mut actual = store
        .read_message_timeline(&owner.id, &request)
        .unwrap()
        .messages
        .remove(0);
    actual.as_object_mut().unwrap().remove("timeline_cursor");
    assert_eq!(actual, serde_json::to_value(message).unwrap());
    assert!(store.read_message_timeline(&other.id, &request).is_err());
    store
        .mark_timeline_read(&owner.id, &request, actual["id"].as_str().unwrap())
        .unwrap();
}
