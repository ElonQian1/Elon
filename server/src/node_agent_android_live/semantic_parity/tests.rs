//! Synthetic fixtures exercise validators only; they are never business evidence.
use super::{contract, evidence, signing, validate_native};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::PathBuf, process::Command};
const KEY: &[u8] = b"isolated-test-key-never-a-node-credential";
fn revision() -> String {
    format!("workspace-sha256:{}", "a".repeat(64))
}

fn contract_value() -> Value {
    json!({"schemaVersion":1,"taskId":"test-task","scope":"OBSERVED_CAPABILITY_STATE",
    "designSystem":{"id":"mobile-design-v2","sourceFile":"design.md","sha256":contract::digest(b"design-v2")},
    "requirementSpec":{"sourceFile":"requirements.md","sha256":contract::digest(b"all states required")},
    "platformDifferences":["Platform native layout"],"states":[{
        "id":"login","androidScreenId":"screen-login","androidFontScale":1.0,
        "webCapture":{"url":"http://127.0.0.1:3000/login","viewport":{"width":411,"height":842},"steps":[],
            "expectedPage":{"kind":"PUBLIC_LOGIN","pageId":"login","path":"/login","readySelector":"#loginView"}},
        "capabilities":[{"id":"submit","state":"visible login button",
            "android":{"definitionId":"submit","assertions":{"/kind":"Button","/text":"登录"}},
            "web":{"selector":"#submit","assertions":{"/role":"button","/label":"登录","/disabled":false}}}]
    }]})
}
fn native_nodes() -> Vec<Value> {
    vec![
        json!({"runtimeNodeId":"runtime-submit","definitionId":"submit","resourceId":"submit","screenId":"screen-login","kind":"Button","text":"登录",
        "geometry":{"visible":true,"fontScale":1.0,"boundsInDisplayPx":{"width":90,"height":40}}}),
    ]
}
fn web_tree() -> Value {
    json!({"schema":"elon.web.semantic-tree.v1","truncated":false,"route":"/login",
        "nodes":[{"selector":"#submit","role":"button","label":"登录","disabled":false,"bounds":{"width":90,"height":40}}]})
}
fn proof() -> Value {
    json!({"runtime":{"connected":true,"historyCount":0,"redoCount":0,"nodeCount":1,"deviceId":"emulator-5554",
        "packageName":"test.ui","runtimeBuildId":"build-1","debugProjectId":"project-1","deviceIdentity":"emulator-5554",
        "sourceProof":{"generation":1,"runtimeBuildId":"build-1","originWorkspaceRevision":revision(),"sourceParityLoss":0.0,
            "sourceRevision":"git-1","integrationRevision":"integration-1","generationRevision":"generation-1"}},
        "integration":{"status":"DEPLOYED","desiredGeneration":1,"installedGeneration":1,"projectId":"project-1",
            "deviceIdentity":"emulator-5554","packageName":"test.ui","sourceRevision":"git-1","integrationRevision":"integration-1"}})
}

