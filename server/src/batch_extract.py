import os

def extract_end_test(src_path, dest_path, mod_name, path_attr):
    with open(src_path, encoding="utf-8") as f:
        content = f.read()
    lines = content.split("\n")
    total = len(lines)
    test_start = None
    for i in range(total-1, -1, -1):
        if lines[i].strip() == "#[cfg(test)]" and i+1 < total and lines[i+1].strip().startswith("mod ") and "{" in lines[i+1]:
            test_start = i
            break
    if test_start is None:
        return False
    brace_depth = 0
    test_end = None
    for i in range(test_start, total):
        brace_depth += lines[i].count("{") - lines[i].count("}")
        if brace_depth == 0 and i > test_start + 1:
            test_end = i
            break
    if test_end is None:
        return False
    after_test = "\n".join(lines[test_end+1:]).strip()
    if after_test:
        print(f"SKIP {src_path}: code after test block")
        return False
    body = "\n".join(lines[test_start+2:test_end])
    d = os.path.dirname(dest_path)
    if d:
        os.makedirs(d, exist_ok=True)
    with open(dest_path, "w", encoding="utf-8") as f:
        f.write(body + "\n")
    pre = "\n".join(lines[:test_start])
    new_content = pre + "\n\n#[cfg(test)]\n#[path = \"" + path_attr + "\"]\nmod " + mod_name + ";\n"
    with open(src_path, "w", encoding="utf-8") as f:
        f.write(new_content)
    print(f"OK {src_path}: {total} -> {test_start+4} lines (extracted {test_end-test_start+1} lines)")
    return True

tasks = [
    ("store/project_member_conversations.rs","store/project_member_conversations_tests.rs","project_member_conversations_tests","project_member_conversations_tests.rs"),
    ("social_ai/reply_core.rs","social_ai/reply_core_tests.rs","reply_core_tests","reply_core_tests.rs"),
    ("store/tasks.rs","store/tasks_tests.rs","tasks_tests","tasks_tests.rs"),
    ("store/codex_vault_emergency.rs","store/codex_vault_emergency_tests.rs","codex_vault_emergency_tests","codex_vault_emergency_tests.rs"),
    ("store/codex_vault_usage_estimation.rs","store/codex_vault_usage_estimation_tests.rs","codex_vault_usage_estimation_tests","codex_vault_usage_estimation_tests.rs"),
    ("store/billing_alerts.rs","store/billing_alerts_tests.rs","billing_alerts_tests","billing_alerts_tests.rs"),
    ("store/codex_vault.rs","store/codex_vault_tests.rs","codex_vault_tests","codex_vault_tests.rs"),
    ("project_attachment_notes.rs","project_attachment_notes_tests.rs","project_attachment_notes_tests","project_attachment_notes_tests.rs"),
    ("node_agent_lifecycle.rs","node_agent_lifecycle_tests.rs","node_agent_lifecycle_tests","node_agent_lifecycle_tests.rs"),
    ("project_membership.rs","project_membership_tests.rs","project_membership_tests","project_membership_tests.rs"),
    ("errors.rs","errors_tests.rs","errors_tests","errors_tests.rs"),
    ("node_client_launcher/mod.rs","node_client_launcher/launcher_tests.rs","launcher_tests","launcher_tests.rs"),
    ("store/friends.rs","store/friends_tests.rs","friends_tests","friends_tests.rs"),
    ("store/pc_project_binding.rs","store/pc_project_binding_tests.rs","pc_project_binding_tests","pc_project_binding_tests.rs"),
    ("node_agent_cli_tool_catalog.rs","node_agent_cli_tool_catalog_tests.rs","node_agent_cli_tool_catalog_tests","node_agent_cli_tool_catalog_tests.rs"),
    ("node_api/public_dev.rs","node_api/public_dev_tests.rs","public_dev_tests","public_dev_tests.rs"),
]
s=0
for t in tasks:
    if extract_end_test(*t): s+=1
print(f"Total: {s}/{len(tasks)}")
