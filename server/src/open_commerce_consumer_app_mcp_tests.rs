use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    open_commerce_developer_model::{
        CreateDeveloperAppRequest, OpenCommerceDeveloperAppCredential,
    },
    store::Store,
};

const LIST_APPS: &str = "open_commerce_list_my_consumer_apps";

struct Fixture {
    store: Store,
    owner_id: String,
    project_id: String,
    active_app_id: String,
    disabled_app_id: String,
    active_token: String,
    disabled_token: String,
    teammate_id: String,
    teammate_app_id: String,
    teammate_token: String,
    other_project_id: String,
    other_project_app_id: String,
    other_project_token: String,
}

#[derive(Debug, PartialEq, Eq)]
struct ReadSnapshot {
    app_rows: Vec<(String, String, String)>,
    audit_count: i64,
}

fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "elon_consumer_app_mcp_{}.db",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).expect("consumer app MCP store should open");
    let owner = store
        .create_user(
            "consumer-app-owner@example.com",
            "secret1",
            Some("Consumer App Owner"),
            None,
        )
        .unwrap();
    let project = store
        .create_project(&owner.id, "Consumer App Project", None, None)
        .unwrap()
        .project;
    let active = create_app(&store, &project.id, &owner.id, "consumer.app-active");
    let disabled = create_app(&store, &project.id, &owner.id, "consumer.app-disabled");
    store
        .disable_open_commerce_developer_app(&project.id, &disabled.app.id)
        .unwrap();

    let teammate = store
        .create_user(
            "consumer-app-teammate@example.com",
            "secret1",
            Some("Consumer App Teammate"),
            None,
        )
        .unwrap();
    let teammate_app = create_app(&store, &project.id, &teammate.id, "consumer.app-teammate");

    let other_project = store
        .create_project(&owner.id, "Consumer App Other Project", None, None)
        .unwrap()
        .project;
    let other_project_app = create_app(
        &store,
        &other_project.id,
        &owner.id,
        "consumer.app-other-project",
    );

    Fixture {
        store,
        owner_id: owner.id,
        project_id: project.id,
        active_app_id: active.app.app_id,
        disabled_app_id: disabled.app.app_id,
        active_token: active.test_token,
        disabled_token: disabled.test_token,
        teammate_id: teammate.id,
        teammate_app_id: teammate_app.app.app_id,
        teammate_token: teammate_app.test_token,
        other_project_id: other_project.id,
        other_project_app_id: other_project_app.app.app_id,
        other_project_token: other_project_app.test_token,
    }
}

fn create_app(
    store: &Store,
    project_id: &str,
    owner_user_id: &str,
    app_id: &str,
) -> OpenCommerceDeveloperAppCredential {
    store
        .create_open_commerce_developer_app(
            project_id,
            owner_user_id,
            CreateDeveloperAppRequest {
                app_id: app_id.to_string(),
                display_name: format!("{app_id} display"),
            },
        )
        .unwrap()
}

fn call(
    fixture: &Fixture,
    project_id: &str,
    user_id: &str,
    current_app_id: &str,
    uses_default_mcp_identity: bool,
    arguments: Value,
) -> anyhow::Result<Value> {
    Ok(super::call_if_handled(
        &fixture.store,
        project_id,
        user_id,
        current_app_id,
        uses_default_mcp_identity,
        LIST_APPS,
        arguments,
    )?
    .unwrap())
}

#[test]
fn default_identity_lists_only_current_users_project_apps_without_secrets_or_writes() {
    let fixture = fixture();
    let before = snapshot(&fixture);
    let response = call(
        &fixture,
        &fixture.project_id,
        &fixture.owner_id,
        "mcp-client",
        true,
        json!({}),
    )
    .unwrap();
    assert_eq!(
        response["schema"],
        "open_commerce.consumer_app_directory.v1"
    );
    assert_eq!(response["project_id"], fixture.project_id);
    assert_eq!(response["current_mcp_identity"]["app_id"], "mcp-client");
    assert_eq!(response["current_mcp_identity"]["kind"], "default_system");
    assert_eq!(response["test_tokens_included"], false);
    assert_eq!(response["production_credentials_included"], false);
    let apps = response["apps"].as_array().unwrap();
    assert_eq!(apps.len(), 2);
    let active = app(apps, &fixture.active_app_id);
    assert_eq!(active["status"], "active");
    assert_eq!(active["environment"], "sandbox");
    assert_eq!(active["can_use_for_sandbox_mcp"], true);
    assert_eq!(active["is_current_mcp_identity"], false);
    let disabled = app(apps, &fixture.disabled_app_id);
    assert_eq!(disabled["status"], "disabled");
    assert_eq!(disabled["can_use_for_sandbox_mcp"], false);
    assert!(apps
        .iter()
        .all(|app| app["is_current_mcp_identity"] == false));

    assert_no_keys(
        &response,
        &[
            "owner_user_id",
            "test_token",
            "token_hint",
            "test_token_hash",
            "production_credential",
            "credential_hash",
        ],
    );
    let serialized = response.to_string();
    for hidden in [
        fixture.teammate_id.as_str(),
        fixture.teammate_app_id.as_str(),
        fixture.other_project_id.as_str(),
        fixture.other_project_app_id.as_str(),
        fixture.active_token.as_str(),
        fixture.disabled_token.as_str(),
        fixture.teammate_token.as_str(),
        fixture.other_project_token.as_str(),
    ] {
        assert!(!serialized.contains(hidden), "leaked value {hidden}");
    }
    assert_eq!(before, snapshot(&fixture));
}

