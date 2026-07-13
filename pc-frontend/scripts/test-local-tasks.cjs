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
  })
  assert.strictEqual(detail.task.id, 'local-1')
  assert.strictEqual(detail.task.final_reply, '已完成本机任务')
  assert.strictEqual(detail.task.token_usage.total_tokens, 17)
  assert.strictEqual(detail.approvals[0].actionable, true)

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

  console.log('pc-frontend local-task tests passed')
} finally {
  require.extensions['.ts'] = originalTsLoader
}

function readSource(relativePath) {
  return fs.readFileSync(path.join(pcRoot, relativePath), 'utf8')
}
