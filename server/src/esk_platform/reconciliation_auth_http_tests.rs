//! Real synthetic administrator/session boundaries; no production credentials.
use super::*;
use axum::http::HeaderMap;

#[tokio::test]
async fn only_one_real_active_administrator_bearer_can_read() {
    let fixture = Fixture::new();
    let _policy = enable_fixture_policy();
    prepare(&fixture, 0).await;
    for token in [
        None,
        Some("synthetic-static-owner-not-a-session"),
        Some(fixture.state.admin_token.as_str()),
        Some("synthetic-unknown-session"),
        Some(""),
    ] {
        let headers = token.map(auth).unwrap_or_default();
        let (status, value) = raw(&fixture, "?never-echo=1", headers, Body::empty()).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(value.as_object().unwrap().len(), 1);
    }
    assert_eq!(
        raw(&fixture, "", auth(&fixture.user_token), Body::empty())
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let mut cookie = HeaderMap::new();
    cookie.insert(
        header::COOKIE,
        format!("session_token={}", fixture.admin_token)
            .parse()
            .unwrap(),
    );
    assert_eq!(
        raw(&fixture, "", cookie, Body::empty()).await.0,
        StatusCode::UNAUTHORIZED
    );
    let mut duplicated = auth(&fixture.admin_token);
    duplicated.append(
        header::AUTHORIZATION,
        format!("Bearer {}", fixture.admin_token).parse().unwrap(),
    );
    assert_eq!(
        raw(&fixture, "", duplicated, Body::empty()).await.0,
        StatusCode::UNAUTHORIZED
    );
    let actor = fixture
        .state
        .store
        .authenticate_token(&fixture.admin_token)
        .unwrap();
    fixture
        .state
        .store
        .conn()
        .unwrap()
        .execute("UPDATE users SET role='owner' WHERE id=?1", [&actor.id])
        .unwrap();
    assert_eq!(get(&fixture).await["key_count"], "1");
    fixture.cleanup();
}

#[tokio::test]
async fn database_session_for_virtual_owner_is_not_a_real_administrator() {
    let fixture = Fixture::new();
    fixture.state.store.conn().unwrap().execute_batch(
        "INSERT INTO users(id,email,password_hash,role,status,created_at,updated_at)
         VALUES('local-owner','virtual@example.test','not-a-password','owner','active','2026-01-01','2026-01-01')"
    ).unwrap();
    let token = fixture
        .state
        .store
        .create_session("local-owner", Some("synthetic-virtual"), None)
        .unwrap()
        .0;
    let (status, value) = raw(&fixture, "", auth(&token), Body::empty()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(value.get("used_payment_keys").is_none());
    fixture.cleanup();
}

#[tokio::test]
async fn malformed_expired_revoked_and_disabled_admins_cannot_export() {
    let fixture = Fixture::new();
    let _policy = enable_fixture_policy();
    prepare(&fixture, 0).await;
    let actor = fixture
        .state
        .store
        .authenticate_token(&fixture.admin_token)
        .unwrap();
    for expiry in [
        "not-a-date",
        "2000-01-01T00:00:00Z",
        "2000-01-01T08:00:00+08:00",
    ] {
        fixture
            .state
            .store
            .conn()
            .unwrap()
            .execute(
                "UPDATE sessions SET expires_at=?1 WHERE user_id=?2",
                rusqlite::params![expiry, actor.id],
            )
            .unwrap();
        assert_eq!(
            raw(&fixture, "", auth(&fixture.admin_token), Body::empty())
                .await
                .0,
            StatusCode::UNAUTHORIZED
        );
    }
    fixture.state.store.conn().unwrap().execute("UPDATE sessions SET expires_at='2099-01-01T00:00:00Z',revoked_at='synthetic-revocation' WHERE user_id=?1", [&actor.id]).unwrap();
    assert_eq!(
        raw(&fixture, "", auth(&fixture.admin_token), Body::empty())
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    fixture
        .state
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE sessions SET revoked_at=NULL WHERE user_id=?1",
            [&actor.id],
        )
        .unwrap();
    fixture
        .state
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE users SET status='disabled' WHERE id=?1",
            [&actor.id],
        )
        .unwrap();
    assert_eq!(
        raw(&fixture, "", auth(&fixture.admin_token), Body::empty())
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    fixture.cleanup();
}

#[tokio::test]
async fn successful_real_admin_precheck_cannot_authorize_a_changed_session_snapshot() {
    for mutation in ["revoke", "expire", "demote", "rebind"] {
        let fixture = Fixture::new();
        let _policy = enable_fixture_policy();
        prepare(&fixture, 0).await;
        let headers = auth(&fixture.admin_token);
        let (actor, token) =
            super::super::super::api::administrator(&fixture.state, &headers).unwrap();
        assert!(fixture
            .state
            .store
            .esk_platform_reconciliation_snapshot(&actor.id, token)
            .is_ok());
        let sql = match mutation {
            "revoke" => "UPDATE sessions SET revoked_at='synthetic-revocation' WHERE user_id=?1",
            "expire" => "UPDATE sessions SET expires_at='2000-01-01T00:00:00Z' WHERE user_id=?1",
            "demote" => "UPDATE users SET role='user' WHERE id=?1",
            _ => "UPDATE sessions SET user_id=?2 WHERE user_id=?1",
        };
        {
            let conn = fixture.state.store.conn().unwrap();
            if mutation == "rebind" {
                conn.execute(sql, rusqlite::params![actor.id, fixture.user_id])
                    .unwrap();
            } else {
                conn.execute(sql, [&actor.id]).unwrap();
            }
        }
        let before = business_state(&fixture);
        let error = fixture
            .state
            .store
            .esk_platform_reconciliation_snapshot(&actor.id, token)
            .unwrap_err();
        assert_eq!(
            error.downcast_ref::<PlatformError>(),
            Some(&PlatformError::Unauthorized)
        );
        assert_eq!(before, business_state(&fixture));
        fixture.cleanup();
    }
}
