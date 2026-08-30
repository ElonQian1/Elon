use super::expected::ExpectedV1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RootOperationV1 {
    Map,
    Lock,
}

impl RootOperationV1 {
    pub(crate) const fn canonical_name(self) -> &'static str {
        match self {
            Self::Map => "map",
            Self::Lock => "lock",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Digest32(pub(crate) [u8; 32]);

impl Digest32 {
    pub(crate) const ZERO: Self = Self([0; 32]);

    pub(crate) fn to_lower_hex(self) -> String {
        let mut result = String::with_capacity(64);
        for byte in self.0 {
            use std::fmt::Write as _;
            write!(&mut result, "{byte:02x}").expect("writing to String cannot fail");
        }
        result
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CoordinateV1 {
    pub(crate) name: String,
    pub(crate) value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct LeafIdentityV1 {
    pub(crate) root: RootOperationV1,
    /// Stable source-leaf identity. It is not a graph node id, even if an initial adapter uses the
    /// same textual spelling while the two identities are separated.
    pub(crate) leaf_id: String,
    pub(crate) family_id: String,
    pub(crate) coordinates: Vec<CoordinateV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DecisionStageV1 {
    AbiValidation,
    RawAdmission,
    RawAbandon,
    Adapter,
    CallbackAdmission,
    ManagedRequest,
    Initialization,
    Coordination,
    NativeCall,
    Cleanup,
    Quarantine,
    CallbackCompletion,
    AbiProjection,
}

impl DecisionStageV1 {
    pub(crate) const fn canonical_name(self) -> &'static str {
        match self {
            Self::AbiValidation => "abi-validation",
            Self::RawAdmission => "raw-admission",
            Self::RawAbandon => "raw-abandon",
            Self::Adapter => "adapter",
            Self::CallbackAdmission => "callback-admission",
            Self::ManagedRequest => "managed-request",
            Self::Initialization => "initialization",
            Self::Coordination => "coordination",
            Self::NativeCall => "native-call",
            Self::Cleanup => "cleanup",
            Self::Quarantine => "quarantine",
            Self::CallbackCompletion => "callback-completion",
            Self::AbiProjection => "abi-projection",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DecisionV1 {
    pub(crate) stage: DecisionStageV1,
    pub(crate) branch: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CaseKeyV1 {
    pub(crate) identity: LeafIdentityV1,
    pub(crate) decisions: Vec<DecisionV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SourceWitnessV1 {
    pub(crate) owner_id: String,
    pub(crate) symbol: String,
    pub(crate) needle: String,
    /// Occurrence is interpreted inside the reviewed symbol span, never across the whole file.
    pub(crate) occurrence: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ExclusionKindV1 {
    TypeInvariant,
    ControlFlow,
    SafetyPremise,
}

impl ExclusionKindV1 {
    pub(crate) const fn canonical_name(self) -> &'static str {
        match self {
            Self::TypeInvariant => "type-invariant",
            Self::ControlFlow => "control-flow",
            Self::SafetyPremise => "safety-premise",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ExclusionProofV1 {
    pub(crate) kind: ExclusionKindV1,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum LeafOutcomeV1 {
    Terminal(ExpectedV1),
    Excluded(ExclusionProofV1),
}

impl LeafOutcomeV1 {
    pub(crate) const fn canonical_name(&self) -> &'static str {
        match self {
            Self::Terminal(_) => "terminal",
            Self::Excluded(_) => "excluded",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct LeafRecordV1 {
    pub(crate) key: CaseKeyV1,
    pub(crate) source_branch: Vec<SourceWitnessV1>,
    pub(crate) outcome: LeafOutcomeV1,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SourceScopeFileV1 {
    pub(crate) owner_id: String,
    pub(crate) repo_relative_path: String,
    pub(crate) git_blob_oid_sha1: String,
    pub(crate) normalized_lf_sha256: String,
    pub(crate) symbol_sentinels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManifestContextV1 {
    pub(crate) schema: String,
    pub(crate) root: RootOperationV1,
    pub(crate) target_scope: String,
    pub(crate) source_baseline_commit_sha1: String,
    pub(crate) source_scope_sha256: Digest32,
    pub(crate) ledger_sha256: Digest32,
    pub(crate) map_profile_set_sha256: Option<Digest32>,
    pub(crate) map_ordinal_domain_sha256: Option<Digest32>,
    pub(crate) lock_range_set_sha256: Option<Digest32>,
    pub(crate) lock_range_count: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShardManifestV1 {
    pub(crate) index: u8,
    pub(crate) included_count: u64,
    pub(crate) excluded_count: u64,
    pub(crate) source_leaf_identity_set_sha256: Digest32,
    pub(crate) case_key_set_sha256: Digest32,
    pub(crate) source_branch_map_sha256: Digest32,
    pub(crate) expected_map_sha256: Digest32,
    pub(crate) exclusion_map_sha256: Digest32,
    pub(crate) full_record_set_sha256: Digest32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RootManifestV1 {
    pub(crate) context: ManifestContextV1,
    pub(crate) included_count: u64,
    pub(crate) excluded_count: u64,
    pub(crate) source_leaf_identity_set_sha256: Digest32,
    pub(crate) case_key_set_sha256: Digest32,
    pub(crate) source_branch_map_sha256: Digest32,
    pub(crate) expected_map_sha256: Digest32,
    pub(crate) exclusion_map_sha256: Digest32,
    pub(crate) full_record_set_sha256: Digest32,
    pub(crate) shards: Vec<ShardManifestV1>,
    /// Digest of every preceding manifest field. This field is excluded from its own digest.
    pub(crate) manifest_sha256: Digest32,
}
