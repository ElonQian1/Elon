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
  const operations = require(path.join(pcRoot, 'src', 'features', 'local-tasks', 'localOperationsModel.ts'))
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
  assert.strictEqual(taskTitle.readableTaskTitle(''), '本机 Codex 任务')
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
  const isolatedTask = model.normalizeLocalTaskList({ tasks: [{
    task_id: 'local-supervised', project_id: 'project-a', status: 'running',
    workspace_path: 'D:\\conversation-worktrees\\project-a\\supervised-1',
    workspace_status: { base_workspace_path: 'D:\\projects\\project-a', active_workspace_path: 'D:\\conversation-worktrees\\project-a\\supervised-1' },
  }] })[0]
  assert.strictEqual(isolatedTask.workspace_path, 'D:\\conversation-worktrees\\project-a\\supervised-1', 'PC normalizer must preserve the production record execution worktree')
  assert.notStrictEqual(isolatedTask.workspace_path, 'D:\\projects\\project-a')

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
    recovery_timing: {
      mode: 'supersede', parent_task_id: 'local-parent', handoff_ms: 102000,
      resumed_work_ms: 360000, total_since_parent_finished_ms: 462000,
      handoff_target_ms: 480000, handoff_within_target: true,
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
  assert.strictEqual(detail.recovery_timing.mode, 'supersede')
  assert.strictEqual(detail.recovery_timing.handoff_ms, 102000)
  assert.strictEqual(detail.recovery_timing.resumed_work_ms, 360000)
  assert.strictEqual(detail.recovery_timing.handoff_within_target, true)

  const canceled = model.normalizeLocalTaskDetail({
    record: { req_id: 'local-cancel', status: 'cancel_requested' },
    events: [{ seq: 1, event: {
      type: 'cancel_requested', requested_by: 'owner-1', source: 'pc_ui',
      reason: 'user_stop_button', requested_at_ms: 1720000002000,
      interruption_source: 'supervisor_intervention',
    } }],
  })
  assert.strictEqual(canceled.cancel_audit.requested_by, 'owner-1')
  assert.strictEqual(canceled.cancel_audit.source, 'pc_ui')
  assert.strictEqual(canceled.cancel_audit.reason, 'user_stop_button')
  assert.strictEqual(canceled.cancel_audit.requested_at_ms, 1720000002000)
  assert.strictEqual(canceled.cancel_audit.interruption_source, 'supervisor_intervention')

  const merged = model.mergeLocalTaskEvents(detail.events, [
    detail.events[1],
    model.normalizeLocalTaskEvent({ seq: 3, event: { type: 'done', message: 'done' } }),
  ])
  assert.deepStrictEqual(merged.map((event) => event.seq), [1, 2, 3])
  assert.strictEqual(model.localTaskStatus('failed').terminal, true)
  assert.strictEqual(model.syncStateLabel('pending'), '待云端恢复后同步')
  assert.strictEqual(model.syncStateLabel('retrying'), '同步重试中')

  const evolution = operations.normalizeSelfEvolutionQueue({
    items: [{ logical_id: 'evolution-1', root_task_id: 'root-1', parent_task_id: 'parent-1', project_id: 'project-1', conversation_id: 'self-evolution-1', status: 'paused', generation: 2, pause_reason: 'global_publish', execution_worktree: 'D:\\worktrees\\self-evolution-1', execution_isolated: true, yield_reason: 'global_publish', interruption_source: 'supervisor_intervention', reviewed_by: 'pc_operator:owner-1', review_source: 'local_pc_ui', retry_count: 1, max_retries: 3 }],
    gates: { publish_active: true, publish_owner: 'server:builder-a', publish_waiter_count: 2 },
  })
  assert.strictEqual(evolution.items[0].generation, 2)
  assert.strictEqual(evolution.items[0].pause_reason, 'global_publish')
  assert.strictEqual(evolution.items[0].execution_isolated, true)
  assert.notStrictEqual(evolution.items[0].execution_worktree, 'D:\\work\\elon', 'queue/UI must expose the isolated execution path')
  assert.strictEqual(evolution.items[0].interruption_source, 'supervisor_intervention')
  assert.strictEqual(evolution.items[0].review_source, 'local_pc_ui')
  assert.strictEqual(evolution.gates.publish_active, true)
  assert.strictEqual(evolution.gates.publish_waiter_count, 2)
  const publish = operations.normalizeGlobalPublishStatus({ stateHealth: 'healthy', globalPublish: {
    owner: { kind: 'server', sha: 'abcdef123456', batchId: 'release-abcdef123456', stage: 'server', builderLabel: 'builder-a' },
    waiters: [{ kind: 'apk', sha: 'fedcba654321', batchId: 'apk-release-fedcba654321', stage: 'android_apk', builderLabel: 'builder-b' }],
    waiterCount: 1, queuePolicy: 'fifo', coalescingKey: 'kind+sha', immutableReleaseSha: true,
  }, releaseBatches: [{ batchId: 'release-abcdef123456', sha: 'abcdef123456', expectedStages: ['server', 'pc_frontend', 'windows_node'], status: 'in_progress', updatedAt: 1720000003000, stages: [{ stage: 'server', kind: 'server', status: 'running', phase: 'server_deploy', phaseStatus: 'running', builderId: 'builder-a', attempt: 2 }] }] })
  assert.strictEqual(publish.owner.kind, 'server')
  assert.strictEqual(publish.owner.batchId, 'release-abcdef123456')
  assert.strictEqual(publish.waiters[0].kind, 'apk')
  assert.strictEqual(publish.immutableReleaseSha, true)
  assert.strictEqual(publish.stateHealth, 'healthy')
  assert.strictEqual(publish.batches[0].stages[0].attempt, 2)
  assert.deepStrictEqual(publish.batches[0].expectedStages, ['server', 'pc_frontend', 'windows_node'])
  assert.strictEqual(publish.batches[0].stages[0].phase, 'server_deploy')
  assert.strictEqual(publish.batches[0].stages[0].phaseStatus, 'running')

  const bannerSource = readSource('src/features/shell/LocalModeBanner.tsx')
  const shellSource = readSource('src/features/shell/Shell.tsx')
  const appSource = readSource('src/App.tsx')
  const railSource = readSource('src/features/shell/ServerRail.tsx')
  const apiSource = readSource('src/features/local-tasks/localTaskApi.ts')
  const pageSource = readSource('src/features/local-tasks/LocalTasksPage.tsx')
  const detailSource = readSource('src/features/local-tasks/LocalTaskDetailPanel.tsx')
  const supervisionSource = readSource('src/features/local-tasks/LocalTaskSupervisionPanel.tsx')
  const recoverySource = readSource('src/features/local-tasks/LocalTaskUpdateRecoveryPanel.tsx')
  const operationsSource = readSource('src/features/local-tasks/LocalOperationsPanel.tsx')
  const continuationSource = readSource('src/features/local-tasks/LocalTaskContinuationPanel.tsx')
  assert.ok(!bannerSource.includes('window.location.replace'), 'cloud recovery must not force navigation')
  assert.ok(bannerSource.includes('返回云端工作台'), 'cloud recovery must expose an explicit return action')
  assert.ok(shellSource.includes('useNotifications(!localMode)'), 'local mode must disable cloud websocket notifications')
  assert.ok(shellSource.includes('!duplicateTab && !localMode'), 'local mode must disable project prewarm')
  assert.ok(appSource.includes("isLocalWorkbench() ? '/local-tasks' : '/ai'"), 'local workbench root must open local tasks without mounting cloud AI')
  assert.ok(railSource.includes('localMode ? [LOCAL_TASK_ITEM] : RAIL_ITEMS'), 'local mode must hide cloud-only navigation')
  assert.ok(apiSource.includes("'/api/local-tasks'"), 'local tasks must use the node-local endpoint')
  assert.ok(apiSource.includes('/cancel`'), 'local task cancel endpoint must be explicit')
  assert.ok(apiSource.includes("source: 'pc_ui'"), 'local cancel must submit its audit source')
  assert.ok(apiSource.includes('/tool-approvals/'), 'local tool approvals must use the task-local endpoint')
  assert.ok(pageSource.includes('ensureLocalFullAccessGrant'), 'local task creation must explicitly confirm and persist workspace access')
  assert.ok(detailSource.includes('当前阶段'), 'local task details must expose the live runtime phase')
  assert.ok(detailSource.includes('current_command'), 'local task details must expose the redacted current command')
  assert.ok(detailSource.includes('idle_duration'), 'local task details must expose progress-aware idle duration')
  assert.ok(detailSource.includes('cancel-audit'), 'local task details must expose cancel audit provenance')
  assert.ok(detailSource.includes('恢复交接和后续开发分别统计'), 'recovery timing must not mix handoff with resumed work')
  assert.ok(continuationSource.includes('继续原任务'), 'unchanged requirements must have an explicit resume action')
  assert.ok(continuationSource.includes('需求变更承接'), 'changed requirements must have an explicit supersede action')
  assert.ok(continuationSource.includes('新的验收条件'), 'supersede must collect explicit revised acceptance criteria')
  assert.ok(supervisionSource.includes('桌面监督闭环'), 'supervised tasks must expose their evidence and verdict in the PC workbench')
  assert.ok(supervisionSource.includes('PC 本机节点负责执行'), 'the PC workbench must explain the executor and supervisor roles')
  assert.ok(recoverySource.includes('更新恢复全过程'), 'update recovery stages must be visible in the PC workbench')
  assert.ok(recoverySource.includes('remote v1 字段已保留'), 'unverified remote recovery must be visibly fail-closed')
  assert.ok(recoverySource.includes('sidecar_output_offset'), 'the durable sidecar replay cursor must be visible')
  assert.ok(recoverySource.includes('无需手动 Resume'), 'automatic update recovery must state that manual Resume is unnecessary')
  assert.ok(operationsSource.includes('低优先自进化'), 'self evolution queue must be visible')
  assert.ok(operationsSource.includes('全局发布租约'), 'global publish owner and waiters must be visible')
  assert.ok(operationsSource.includes('待审查'), 'self evolution review state must be visible')
  assert.ok(operationsSource.includes('releaseStatus'), 'release batch stages must be visible')
  assert.ok(operationsSource.includes('review_source'), 'self evolution review provenance must be visible')

  console.log('pc-frontend local-task tests passed')
} finally {
  require.extensions['.ts'] = originalTsLoader
}

function readSource(relativePath) {
  return fs.readFileSync(path.join(pcRoot, relativePath), 'utf8')
}
