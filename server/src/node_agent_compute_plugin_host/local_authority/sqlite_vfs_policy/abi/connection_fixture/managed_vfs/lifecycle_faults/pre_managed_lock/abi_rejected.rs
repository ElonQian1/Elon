//! Consuming no-entry receipt shared by the q10 ABI rejection path and ordinary snapshots.

use super::*;

pub(super) fn finish(
    state: &mut ManagedTestPreManagedLockState,
    route: ManagedTestRouteOrdinal,
) -> Result<ManagedTestPreManagedLockSnapshot, &'static str> {
    if state.entries.get(&route).map(|entry| entry.path)
        != Some(ManagedTestPreManagedLockPath::AbiRejected)
    {
        return Err("ABI-rejected Lock observation route/path mismatch");
    }
    let entry = state
        .entries
        .remove(&route)
        .ok_or("ABI-rejected Lock observation was not armed")?;
    snapshot_entry(&entry)
}

pub(super) fn snapshot_entry(
    entry: &Entry,
) -> Result<ManagedTestPreManagedLockSnapshot, &'static str> {
    let (custody, shm_present, rejection, managed_reached) =
        entry
            .dispatch
            .unwrap_or((Custody::Sidecar, false, None, false));
    let receipt = entry
        .receipt
        .map(PreemptionReceipt::ordered_values)
        .unwrap_or([0; 4]);
    let values = [
        1,
        entry.path.tag(),
        entry.entry_count,
        entry.admission.is_some() as u64,
        admission_tag(entry.admission),
        entry.dispatch.is_some() as u64,
        custody_tag(entry.dispatch.map(|_| custody)),
        shm_present as u64,
        rejection_tag(rejection),
        managed_reached as u64,
        entry.completion.is_some() as u64,
        completion_tag(entry.completion),
        entry.claim_count,
        receipt[0],
        receipt[1],
        receipt[2],
        receipt[3],
        entry.violations,
    ];
    validate_snapshot(entry.path, values)?;
    Ok(ManagedTestPreManagedLockSnapshot { values })
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU8;

    use super::*;
    use crate::node_agent_managed_fs::ManagedSqliteShmLockAction;

    fn request(first: u8) -> ManagedSqliteShmLockRequest {
        ManagedSqliteShmLockRequest::new(
            first,
            NonZeroU8::new(1).unwrap(),
            ManagedSqliteShmLockAction::LockShared,
        )
        .unwrap()
    }

    #[test]
    fn no_entry_receipt_is_exact_consuming_and_event_sensitive() {
        let route = ManagedTestRouteOrdinal::test_value(6);
        let mut clean = ManagedTestPreManagedLockState::default();
        clean
            .arm(
                route,
                ManagedTestPreManagedLockPath::AbiRejected,
                request(0),
            )
            .unwrap();
        assert_eq!(
            finish(&mut clean, route).unwrap().ordered_values(),
            [1, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert!(finish(&mut clean, route).is_err());

        for observed in [request(0), request(1)] {
            let mut contaminated = ManagedTestPreManagedLockState::default();
            contaminated
                .arm(
                    route,
                    ManagedTestPreManagedLockPath::AbiRejected,
                    request(0),
                )
                .unwrap();
            contaminated.observe(route, Event::Entry { request: observed });
            assert!(finish(&mut contaminated, route).is_err());
        }
    }
}
