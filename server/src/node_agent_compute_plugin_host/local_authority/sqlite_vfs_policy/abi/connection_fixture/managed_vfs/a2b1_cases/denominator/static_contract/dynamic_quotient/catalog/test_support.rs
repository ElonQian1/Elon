use super::*;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2b1_cases::denominator::static_contract::terminal_descriptor::{
    TerminalDescriptorV1,
};
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2b1_cases::denominator::static_contract::source_leaf_authority::{
    LeafRecordV1, LeafSealV1,
};

impl DynamicCatalogBuilderV1 {
    pub(crate) fn observe_synthetic_projection_for_test(
        &mut self,
        record: &LeafRecordV1,
        descriptor: &TerminalDescriptorV1,
        seal: &LeafSealV1,
    ) -> Result<(), CatalogErrorV1> {
        let member = self.register_terminal_member_for_test(seal)?;
        let validated = super::super::project_validated_dynamic_terminal_v1(record, descriptor)
            .map_err(|error| CatalogErrorV1::ProjectionFailed {
                count: 1,
                first: ProjectionFailureV1 { member, error },
            })?;
        self.observe_descriptor_binding(member, validated.descriptor_binding)?;
        self.observe_runner_admission(
            member,
            validated.descriptor_binding,
            validated.runner_admission,
        )?;
        // This helper exercises catalog/manifest structure only. It deliberately injects a
        // successful projection while retaining the planned-missing admission receipt and key;
        // no test fixture may manufacture a `Supported` admission decision.
        self.observe_projection(
            member,
            super::super::DynamicProjectionV1 {
                key: validated.semantic_key,
                class_key_sha256: super::super::digest_dynamic_class_key_v1(
                    &validated.semantic_key,
                ),
                member,
            },
        )
    }

    pub(crate) fn inject_runner_gap_for_test(
        &mut self,
        seal: &LeafSealV1,
        gap: CapabilityGapV1,
    ) -> Result<(), CatalogErrorV1> {
        let member = self.register_terminal_member_for_test(seal)?;
        self.projection_failures.push(ProjectionFailureV1 {
            member,
            error: ProjectionErrorV1::RunnerCapabilityMissing(gap),
        });
        Ok(())
    }

    fn register_terminal_member_for_test(
        &mut self,
        seal: &LeafSealV1,
    ) -> Result<StaticMemberSealV1, CatalogErrorV1> {
        if seal.root != self.root {
            return Err(CatalogErrorV1::RootMismatch);
        }
        if seal.outcome != LeafSealOutcomeV1::Terminal {
            return Err(CatalogErrorV1::OutcomeMismatch);
        }
        let member = StaticMemberSealV1 {
            case_key_sha256: seal.case_key_sha256,
            full_record_sha256: seal.full_record_sha256,
        };
        if self.excluded_members.contains(&member) {
            return Err(CatalogErrorV1::ExcludedMemberProjected(member));
        }
        if !self.static_members.insert(member) {
            return Err(CatalogErrorV1::DuplicateStaticMember(member));
        }
        Ok(member)
    }
}
