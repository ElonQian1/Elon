//! Frozen commitment from each static terminal record to its typed descriptor semantics.
//!
//! The source-leaf ledger freezes the static record, while this compact root commitment freezes
//! which validated typed descriptor belongs to that record. Capability readiness is normalized
//! away so integrating a real runner does not rewrite this semantic authority.

use sha2::{Digest as _, Sha256};

use super::super::source_leaf_authority::{Digest32, FrozenStaticBindingV1, RootOperationV1};
use super::super::terminal_descriptor::CapabilityGapV1;
use super::{DynamicProjectionV1, StaticMemberSealV1, DYNAMIC_PROJECTOR_SCHEMA_V1};

const DESCRIPTOR_BINDING_DOMAIN_V1: &str = "ELON-A2-MAP-LOCK-DYNAMIC-DESCRIPTOR-BINDING-V1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct DescriptorBindingEntryV1 {
    pub(super) member: StaticMemberSealV1,
    pub(super) descriptor_semantic_sha256: Digest32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ValidatedDynamicTerminalV1 {
    pub(super) descriptor_binding: DescriptorBindingEntryV1,
    pub(super) semantic_key: super::DynamicClassKeyV1,
    pub(super) projection: Result<DynamicProjectionV1, CapabilityGapV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DescriptorBindingContextV1 {
    pub(super) root: RootOperationV1,
    pub(super) projector_schema_version: u16,
    pub(super) static_manifest_sha256: Digest32,
    pub(super) included_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FrozenDescriptorBindingAuthorityV1 {
    pub(super) context: DescriptorBindingContextV1,
    pub(super) descriptor_binding_sha256: Digest32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DescriptorBindingContextDriftV1 {
    pub(super) expected: DescriptorBindingContextV1,
    pub(super) actual: DescriptorBindingContextV1,
}

const MAP_STATIC_MANIFEST_SHA256: Digest32 = Digest32([
    0x0c, 0x51, 0xc3, 0xab, 0xe5, 0x2f, 0x1a, 0x4f, 0x5a, 0xd1, 0x21, 0x7c, 0x79, 0xeb, 0xd7, 0x39,
    0x31, 0x88, 0x45, 0x2f, 0xf0, 0x96, 0x59, 0x73, 0x9c, 0xa6, 0xe1, 0xd9, 0x3d, 0x20, 0x5c, 0x19,
]);
const LOCK_STATIC_MANIFEST_SHA256: Digest32 = Digest32([
    0xc6, 0x90, 0xc2, 0xf5, 0xb7, 0x8b, 0x68, 0x20, 0x1b, 0xd5, 0xc0, 0xea, 0xcd, 0x4e, 0x64, 0x89,
    0xe8, 0x7b, 0xb4, 0xc6, 0xab, 0xf8, 0xab, 0x58, 0x4a, 0xa2, 0x4e, 0x44, 0x37, 0x95, 0x49, 0x1e,
]);

const MAP_DESCRIPTOR_BINDING_SHA256: Digest32 = Digest32([
    0xd3, 0xba, 0x08, 0xa5, 0xba, 0x00, 0x19, 0xf9, 0xcc, 0xda, 0x99, 0xac, 0xe8, 0xb5, 0x80, 0xef,
    0x06, 0xeb, 0x4d, 0x66, 0x53, 0xba, 0x80, 0xc0, 0xdb, 0x54, 0x97, 0xbe, 0xc5, 0x1b, 0xd8, 0x70,
]);
const LOCK_DESCRIPTOR_BINDING_SHA256: Digest32 = Digest32([
    0x0c, 0xc9, 0x51, 0xc8, 0xc9, 0x79, 0x60, 0x8f, 0xb9, 0x86, 0x11, 0x67, 0xf8, 0xd8, 0x80, 0xa7,
    0x4f, 0xd2, 0xe0, 0x42, 0xc4, 0xd2, 0xcd, 0x42, 0x67, 0x31, 0x00, 0xe1, 0x40, 0x83, 0xe8, 0xef,
]);

const MAP_AUTHORITY: FrozenDescriptorBindingAuthorityV1 = FrozenDescriptorBindingAuthorityV1 {
    context: DescriptorBindingContextV1 {
        root: RootOperationV1::Map,
        projector_schema_version: DYNAMIC_PROJECTOR_SCHEMA_V1,
        static_manifest_sha256: MAP_STATIC_MANIFEST_SHA256,
        included_count: 43_476,
    },
    descriptor_binding_sha256: MAP_DESCRIPTOR_BINDING_SHA256,
};

const LOCK_AUTHORITY: FrozenDescriptorBindingAuthorityV1 = FrozenDescriptorBindingAuthorityV1 {
    context: DescriptorBindingContextV1 {
        root: RootOperationV1::Lock,
        projector_schema_version: DYNAMIC_PROJECTOR_SCHEMA_V1,
        static_manifest_sha256: LOCK_STATIC_MANIFEST_SHA256,
        included_count: 8_668,
    },
    descriptor_binding_sha256: LOCK_DESCRIPTOR_BINDING_SHA256,
};

pub(super) fn checked_in_authority_v1(
    binding: &FrozenStaticBindingV1,
) -> Result<FrozenDescriptorBindingAuthorityV1, DescriptorBindingContextDriftV1> {
    let expected = match binding.context.root {
        RootOperationV1::Map => MAP_AUTHORITY,
        RootOperationV1::Lock => LOCK_AUTHORITY,
    };
    let actual = DescriptorBindingContextV1 {
        root: binding.context.root,
        projector_schema_version: DYNAMIC_PROJECTOR_SCHEMA_V1,
        static_manifest_sha256: binding.static_manifest_sha256,
        included_count: binding.included_count,
    };
    if expected.context == actual {
        Ok(expected)
    } else {
        Err(DescriptorBindingContextDriftV1 {
            expected: expected.context,
            actual,
        })
    }
}

pub(super) fn digest_descriptor_binding_v1(
    context: DescriptorBindingContextV1,
    entries: impl IntoIterator<Item = DescriptorBindingEntryV1>,
) -> Digest32 {
    let mut entries = entries.into_iter().collect::<Vec<_>>();
    entries.sort_unstable();

    let mut out = Sha256::new();
    add_bytes(&mut out, "domain", DESCRIPTOR_BINDING_DOMAIN_V1.as_bytes());
    add_bytes(&mut out, "root", context.root.canonical_name().as_bytes());
    add_bytes(
        &mut out,
        "projector_schema_version",
        &context.projector_schema_version.to_be_bytes(),
    );
    add_bytes(
        &mut out,
        "static_manifest_sha256",
        &context.static_manifest_sha256.0,
    );
    add_bytes(
        &mut out,
        "included_count",
        &context.included_count.to_be_bytes(),
    );
    add_bytes(
        &mut out,
        "entry_count",
        &(entries.len() as u64).to_be_bytes(),
    );
    for entry in entries {
        let mut member = [0_u8; 64];
        member[..32].copy_from_slice(&entry.member.case_key_sha256.0);
        member[32..].copy_from_slice(&entry.member.full_record_sha256.0);
        add_bytes(&mut out, "member", &member);
        add_bytes(
            &mut out,
            "descriptor_semantic_sha256",
            &entry.descriptor_semantic_sha256.0,
        );
    }
    Digest32(out.finalize().into())
}

fn add_bytes(out: &mut Sha256, label: &str, value: &[u8]) {
    out.update(label.as_bytes());
    out.update([0]);
    out.update((value.len() as u64).to_be_bytes());
    out.update(value);
}

#[cfg(test)]
pub(super) fn authority_for_test(
    context: DescriptorBindingContextV1,
    entries: impl IntoIterator<Item = DescriptorBindingEntryV1>,
) -> FrozenDescriptorBindingAuthorityV1 {
    FrozenDescriptorBindingAuthorityV1 {
        context,
        descriptor_binding_sha256: digest_descriptor_binding_v1(context, entries),
    }
}
