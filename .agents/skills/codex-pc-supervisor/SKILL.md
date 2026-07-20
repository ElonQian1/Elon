---
name: codex-pc-supervisor
description: Use the Yilong PC local node as the coding executor while Codex Desktop acts as an independent supervisor. Trigger when the user asks Codex Desktop to dispatch, supervise, inspect, improve, repair, or resume a project task through the PC node or Codex CLI, including the project's “双重自进化” workflow. Do not trigger when the request is already marked as a PC executor prompt; that agent must complete the work directly.
---

# Codex PC Supervisor

Use Codex Desktop for planning, independent evidence review, and improvement decisions. Use the existing Yilong PC node and its Codex CLI for repository writes, tests, commits, publishing, and project-specific finish rules.

## Guard against recursive dispatch

If the current request contains `<elon-pc-executor>`, stop using this skill. Work directly in the named project, follow its `AGENTS.md`, and return verifiable evidence. Never send the task back to the PC node.

## Run the supervised workflow

1. Inspect the project read-only and turn the request into concrete acceptance criteria.
2. Submit the requirement with `scripts/invoke-supervised-task.ps1 -Action Submit`. Always pass the absolute workspace path, original user request, criteria, and a stable project ID.
3. Save the returned `task_id`. Use `-Action Wait -TaskId ...` in windows no longer than 55 seconds. `Wait` retries transient node restarts, advances an event cursor, and returns `next_cursor`; pass that value back with `-Since` on the next call. Use `-Compact` for bounded summaries and `Inspect -Since ... -Limit ...` for incremental evidence. Continue until the task is terminal or needs a user tool approval.
4. Independently inspect the journal, diff, tests, commit, published artifact, and project finish report. A successful executor message alone is not acceptance evidence.
   For Rust/Cargo, require the managed validator fingerprint, durable evidence path, exit code, owner/queue/resource class, and reuse state. Streaming output is not primary evidence, and truncation alone must not trigger a rerun.
5. Record a verdict with `-Action Review`:
   - `accepted`: all acceptance criteria and project finish rules passed.
   - `needs_follow_up`: the task can be corrected without changing PC platform capability.
   - `blocked_capability`: a PC node, dispatcher, recovery, or evidence capability prevents the original task.
   - `rejected`: the result is unsafe or materially wrong.
6. For `needs_follow_up`, submit a narrow follow-up requirement. For `blocked_capability`, run `-Action Improve -BlockingImprovement`, review that repair, then run `-Action Resume` against the terminated original task. `Resume` fails closed unless the parent has the current supervision protocol and a platform-recorded isolated worktree; the node independently revalidates Git identity and exclusive occupancy.
7. Queue non-blocking platform improvements with `-Action Improve` only after the user task is complete. Do not delay the requested result for optional self-improvement.

Freeze the submitted acceptance criteria before real smoke. Adjacent smoke defects must be recorded as structured `needs_follow_up` or `blocked_capability` with whether they directly block that frozen scope; do not expand the current release loop otherwise. A blocking platform repair must retain the original root/parent task association and resume the original business/UI task after repair. Never turn that relationship into an unrelated sleeping improvement task.

The PowerShell helper tries `ELON_NODE_ADMIN_URL`, the current/recent successful token-free URL, and the 7799 fast path before scanning 7800–7819. It obtains the loopback-only admin token afresh without printing or caching it and returns stable JSON. Set `ELON_NODE_ADMIN_URL` only when the node uses another trusted local URL.
Use `-Action Probe` to verify local connectivity and version without creating a task.
Use `-Action Projects` to read the logged-in account's projects and this PC node's workspace bindings without opening a browser. The node calls the Yilong cloud with its direct no-proxy client; `-ProjectFilterId` narrows the result and `-IncludeSystemProjects` includes system projects. Never infer another developer's path from the top-level cloud project record.

For multiple acceptance criteria passed through an outer `powershell -File`, prefer `-AcceptanceCriteriaJson '["one","two"]'` or a UTF-8 JSON file via `-AcceptanceCriteriaFile`. The legacy `-AcceptanceCriteria` array remains compatible.

## Keep authority boundaries explicit

- Desktop supervision does not expand the user's authorization. Destructive, external, or otherwise approval-sensitive work still needs the normal approval.
- Do not silently fall back to Desktop edits when the node is unavailable. Report the node evidence and ask before changing execution mode.
- Keep the executor and reviewer logically separate: the PC node produces evidence; Desktop decides whether it satisfies the contract.
- Treat this as system-level iterative improvement, not autonomous model training. Preserve worktree isolation, versioning, rollback, tests, and audit records.
- Use the repository's own workflow as the final authority. In this project, that includes preflight, the returned `EDIT_ROOT`, commit/push/publish requirements, and a `FINALIZABLE=true` finish report.

## Commands

```powershell
$helper = '.agents\skills\codex-pc-supervisor\scripts\invoke-supervised-task.ps1'

powershell -NoProfile -ExecutionPolicy Bypass -File $helper -Action Submit `
  -ProjectId 'elon-project' -WorkspacePath 'D:\path\to\repo' `
  -Prompt '完成用户需求' -AcceptanceCriteria '定向测试通过','发布验证通过'

powershell -NoProfile -ExecutionPolicy Bypass -File $helper -Action Wait -TaskId 'local-...'

powershell -NoProfile -ExecutionPolicy Bypass -File $helper -Action Inspect `
  -TaskId 'local-...' -Since 40 -Limit 25 -Compact

powershell -NoProfile -ExecutionPolicy Bypass -File $helper -Action Projects `
  -ProjectFilterId 'project-id'

powershell -NoProfile -ExecutionPolicy Bypass -File $helper -Action Review `
  -TaskId 'local-...' -Verdict accepted -Summary 'diff、测试和发布均已验收'
```

Use `Inspect`, `Improve`, and `Resume` for the remaining workflow states. Run `SelfTest` after changing the helper.
