#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
    LockShared,
    LockExclusive,
    UnlockShared,
    UnlockExclusive,
}

impl Action {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::LockShared => "lock-shared",
            Self::LockExclusive => "lock-exclusive",
            Self::UnlockShared => "unlock-shared",
            Self::UnlockExclusive => "unlock-exclusive",
        }
    }

    pub(super) const fn is_shared(self) -> bool {
        matches!(self, Self::LockShared | Self::UnlockShared)
    }

    pub(super) const fn mode(self) -> LockMode {
        if self.is_shared() {
            LockMode::Shared
        } else {
            LockMode::Exclusive
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct RangeCell {
    pub(super) first: u8,
    pub(super) count: u8,
}

impl RangeCell {
    pub(super) fn mask(self) -> u8 {
        derived_mask(self.first, self.count)
    }

    pub(super) fn label(self) -> String {
        format!(
            "first{}.count{}.mask{:02x}",
            self.first,
            self.count,
            self.mask()
        )
    }
}

pub(super) fn representatives(action: Action) -> Vec<RangeCell> {
    if action.is_shared() {
        (0..8).map(|first| RangeCell { first, count: 1 }).collect()
    } else {
        (1..=8)
            .flat_map(|count| (0..=(8 - count)).map(move |first| RangeCell { first, count }))
            .collect()
    }
}

/// Proves that the graph materializes every concrete SQLite byte range. `first` is not quotiented:
/// production sends it to the native byte offset and retains it in the exclusive range table.
pub(super) fn validate_translation_quotient() {
    let shared = representatives(Action::LockShared);
    assert_eq!(shared.len(), 8);
    assert_eq!(shared, representatives(Action::UnlockShared));
    assert!(shared.iter().all(|cell| cell.mask().count_ones() == 1));
    assert!(shared.contains(&RangeCell { first: 0, count: 1 }));
    assert!(shared.contains(&RangeCell { first: 7, count: 1 }));

    let exclusive = representatives(Action::LockExclusive);
    assert_eq!(exclusive.len(), 36);
    assert_eq!(exclusive, representatives(Action::UnlockExclusive));
    assert!(exclusive
        .iter()
        .all(|cell| cell.mask() == derived_mask(cell.first, cell.count)));
    assert!(exclusive.contains(&RangeCell { first: 0, count: 8 }));
    assert!(exclusive.contains(&RangeCell { first: 7, count: 1 }));
    for width in 1..=8 {
        assert_eq!(
            exclusive.iter().filter(|cell| cell.count == width).count(),
            usize::from(9 - width)
        );
    }
}

fn derived_mask(first: u8, count: u8) -> u8 {
    assert!(count != 0 && first.checked_add(count).is_some_and(|end| end <= 8));
    let low = 1_u16 << first;
    let high = 1_u16 << (first + count);
    (high - low) as u8
}
use super::super::model::LockMode;