struct Fixture {
    root: PathBuf,
    task: PathBuf,
    contract: contract::Contract,
    hash: String,
    envelope: Value,
}
impl Fixture {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("elon-semantic-test-{}", uuid::Uuid::new_v4()));
        let task = root.join(".elon/ui-design/tasks/test-task");
        std::fs::create_dir_all(task.join("evidence")).unwrap();
        std::fs::write(root.join("design.md"), "design-v2").unwrap();
        std::fs::write(root.join("requirements.md"), "all states required").unwrap();
        let bytes = serde_json::to_vec(&contract_value()).unwrap();
        std::fs::write(root.join("contract.json"), &bytes).unwrap();
        for args in [
            vec!["init", "--quiet"],
            vec!["add", "design.md", "requirements.md", "contract.json"],
            vec![
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ],
        ] {
            assert!(Command::new("git")
                .current_dir(&root)
                .args(args)
                .output()
                .unwrap()
                .status
                .success());
        }
        let (contract, hash) = contract::load(&root, "contract.json", "test-task").unwrap();
        let mut fixture = Self {
            root,
            task,
            contract,
            hash,
            envelope: Value::Null,
        };
        fixture.envelope = fixture.receipt();
        fixture
    }
    fn artifact(&self, name: &str, bytes: &[u8]) -> evidence::Artifact {
        let path = format!("evidence/{name}");
        std::fs::write(self.task.join(&path), bytes).unwrap();
        evidence::Artifact {
            path,
            sha256: contract::digest(bytes),
        }
    }
    fn receipt(&self) -> Value {
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image::RgbImage::new(16, 16))
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let mut artifacts = BTreeMap::new();
        artifacts.insert("androidImage", self.artifact("android.png", png.get_ref()));
        artifacts.insert("webImage", self.artifact("web.png", png.get_ref()));
        artifacts.insert(
            "androidTree",
            self.artifact(
                "android.json",
                &serde_json::to_vec(&native_nodes()).unwrap(),
            ),
        );
        artifacts.insert(
            "webTree",
            self.artifact("web.json", &serde_json::to_vec(&web_tree()).unwrap()),
        );
        let manifest = json!({"schema":"elon.pwa.runtime-capture.v1","revision":{"sourceRevision":revision(),"routeRevision":format!("semantic:{}:login",self.hash)},
            "artifact":{"sha256":artifacts["webImage"].sha256},"semanticTree":{"sha256":artifacts["webTree"].sha256},
            "expectedPage":self.contract.states[0].web_capture["expectedPage"],"fixtureProfile":null,"authenticationMode":"none",
            "route":{"origin":"http://127.0.0.1:3000","path":"/login"}});
        artifacts.insert(
            "webManifest",
            self.artifact("manifest.json", &serde_json::to_vec(&manifest).unwrap()),
        );
        let payload = json!({"schemaVersion":1,"taskId":"test-task","stateId":"login","runId":"run1","projectRoot":self.root,
            "sourceRevision":revision(),"contractSha256":self.hash,"capturedAt":chrono::Utc::now().to_rfc3339(),"nativeProof":proof(),
            "webAuthentication":{"mode":"none","profile":null},"artifacts":artifacts,
            "result":contract::evaluate(&self.contract.states[0], &native_nodes(), &web_tree()).unwrap()});
        json!({"signature":signing::sign(&payload,KEY).unwrap(),"payload":payload})
    }
    fn verify(&self, envelope: &Value) -> anyhow::Result<Value> {
        evidence::verify_state(
            &self.task,
            envelope,
            &self.contract,
            &self.hash,
            &self.contract.states[0],
            &revision(),
            "run1",
            KEY,
        )
    }
    fn summary(&self) -> Value {
        json!({"schemaVersion":3,"verificationMode":"SEMANTIC_PARITY","taskId":"test-task","sourceRevision":revision(),
            "contractPath":"contract.json","contractSha256":self.hash,"runId":"run1","stateReceipts":{"login":"evidence/receipt.json"}})
    }
    fn summary_result(&self, value: &Value) -> anyhow::Result<Value> {
        std::fs::write(
            self.task.join("evidence/receipt.json"),
            serde_json::to_vec(&self.envelope).unwrap(),
        )
        .unwrap();
        let path = self.task.join("cross-platform-verification.json");
        std::fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
        evidence::verify_summary(&path, "test-task", &revision(), KEY)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn rejects_distinct_selectors_reusing_the_same_native_node() {
    let mut value = contract_value();
    let mut duplicate = value["states"][0]["capabilities"][0].clone();
    duplicate["id"] = json!("other-submit");
    duplicate["android"] =
        json!({"resourceId":"submit","assertions":{"/kind":"Button","/text":"登录"}});
    duplicate["web"]["selector"] = json!("#other-submit");
    value["states"][0]["capabilities"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    let contract: contract::Contract = serde_json::from_value(value).unwrap();
    assert!(
        contract::evaluate(&contract.states[0], &native_nodes(), &web_tree())
            .unwrap_err()
            .to_string()
            .contains("NODE_REUSED")
    );
}

#[test]
fn observes_all_declared_capabilities_without_pixel_loss() {
    let fixture = Fixture::new();
    assert!(fixture.verify(&fixture.envelope).is_ok());
    let result = fixture.summary_result(&fixture.summary()).unwrap();
    assert_eq!(result["status"], "PASSED");
    assert!(result.get("visualLoss").is_none());
}
#[test]
fn rejects_missing_states_and_forged_pixel_metric() {
    let fixture = Fixture::new();
    let mut summary = fixture.summary();
    summary["stateReceipts"] = json!({});
    assert!(fixture
        .summary_result(&summary)
        .unwrap_err()
        .to_string()
        .contains("INCOMPLETE"));
    let mut summary = fixture.summary();
    summary["visualLoss"] = json!(0);
    assert!(fixture.summary_result(&summary).is_err());
}
#[test]
fn rejects_unsigned_tampered_or_cross_task_receipts() {
    let fixture = Fixture::new();
    let mut envelope = fixture.envelope.clone();
    envelope["payload"]["result"] = json!("PASSED");
    assert!(fixture
        .verify(&envelope)
        .unwrap_err()
        .to_string()
        .contains("SIGNATURE"));
    envelope["signature"] = json!(signing::sign(&envelope["payload"], b"wrong-node-key").unwrap());
    assert!(fixture.verify(&envelope).is_err());
    envelope = fixture.envelope.clone();
    envelope["payload"]["taskId"] = json!("other-task");
    envelope["signature"] = json!(signing::sign(&envelope["payload"], KEY).unwrap());
    assert!(fixture.verify(&envelope).is_err());
}
#[test]
fn rejects_artifact_tampering_path_escape_and_expired_receipt() {
    let fixture = Fixture::new();
    std::fs::write(fixture.task.join("evidence/android.png"), b"changed").unwrap();
    assert!(fixture
        .verify(&fixture.envelope)
        .unwrap_err()
        .to_string()
        .contains("ARTIFACT_CHANGED"));
    let mut receipt = fixture.envelope.clone();
    receipt["payload"]["capturedAt"] = json!("2000-01-01T00:00:00Z");
    receipt["signature"] = json!(signing::sign(&receipt["payload"], KEY).unwrap());
    assert!(fixture
        .verify(&receipt)
        .unwrap_err()
        .to_string()
        .contains("EXPIRED"));
    assert!(evidence::read_bytes(&fixture.task, "../../contract.json", 1024).is_err());
}
#[test]
fn rejects_missing_duplicate_wrong_state_font_and_disabled_nodes() {
    let contract: contract::Contract = serde_json::from_value(contract_value()).unwrap();
    let state = &contract.states[0];
    assert!(contract::evaluate(state, &[], &web_tree()).is_err());
    assert!(contract::evaluate(
        state,
        &[native_nodes()[0].clone(), native_nodes()[0].clone()],
        &web_tree()
    )
    .is_err());
    for pointer in [
        "/screenId",
        "/geometry/visible",
        "/geometry/fontScale",
        "/text",
    ] {
        let mut nodes = native_nodes();
        *nodes[0].pointer_mut(pointer).unwrap() = match pointer {
            "/geometry/visible" => json!(false),
            "/geometry/fontScale" => json!(2.0),
            _ => json!("wrong"),
        };
        assert!(
            contract::evaluate(state, &nodes, &web_tree()).is_err(),
            "{pointer}"
        );
    }
    let mut tree = web_tree();
    tree["nodes"][0]["disabled"] = json!(true);
    assert!(contract::evaluate(state, &native_nodes(), &tree).is_err());
    tree = web_tree();
    tree["truncated"] = json!(true);
    assert!(contract::evaluate(state, &native_nodes(), &tree).is_err());
}
#[test]
fn rejects_empty_reduced_uncommitted_or_mutating_contracts() {
    for (pointer, value) in [
        ("/states", json!([])),
        ("/scope", json!("ALL_FUNCTIONS_PROVEN")),
        ("/states/0/capabilities", json!([])),
        (
            "/states/0/capabilities/0/android/assertions",
            json!({"/kind":"Button"}),
        ),
        (
            "/states/0/webCapture/steps",
            json!([{"action":"click","selector":"#submit"}]),
        ),
    ] {
        let mut value_all = contract_value();
        *value_all
            .pointer_mut(pointer)
            .unwrap_or_else(|| panic!("{pointer}")) = value;
        let parsed: contract::Contract = serde_json::from_value(value_all).unwrap();
        assert!(contract::validate(&parsed, "test-task").is_err());
    }
    let fixture = Fixture::new();
    std::fs::write(fixture.root.join("contract.json"), b"{}").unwrap();
    assert!(contract::load(&fixture.root, "contract.json", "test-task").is_err());
    assert!(contract::tracked_bytes(&fixture.root, "../contract.json", 1024).is_err());
}
#[test]
fn missing_source_proof_and_stale_generation_never_pass() {
    let good = proof();
    validate_native(&good["runtime"], &good["integration"], &revision()).unwrap();
    for (pointer, value) in [
        ("/runtime/sourceProof", Value::Null),
        ("/runtime/historyCount", json!(1)),
        ("/runtime/redoCount", json!(1)),
        ("/runtime/runtimeBuildId", json!("other")),
        ("/integration/installedGeneration", json!(2)),
        ("/integration/status", json!("BUILDING")),
        (
            "/runtime/sourceProof/originWorkspaceRevision",
            json!("stale"),
        ),
    ] {
        let mut bad = good.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        assert!(
            validate_native(&bad["runtime"], &bad["integration"], &revision()).is_err(),
            "{pointer}"
        );
    }
}
