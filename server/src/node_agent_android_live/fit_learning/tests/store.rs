use std::fs;

use serde_json::Value;

use super::super::promotion::{promote_priors, FitPromotionPolicy};
use super::super::store::FitLearningStore;
use super::{fit_case, MockEvaluator};

#[test]
fn store_retains_negative_evidence_and_atomically_updates_priors() {
    let root = std::env::temp_dir().join(format!(
        "elon-fit-learning-{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).unwrap();
    let store = FitLearningStore::new(&root).unwrap();
    let accepted = fit_case("1", "checkout.pay", "checkout", 4.0, true);
    let rejected = fit_case("2", "checkout.pay", "checkout", 8.0, false);
    store.record_case(accepted.clone()).unwrap();
    let cases = store.record_case(rejected).unwrap();
    assert_eq!(cases.cases.len(), 2);
    assert!(cases.cases.iter().any(|case| !case.promotable));

    let result = promote_priors(
        &cases.cases,
        &[],
        &MockEvaluator { regress: false },
        &FitPromotionPolicy::default(),
    )
    .unwrap();
    store.save_priors(&result.document).unwrap();
    let loaded = store.load_priors().unwrap();
    assert!(!loaded.priors.is_empty());
    assert!(store
        .priors_path()
        .ends_with(".elon/ui-standards/fit-priors.v1.json"));
    assert_eq!(store.load_cases().unwrap().cases.len(), 2);
    let cases_text = fs::read_to_string(store.cases_path()).unwrap();
    assert!(!cases_text.contains("D:/project"));
    assert!(cases_text.contains("\"projectRoot\": \".\""));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn fixed_learning_backup_recovers_corrupt_primary() {
    let root = std::env::temp_dir().join(format!(
        "elon-fit-learning-backup-{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).unwrap();
    let store = FitLearningStore::new(&root).unwrap();
    store
        .record_case(fit_case("1", "checkout.pay", "checkout", 4.0, true))
        .unwrap();
    store
        .record_case(fit_case("2", "profile.save", "profile", 6.0, true))
        .unwrap();
    let cases_path = store.cases_path();
    let backup_path = cases_path.with_file_name("fit-cases.v1.json.bak");
    assert!(backup_path.is_file());
    fs::write(&cases_path, b"corrupt-primary").unwrap();
    let recovered = store.load_cases().unwrap();
    assert_eq!(recovered.cases.len(), 1);
    assert!(serde_json::from_str::<Value>(&fs::read_to_string(&cases_path).unwrap()).is_ok());

    let promoted = promote_priors(
        &recovered.cases,
        &[],
        &MockEvaluator { regress: false },
        &FitPromotionPolicy::default(),
    )
    .unwrap()
    .document;
    store.save_priors(&promoted).unwrap();
    store.save_priors(&promoted).unwrap();
    let priors_path = store.priors_path();
    assert!(priors_path
        .with_file_name("fit-priors.v1.json.bak")
        .is_file());
    fs::write(&priors_path, b"corrupt-primary").unwrap();
    assert!(!store.load_priors().unwrap().priors.is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn artifact_provenance_is_project_relative_or_removed() {
    let root = std::env::temp_dir().join(format!(
        "elon-fit-learning-paths-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let frame = root.join(".elon/ui-tuner/fit-runs/fit_1/frames/final.png");
    fs::create_dir_all(frame.parent().unwrap()).unwrap();
    fs::write(&frame, b"png").unwrap();
    let mut case = fit_case("1", "checkout.pay", "checkout", 4.0, true);
    case.project_root = root.display().to_string();
    case.provenance.final_screenshot_path = Some(frame.display().to_string());
    let outside = root.parent().unwrap().join("outside/diff.json");
    case.provenance.final_diff_artifact_path = Some(outside.display().to_string());
    let store = FitLearningStore::new(&root).unwrap();
    let document = store.record_case(case).unwrap();
    let persisted = &document.cases[0];
    assert_eq!(persisted.project_root, ".");
    assert_eq!(
        persisted.provenance.final_screenshot_path.as_deref(),
        Some(".elon/ui-tuner/fit-runs/fit_1/frames/final.png")
    );
    assert!(persisted.provenance.final_diff_artifact_path.is_none());
    let text = fs::read_to_string(store.cases_path()).unwrap();
    assert!(!text.contains(&root.display().to_string()));
    assert!(!text.contains(&outside.display().to_string()));
    fs::remove_dir_all(root).unwrap();
}
