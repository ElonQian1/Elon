use super::*;

fn fixture() -> (Connection, ProjectAction) {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE users(id TEXT PRIMARY KEY); CREATE TABLE friend_groups(id TEXT PRIMARY KEY,name TEXT);
        CREATE TABLE friend_group_members(user_id TEXT,group_id TEXT);
        INSERT INTO users VALUES('u'),('other'); INSERT INTO friend_groups VALUES('g','Group'),('g2','Group');
        INSERT INTO friend_group_members VALUES('u','g'),('other','g'),('u','g2');").unwrap();
    migrate(&conn).unwrap();
    let req = ProjectAction {
        action: "acquire".into(),
        account_scope: "a".repeat(64),
        operation_id: uuid::Uuid::new_v4().to_string(),
        lease_id: None,
        generation: None,
        project_id: None,
        conversation_id: None,
        confirmed_missing: false,
    };
    (conn, req)
}
fn leased(req: &mut ProjectAction, value: &ProjectBinding) {
    req.lease_id = Some(value.lease_id.clone());
    req.generation = Some(value.generation);
}

#[test]
fn project_binding_is_member_account_and_group_scoped_not_name_scoped() {
    let (mut conn, mut req) = fixture();
    let first = apply(&mut conn, "u", "g", &req, 1).unwrap();
    assert!(apply(&mut conn, "u", "g", &req, 2).is_err());
    assert!(apply(&mut conn, "outsider", "g", &req, 2).is_err());
    let other = apply(&mut conn, "other", "g", &req, 2).unwrap();
    let group = apply(&mut conn, "u", "g2", &req, 2).unwrap();
    assert_ne!(first.binding_id, other.binding_id);
    assert_ne!(first.binding_id, group.binding_id);
    req.account_scope = "b".repeat(64);
    assert_ne!(
        first.binding_id,
        apply(&mut conn, "u", "g", &req, 2).unwrap().binding_id
    );
    conn.execute("UPDATE friend_groups SET name='Renamed' WHERE id='g'", [])
        .unwrap();
    req.account_scope = "a".repeat(64);
    assert_eq!(
        first.binding_id,
        apply(&mut conn, "u", "g", &req, 302).unwrap().binding_id
    );
}

#[test]
fn unknown_create_survives_expiry_and_cannot_be_replayed() {
    let (mut conn, mut req) = fixture();
    let lease = apply(&mut conn, "u", "g", &req, 1).unwrap();
    leased(&mut req, &lease);
    req.action = "create_begin".into();
    apply(&mut conn, "u", "g", &req, 2).unwrap();
    assert!(apply(&mut conn, "u", "g", &req, 3).is_err());
    let old_lease = req.lease_id.clone();
    req.action = "acquire".into();
    req.operation_id = uuid::Uuid::new_v4().to_string();
    let recovered = apply(&mut conn, "u", "g", &req, 302).unwrap();
    assert_eq!(recovered.state, "creating");
    leased(&mut req, &recovered);
    req.action = "create_begin".into();
    assert!(apply(&mut conn, "u", "g", &req, 303).is_err());
    req.action = "bind".into();
    req.project_id = Some(format!("g-p-{}", "a".repeat(32)));
    req.lease_id = old_lease;
    assert!(apply(&mut conn, "u", "g", &req, 303).is_err());
    leased(&mut req, &recovered);
    assert_eq!(
        apply(&mut conn, "u", "g", &req, 304).unwrap().state,
        "ready"
    );
}

#[test]
fn conversation_commit_and_rebuild_require_current_lease_and_generation() {
    let (mut conn, mut req) = fixture();
    let lease = apply(&mut conn, "u", "g", &req, 1).unwrap();
    leased(&mut req, &lease);
    req.action = "create_begin".into();
    apply(&mut conn, "u", "g", &req, 2).unwrap();
    req.action = "bind".into();
    req.project_id = Some(format!("g-p-{}", "a".repeat(32)));
    apply(&mut conn, "u", "g", &req, 3).unwrap();
    req.action = "conversation".into();
    req.conversation_id = Some(uuid::Uuid::new_v4().to_string());
    assert_eq!(
        apply(&mut conn, "u", "g", &req, 4).unwrap().conversation_id,
        req.conversation_id
    );
    req.conversation_id = Some(uuid::Uuid::new_v4().to_string());
    assert!(apply(&mut conn, "u", "g", &req, 5).is_err());
    req.action = "rebuild".into();
    assert!(apply(&mut conn, "u", "g", &req, 6).is_err());
    req.confirmed_missing = true;
    let rebuilt = apply(&mut conn, "u", "g", &req, 7).unwrap();
    assert_eq!(rebuilt.generation, 2);
    assert_eq!(rebuilt.state, "empty");
    assert!(rebuilt.conversation_id.is_none());
    req.action = "create_begin".into();
    assert!(apply(&mut conn, "u", "g", &req, 8).is_err());
    leased(&mut req, &rebuilt);
    assert!(apply(&mut conn, "u", "g", &req, 8).is_ok());
    conn.execute("DELETE FROM friend_group_members WHERE user_id='u'", [])
        .unwrap();
    req.action = "release".into();
    assert!(apply(&mut conn, "u", "g", &req, 9).is_err());
}
