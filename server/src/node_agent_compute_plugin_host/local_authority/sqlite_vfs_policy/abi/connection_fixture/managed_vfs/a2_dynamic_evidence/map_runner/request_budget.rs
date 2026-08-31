//! Exact native request inputs for each admitted Map managed-budget guard.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) enum MapRunnerRequestBudgetV1 {
    RegionSize,
    RegionCount,
    LogicalSize,
}

impl MapRunnerRequestBudgetV1 {
    pub(super) const fn selector(self) -> &'static str {
        match self {
            Self::RegionSize => "region-size-budget-completed",
            Self::RegionCount => "region-count-budget-completed",
            Self::LogicalSize => "logical-size-budget-completed",
        }
    }

    pub(super) const fn tag(self) -> u64 {
        match self {
            Self::RegionSize => 1,
            Self::RegionCount => 2,
            Self::LogicalSize => 3,
        }
    }

    pub(super) const fn region(self) -> i32 {
        match self {
            Self::RegionSize => 0,
            Self::RegionCount => 256,
            Self::LogicalSize => 255,
        }
    }

    pub(super) const fn region_size(self) -> i32 {
        match self {
            Self::RegionSize => 65_537,
            Self::RegionCount => 32_768,
            Self::LogicalSize => 65_536,
        }
    }
}
