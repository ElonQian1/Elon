use super::batch_accept::{validate_batch_request, BatchAcceptRequest};

#[test]
fn batch_request_requires_unique_non_empty_runs() {
    let empty = request(Vec::new());
    assert!(validate_batch_request(&empty).is_err());

    let duplicate = request(vec!["fit_1".into(), "fit_1".into()]);
    assert!(validate_batch_request(&duplicate).is_err());

    let valid = request(vec!["fit_1".into(), "fit_2".into()]);
    assert!(validate_batch_request(&valid).is_ok());
}

#[test]
fn batch_request_requires_source_revision_even_before_codex() {
    let mut value = request(vec!["fit_1".into()]);
    value.source_revision.clear();
    assert!(validate_batch_request(&value).is_err());
}

fn request(run_ids: Vec<String>) -> BatchAcceptRequest {
    BatchAcceptRequest {
        run_ids,
        source_revision: "source-v1".into(),
        codex_completed: false,
    }
}
