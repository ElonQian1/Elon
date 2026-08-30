//! Exact comparison between source-first authority expansion and graph DFS projection.

use std::collections::{BTreeMap, BTreeSet};

use super::model::{CaseKeyV1, LeafRecordV1};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RecordDiff {
    pub(crate) duplicate_authority_keys: Vec<CaseKeyV1>,
    pub(crate) duplicate_actual_keys: Vec<CaseKeyV1>,
    pub(crate) missing_keys: Vec<CaseKeyV1>,
    pub(crate) extra_keys: Vec<CaseKeyV1>,
    /// Same CaseKey, but ordered SourceBranch, Expected, terminal/excluded kind or proof changed.
    pub(crate) changed_keys: Vec<CaseKeyV1>,
}

impl RecordDiff {
    pub(crate) fn is_empty(&self) -> bool {
        self.duplicate_authority_keys.is_empty()
            && self.duplicate_actual_keys.is_empty()
            && self.missing_keys.is_empty()
            && self.extra_keys.is_empty()
            && self.changed_keys.is_empty()
    }
}

pub(crate) fn compare_exact_records(
    authority: &[LeafRecordV1],
    actual: &[LeafRecordV1],
) -> Result<(), RecordDiff> {
    let (authority_by_key, duplicate_authority_keys) = index(authority);
    let (actual_by_key, duplicate_actual_keys) = index(actual);
    let authority_keys = authority_by_key.keys().cloned().collect::<BTreeSet<_>>();
    let actual_keys = actual_by_key.keys().cloned().collect::<BTreeSet<_>>();
    let missing_keys = authority_keys
        .difference(&actual_keys)
        .cloned()
        .collect::<Vec<_>>();
    let extra_keys = actual_keys
        .difference(&authority_keys)
        .cloned()
        .collect::<Vec<_>>();
    let changed_keys = authority_keys
        .intersection(&actual_keys)
        .filter(|key| authority_by_key.get(*key) != actual_by_key.get(*key))
        .cloned()
        .collect::<Vec<_>>();
    let diff = RecordDiff {
        duplicate_authority_keys,
        duplicate_actual_keys,
        missing_keys,
        extra_keys,
        changed_keys,
    };
    if diff.is_empty() {
        Ok(())
    } else {
        Err(diff)
    }
}

fn index(records: &[LeafRecordV1]) -> (BTreeMap<CaseKeyV1, LeafRecordV1>, Vec<CaseKeyV1>) {
    let mut by_key = BTreeMap::new();
    let mut duplicates = BTreeSet::new();
    for record in records {
        if by_key.insert(record.key.clone(), record.clone()).is_some() {
            duplicates.insert(record.key.clone());
        }
    }
    (by_key, duplicates.into_iter().collect())
}
