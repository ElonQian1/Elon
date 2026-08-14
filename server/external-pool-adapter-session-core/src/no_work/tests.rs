use std::{thread, time::Duration};

use crate::{
    prepare_external_pool_adapter_supervisor_session, AuthenticatedExternalPoolAdapterSession,
    ExternalPoolAdapterSessionRoots,
};

use super::{
    execute_external_pool_adapter_no_work_probe,
    receive_external_pool_adapter_no_work_probe_request,
};

const REQUEST: &[u8] = b"ELON-TEST-NO-WORK\n";
const RESPONSE: &[u8] = b"ELON-TEST-NO-TASK\n";
const TIMEOUT: Duration = Duration::from_millis(15_000);

#[test]
fn exact_response_produces_one_child_validated_receipt() {
    let (mut host, mut child) = authenticated_pair();
    let child_thread = thread::spawn(move || {
        execute_external_pool_adapter_no_work_probe(
            &mut child,
            REQUEST,
            RESPONSE.len(),
            TIMEOUT,
            |response| {
                if response != RESPONSE {
                    anyhow::bail!("test response rejected");
                }
                Ok(())
            },
        )
    });
    let request = receive_external_pool_adapter_no_work_probe_request(&mut host, TIMEOUT)
        .expect("receive no-work request");
    assert_eq!(request.request(), REQUEST);
    let receipt = request
        .complete(&mut host, RESPONSE)
        .expect("complete exact response");
    assert_eq!(receipt.request_bytes(), REQUEST.len() as u32);
    assert_eq!(receipt.response_bytes(), RESPONSE.len() as u32);
    assert_eq!(receipt.probe_root_hex().len(), 64);
    child_thread
        .join()
        .expect("join child")
        .expect("child validates response");
}

#[test]
fn wrong_length_and_semantic_rejection_are_terminal() {
    let (mut host, mut child) = authenticated_pair();
    let child_thread = thread::spawn(move || {
        execute_external_pool_adapter_no_work_probe(
            &mut child,
            REQUEST,
            RESPONSE.len(),
            TIMEOUT,
            |_| Ok(()),
        )
    });
    let request = receive_external_pool_adapter_no_work_probe_request(&mut host, TIMEOUT)
        .expect("receive wrong-length request");
    assert!(request.complete(&mut host, b"short").is_err());
    assert!(child_thread
        .join()
        .expect("join wrong-length child")
        .is_err());

    let (mut host, mut child) = authenticated_pair();
    let child_thread = thread::spawn(move || {
        execute_external_pool_adapter_no_work_probe(
            &mut child,
            REQUEST,
            RESPONSE.len(),
            TIMEOUT,
            |_| anyhow::bail!("semantic rejection"),
        )
    });
    let request = receive_external_pool_adapter_no_work_probe_request(&mut host, TIMEOUT)
        .expect("receive semantic request");
    assert!(request.complete(&mut host, RESPONSE).is_err());
    assert!(child_thread.join().expect("join semantic child").is_err());
}

fn authenticated_pair() -> (
    AuthenticatedExternalPoolAdapterSession,
    AuthenticatedExternalPoolAdapterSession,
) {
    let roots = roots();
    let prepared = prepare_external_pool_adapter_supervisor_session(roots.clone())
        .expect("prepare test session");
    let (host, child) = prepared.split();
    let child_thread = thread::spawn(move || child.authenticate(roots));
    let host = host.authenticate().expect("authenticate host");
    let child = child_thread
        .join()
        .expect("join bootstrap child")
        .expect("authenticate child");
    (host, child)
}

fn roots() -> ExternalPoolAdapterSessionRoots {
    ExternalPoolAdapterSessionRoots::new(
        &"11".repeat(32),
        &"22".repeat(32),
        &"33".repeat(32),
        &"44".repeat(32),
        &"55".repeat(32),
        &"66".repeat(32),
    )
    .expect("construct test roots")
}
