const assert = require('assert')
const fs = require('fs')
const path = require('path')
const ts = require('typescript')

const pcRoot = path.resolve(__dirname, '..')
const originalTsLoader = require.extensions['.ts']

require.extensions['.ts'] = function loadTsModule(module, filename) {
  const source = fs.readFileSync(filename, 'utf8')
  const output = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.CommonJS,
      target: ts.ScriptTarget.ES2020,
    },
    fileName: filename,
  })
  module._compile(output.outputText, filename)
}

try {
  const model = require(path.join(pcRoot, 'src', 'features', 'local-tasks', 'localTaskModel.ts'))
  const taskTitle = require(path.join(pcRoot, 'src', 'lib', 'taskTitle.ts'))

  assert.strictEqual(taskTitle.readableTaskTitle(' 请修复登录按钮错位，并补充测试。 '), '修复登录按钮错位，并补充测试')
  assert.strictEqual(taskTitle.readableTaskTitle('codex://threads/019-test\n请完善本机任务标题'), '完善本机任务标题')
  assert.strictEqual(taskTitle.readableTaskTitle(`
<elon-pc-executor version="1">监督协议</elon-pc-executor>
<user-request>
用户原始需求：“用户希望的是，有适合人阅读且可区分的任务标题。”
桌面监督分析结论：不需要 Goal 模式。
</user-request>`), '适合人阅读且可区分的任务标题')
  assert.strictEqual(taskTitle.readableTaskTitle('codex://threads/019-test\n请继续完成上述任务并运行统一收尾'), '本机 Codex 任务')
  assert.strictEqual(taskTitle.readableTaskTitle('修复'.repeat(40)).length, 34)

  const tasks = model.normalizeLocalTaskList({
    records: [{
      req_id: 'local-1',
      cwd: 'D:\\work\\elon',
      status: 'running',
      sync_state: 'pending',
      usage: { prompt_tokens: 120, completion_tokens: 30 },
    }],
  })
  assert.strictEqual(tasks.length, 1)
  assert.strictEqual(tasks[0].id, 'local-1')
  assert.strictEqual(tasks[0].workspace_path, 'D:\\work\\elon')
  assert.strictEqual(tasks[0].token_usage.total_tokens, 150)
  assert.strictEqual(model.pendingLocalTaskSyncCount(tasks), 1)
  assert.strictEqual(model.pendingSyncCountFromList({ tasks, pending_sync_count: 7 }), 7)
  assert.strictEqual(model.pendingLocalTaskSyncCount([{ ...tasks[0], sync_state: 'local_only' }]), 0)

  const detail = model.normalizeLocalTaskDetail({
    task_id: 'local-1',
    record: { req_id: 'local-1', status: 'done', sync_state: 'synced' },
    events: [
      { seq: 1, event: { type: 'usage', input_tokens: 10, output_tokens: 7, total_tokens: 17 } },
      { seq: 2, event: { type: 'final_reply', message: '已完成本机任务' } },
    ],
    last_event_seq: 2,
    approval_state: {
      approvals: [{ approval_id: 'approval-1', tool: 'run_command', actionable: true }],
    },
    runtime: {
      phase: 'verification',
      current_command: 'cargo test --bin elon-pc-node',
      last_progress: 1720000000000,
      heartbeat: 1720000001000,
      idle_duration: 3,
      timeout_policy: {
        mode: 'progress_aware', total_timeout_secs: 21600, idle_timeout_secs: 900,
        heartbeat_secs: 15, progress_aware: true,
      },
    },
    supervision: {
      protocol: 'elon.desktop_pc_supervision.v1',
      enabled: true,
      contract: {
        protocol: 'elon.desktop_pc_supervision.v1',
        supervisor: 'codex_desktop',
        task_role: 'requirement',
        acceptance_criteria: ['测试通过', '监督验收'],
        improvement_policy: 'after_task_or_unblock',
      },
      evidence: {
        event_count: 8, tool_calls: 2, tool_results: 2, changed_files: ['server/src/main.rs'],
        command_exit_codes: [{ command: 'cargo test', exit_code: 0 }], agent_messages: 1,
      },
    },
    update_recovery: {
      protocol: 'elon.node_update_recovery.v1',
      update_id: 'update-1',
      state: 'reattaching',
      original_task_id: 'local-1',
      from_release: { version: '0.3.69', git_sha: 'oldsha' },
      to_release: { version: '0.3.70', git_sha: 'newsha' },
      sidecar_session_id: 'sidecar-1',
      journal_cursor: 15425,
      sidecar_output_offset: 58,
      sidecar_output_sequence: 3,
      transport: { kind: 'local_loopback', protocol: 'elon.node.v1', capabilities: ['update_recovery_v1'], replay_from_cursor: true },
      completion_event_id: 'completion-1',
      terminal_task_status: 'done',
    },
    resume_workspace_status: {
      eligible: true,
      derivation: 'legacy_started_cwd_git_registry',
      active_workspace_path: 'D:\\conversation-worktrees\\project\\conversation',
      branch: 'ai/session/project/conversation',
      git_head: '0123456789abcdef',
    },
  })
  assert.strictEqual(detail.task.id, 'local-1')
  assert.strictEqual(detail.task.final_reply, '已完成本机任务')
  assert.strictEqual(detail.task.token_usage.total_tokens, 17)
  assert.strictEqual(detail.approvals[0].actionable, true)
  assert.strictEqual(detail.supervision.enabled, true)
  assert.strictEqual(detail.supervision.contract.acceptance_criteria.length, 2)
  assert.strictEqual(detail.supervision.evidence.tool_calls, 2)
  assert.strictEqual(detail.supervision.evidence.command_exit_codes[0].exit_code, 0)
  assert.strictEqual(detail.supervision.evidence.agent_messages, 1)
  assert.strictEqual(detail.runtime.phase, 'verification')
  assert.strictEqual(detail.runtime.current_command, 'cargo test --bin elon-pc-node')
  assert.strictEqual(detail.runtime.timeout_policy.idle_timeout_secs, 900)
  assert.strictEqual(detail.update_recovery.state, 'reattaching')
  assert.strictEqual(detail.update_recovery.sidecar_output_offset, 58)
  assert.strictEqual(detail.resume_workspace_status.eligible, true)

  const merged = model.mergeLocalTaskEvents(detail.events, [
    detail.events[1],
    model.normalizeLocalTaskEvent({ seq: 3, event: { type: 'done', message: 'done' } }),
  ])
  assert.deepStrictEqual(merged.map((event) => event.seq), [1, 2, 3])
  assert.strictEqual(model.localTaskStatus('failed').terminal, true)
  assert.strictEqual(model.syncStateLabel('pending'), '待云端恢复后同步')
  assert.strictEqual(model.syncStateLabel('retrying'), '同步重试中')

  const bannerSource = readSource('src/features/shell/LocalModeBanner.tsx')
  const shellSource = readSource('src/features/shell/Shell.tsx')
  const appSource = readSource('src/App.tsx')
  const railSource = readSource('src/features/shell/ServerRail.tsx')
  const apiSource = readSource('src/features/local-tasks/localTaskApi.ts')
  const pageSource = readSource('src/features/local-tasks/LocalTasksPage.tsx')
  const detailSource = readSource('src/features/local-tasks/LocalTaskDetailPanel.tsx')
  const supervisionSource = readSource('src/features/local-tasks/LocalTaskSupervisionPanel.tsx')
  const recoverySource = readSource('src/features/local-tasks/LocalTaskUpdateRecoveryPanel.tsx')
  assert.ok(!bannerSource.includes('window.location.replace'), 'cloud recovery must not force navigation')
  assert.ok(bannerSource.includes('返回云端工作台'), 'cloud recovery must expose an explicit return action')
  assert.ok(shellSource.includes('useNotifications(!localMode)'), 'local mode must disable cloud websocket notifications')
  assert.ok(shellSource.includes('!duplicateTab && !localMode'), 'local mode must disable project prewarm')
  assert.ok(appSource.includes("isLocalWorkbench() ? '/local-tasks' : '/ai'"), 'local workbench root must open local tasks without mounting cloud AI')
  assert.ok(railSource.includes('localMode ? [LOCAL_TASK_ITEM] : RAIL_ITEMS'), 'local mode must hide cloud-only navigation')
  assert.ok(apiSource.includes("'/api/local-tasks'"), 'local tasks must use the node-local endpoint')
  assert.ok(apiSource.includes('/cancel`'), 'local task cancel endpoint must be explicit')
  assert.ok(apiSource.includes('/tool-approvals/'), 'local tool approvals must use the task-local endpoint')
  assert.ok(pageSource.includes('ensureLocalFullAccessGrant'), 'local task creation must explicitly confirm and persist workspace access')
  assert.ok(detailSource.includes('当前阶段'), 'local task details must expose the live runtime phase')
  assert.ok(detailSource.includes('current_command'), 'local task details must expose the redacted current command')
  assert.ok(detailSource.includes('idle_duration'), 'local task details must expose progress-aware idle duration')
  assert.ok(supervisionSource.includes('桌面监督闭环'), 'supervised tasks must expose their evidence and verdict in the PC workbench')
  assert.ok(supervisionSource.includes('PC 本机节点负责执行'), 'the PC workbench must explain the executor and supervisor roles')
  assert.ok(recoverySource.includes('更新恢复全过程'), 'update recovery stages must be visible in the PC workbench')
  assert.ok(recoverySource.includes('remote v1 字段已保留'), 'unverified remote recovery must be visibly fail-closed')
  assert.ok(recoverySource.includes('sidecar_output_offset'), 'the durable sidecar replay cursor must be visible')

  console.log('pc-frontend local-task tests passed')
} finally {
  require.extensions['.ts'] = originalTsLoader
}

function readSource(relativePath) {
  return fs.readFileSync(path.join(pcRoot, relativePath), 'utf8')
}
