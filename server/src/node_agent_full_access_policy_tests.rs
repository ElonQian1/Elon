use super::*;

#[test]
fn runtime_policy_summary_exposes_route_bc_safety_limits() {
    let summary = runtime_policy_summary();

    assert_eq!(summary["schema"], "elon.pc_node.runtime_policy.v1");
    assert_eq!(summary["fullAccess"]["routeAInstalledCliOnly"], true);
    assert_eq!(
        summary["fullAccess"]["routeBCFullAccessEffect"],
        "keeps_workspace_path_checks_command_allowlist_and_tool_approvals"
    );
    assert_eq!(
        summary["fullAccess"]["routeBCDangerFullAccessEffect"],
        "danger_full_access_allows_absolute_paths_arbitrary_shell_and_skips_tool_approvals"
    );
    assert_eq!(
        summary["operatorVisibility"]["policyField"],
        "runtime_policy"
    );

    let approval_tools = summary["routeBC"]["approvalRequiredTools"]
        .as_array()
        .expect("approvalRequiredTools should be an array");
    for tool in ["write_file", "apply_patch", "run_command"] {
        assert!(
            approval_tools
                .iter()
                .any(|item| item.as_str() == Some(tool)),
            "missing approval tool {tool}"
        );
    }

    let denied = summary["routeBC"]["highRiskGitPushDenied"]
        .as_array()
        .expect("highRiskGitPushDenied should be an array");
    for arg in ["--force*", "--delete", "--mirror", "+refspec", ":branch"] {
        assert!(
            denied.iter().any(|item| item.as_str() == Some(arg)),
            "missing high-risk git push marker {arg}"
        );
    }
}