#[tokio::test]
async fn explicit_owned_active_or_disabled_identity_is_marked_and_routed() {
    let fixture = fixture();
    let active = crate::open_commerce_mcp::call_tool(
        &fixture.store,
        &fixture.project_id,
        &fixture.owner_id,
        "owner",
        &fixture.active_app_id,
        json!({"name":LIST_APPS,"arguments":{}}),
    )
    .await
    .unwrap();
    let active = &active["structuredContent"];
    assert_eq!(active["current_mcp_identity"]["kind"], "registered_app");
    assert_eq!(
        app(active["apps"].as_array().unwrap(), &fixture.active_app_id)["is_current_mcp_identity"],
        true
    );

    let disabled = call(
        &fixture,
        &fixture.project_id,
        &fixture.owner_id,
        &fixture.disabled_app_id,
        false,
        json!({}),
    )
    .unwrap();
    let disabled = app(
        disabled["apps"].as_array().unwrap(),
        &fixture.disabled_app_id,
    );
    assert_eq!(disabled["is_current_mcp_identity"], true);
    assert_eq!(disabled["can_use_for_sandbox_mcp"], false);
}

#[test]
fn teammate_cross_project_and_unknown_explicit_identities_fail_closed() {
    let fixture = fixture();
    let before = snapshot(&fixture);
    for invalid_app_id in [
        fixture.teammate_app_id.as_str(),
        fixture.other_project_app_id.as_str(),
        "consumer.app-missing",
    ] {
        let error = call(
            &fixture,
            &fixture.project_id,
            &fixture.owner_id,
            invalid_app_id,
            false,
            json!({}),
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("不属于当前用户和项目，或该 App 已不存在"));
        assert!(!error.to_string().contains(&fixture.teammate_id));
    }
    assert_eq!(before, snapshot(&fixture));
}

#[test]
fn empty_directory_and_argument_contract_are_stable() {
    let fixture = fixture();
    let empty_owner = fixture
        .store
        .create_user(
            "consumer-app-empty@example.com",
            "secret1",
            Some("Empty Consumer"),
            None,
        )
        .unwrap();
    let empty_project = fixture
        .store
        .create_project(&empty_owner.id, "Empty Consumer Project", None, None)
        .unwrap()
        .project;
    let empty = call(
        &fixture,
        &empty_project.id,
        &empty_owner.id,
        "mcp-client",
        true,
        json!({}),
    )
    .unwrap();
    assert_eq!(empty["apps"], json!([]));

    let invalid_arguments = call(
        &fixture,
        &fixture.project_id,
        &fixture.owner_id,
        "mcp-client",
        true,
        json!({"include_tokens":true}),
    )
    .unwrap_err();
    assert!(invalid_arguments.to_string().contains("不接受参数"));
    assert!(super::call_if_handled(
        &fixture.store,
        &fixture.project_id,
        &fixture.owner_id,
        "mcp-client",
        true,
        "unknown_tool",
        json!({}),
    )
    .unwrap()
    .is_none());

    let definition = super::definitions().pop().unwrap();
    assert_eq!(definition["name"], LIST_APPS);
    assert_eq!(definition["annotations"]["readOnlyHint"], true);
    assert_eq!(definition["annotations"]["destructiveHint"], false);
    assert_eq!(definition["inputSchema"]["additionalProperties"], false);
    let initialize = crate::open_commerce_mcp_protocol::initialize_response();
    assert!(initialize["instructions"]
        .as_str()
        .unwrap()
        .contains(LIST_APPS));
}

fn app<'a>(apps: &'a [Value], app_id: &str) -> &'a Value {
    apps.iter()
        .find(|app| app["app_id"] == app_id)
        .expect("app should be present")
}

fn assert_no_keys(value: &Value, forbidden: &[&str]) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                assert!(!forbidden.contains(&key.as_str()), "leaked field {key}");
                assert_no_keys(child, forbidden);
            }
        }
        Value::Array(items) => {
            for item in items {
                assert_no_keys(item, forbidden);
            }
        }
        _ => {}
    }
}

fn snapshot(fixture: &Fixture) -> ReadSnapshot {
    let conn = fixture.store.conn().unwrap();
    let mut statement = conn
        .prepare(
            "SELECT id, status, updated_at
               FROM open_commerce_developer_apps ORDER BY id",
        )
        .unwrap();
    let app_rows = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    let audit_count = conn
        .query_row(
            "SELECT COUNT(*) FROM open_commerce_audit_events",
            [],
            |row| row.get(0),
        )
        .unwrap();
    ReadSnapshot {
        app_rows,
        audit_count,
    }
}
