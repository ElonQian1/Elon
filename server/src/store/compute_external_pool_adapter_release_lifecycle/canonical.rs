use anyhow::Result;

use crate::compute_federation::external_pool_adapter_release_lifecycle::{
    canonical_external_pool_adapter_release_admission_terminal_json_and_digest,
    canonical_external_pool_adapter_release_admission_terminal_request_digest,
    ComputeExternalPoolAdapterReleaseAdmissionTerminal,
    ComputeExternalPoolAdapterReleaseAdmissionTerminalReceipt,
};

pub(super) fn canonical_terminal_json_and_digest(
    receipt: &ComputeExternalPoolAdapterReleaseAdmissionTerminalReceipt,
) -> Result<(String, String)> {
    canonical_external_pool_adapter_release_admission_terminal_json_and_digest(receipt)
}

pub(super) fn terminal_request_digest(
    terminal: &ComputeExternalPoolAdapterReleaseAdmissionTerminal,
) -> Result<String> {
    canonical_external_pool_adapter_release_admission_terminal_request_digest(terminal)
}
