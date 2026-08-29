//! Single-run supervisor for the complete Windows Unmap evidence family.

use anyhow::{anyhow, Context};

use super::{
    super::a2_dynamic_evidence::{
        UnmapFamilyCohort, ValidatedUnmapCleanCheckoutReceipt, ValidatedUnmapFamily,
        A2_DYNAMIC_CHILD_NONCE_ENV,
    },
    cases, child,
};

pub(super) fn run() -> anyhow::Result<()> {
    reject_ambient_child_environment()?;
    cases::validate_all().map_err(anyhow::Error::msg)?;

    let executable = std::env::current_exe().context("resolve Unmap family test executable")?;
    let cohort = UnmapFamilyCohort::new();
    let mut members = Vec::with_capacity(cases::ALL.len());
    for case in cases::ALL {
        members.push(
            super::capture_family_member(&executable, case, &cohort).with_context(|| {
                format!(
                    "capture exact Unmap family member {}",
                    case.selector.report_name()
                )
            })?,
        );
    }

    let checkout =
        ValidatedUnmapCleanCheckoutReceipt::capture(&cohort).map_err(anyhow::Error::msg)?;
    let rendered = ValidatedUnmapFamily::reduce(cohort, members, checkout)
        .map_err(anyhow::Error::msg)?
        .render_atomic();
    println!("{}", rendered.as_str());
    Ok(())
}

fn reject_ambient_child_environment() -> anyhow::Result<()> {
    if std::env::var_os(child::CHILD_ROOT_ENV).is_some()
        || std::env::var_os(A2_DYNAMIC_CHILD_NONCE_ENV).is_some()
    {
        return Err(anyhow!("A2_UNMAP_FAMILY_AMBIENT_CHILD_ENV"));
    }
    Ok(())
}
