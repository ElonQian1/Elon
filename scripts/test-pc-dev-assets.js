// scripts/test-pc-dev-assets.js
const assert = require('assert');
const fs = require('fs');
const path = require('path');
const vm = require('vm');

const repoRoot = path.resolve(__dirname, '..');

function createMemoryStorage(initial = {}) {
  const entries = new Map(Object.entries(initial));
  return {
    getItem(key) {
      return entries.has(key) ? entries.get(key) : null;
    },
    setItem(key, value) {
      entries.set(key, String(value));
    },
    removeItem(key) {
      entries.delete(key);
    }
  };
}

function loadAsset(relativePath, extraSandbox = {}) {
  const code = fs.readFileSync(path.join(repoRoot, relativePath), 'utf8');
  const localStorage = extraSandbox.localStorage || createMemoryStorage();
  const sandbox = {
    window: {},
    document: { querySelector: () => null, createElement: () => ({ querySelectorAll: () => [] }) },
    localStorage,
    ...extraSandbox
  };
  if (!sandbox.window.localStorage) sandbox.window.localStorage = sandbox.localStorage;
  vm.runInNewContext(code, sandbox, { filename: relativePath });
  return sandbox;
}

function clean(value) {
  return String(value || '').trim();
}

function escapeHtml(value) {
  return String(value || '').replace(/[&<>"']/g, (ch) => ({
    '&': '&amp;',
    '<': '&lt;',
    '>': '&gt;',
    '"': '&quot;',
    "'": '&#39;'
  }[ch]));
}

function testDevTasksContinueAction() {
  const sandbox = loadAsset('server/src/assets/pc_app_dev_tasks.js');
  let drafted = '';
  const devTasks = sandbox.window.ElonPcDevTasks.create({
    clean,
    escapeHtml,
    markdown: { renderMessage: (content) => `<div>${escapeHtml(content)}</div>` },
    refreshActiveChannel: () => {},
    cancelTask: () => {},
    draftContinuation: (draft) => { drafted = draft; }
  });

  const messages = [
    { kind: 'ai_task', task_id: 'tsk_1234567890', content: '发起 AI 开发任务：修复登录按钮' },
    { kind: 'ai_progress', task_id: 'tsk_1234567890', content: '正在检查项目' },
    { kind: 'ai_result', task_id: 'tsk_1234567890', content: '任务失败：测试未通过' }
  ];
  const snapshots = new Map([[
    'tsk_1234567890',
    {
      task: { id: 'tsk_1234567890', status: 'failed' },
      pc_req_id: 'req-local-1',
      resume: {
        can_resume_codex_session: true,
        codex_session: { id: 'session-uuid', scope_key: 'scope-a' }
      }
    }
  ]]);
  const context = devTasks.buildContext(messages, { snapshots });
  const html = devTasks.renderMessage(messages[2], context);

  assert.ok(html.includes('data-dev-task-action="continue"'), 'result card should expose continue action');
  assert.ok(html.includes('data-dev-task-action="refresh"'), 'result card should keep refresh action');
  assert.strictEqual(devTasks.hasOpenTasks(messages, context), false, 'finished task should not be considered open');

  let handler = null;
  const buttons = [{
    dataset: { taskId: 'tsk_1234567890' },
    addEventListener: (_event, callback) => { handler = callback; }
  }];
  devTasks.bindActions({
    querySelectorAll(selector) {
      return selector === '[data-dev-task-action="continue"]' ? buttons : [];
    }
  });
  assert.ok(handler, 'bindActions should register continuation click handler');
  handler();

  assert.ok(drafted.includes('云端任务 ID：tsk_1234567890'), 'continuation draft should include cloud task id');
  assert.ok(drafted.includes('本机请求 ID：req-local-1'), 'continuation draft should include local pc req id');
  assert.ok(drafted.includes('本机 Codex session 已记录'), 'continuation draft should explain automatic codex resume');
  assert.ok(!drafted.includes('session-uuid'), 'continuation draft should not leak raw codex session id');
}

function testDevTasksHasOpenPendingApproval() {
  const sandbox = loadAsset('server/src/assets/pc_app_dev_tasks.js');
  const devTasks = sandbox.window.ElonPcDevTasks.create({
    clean,
    escapeHtml,
    markdown: { renderMessage: (content) => `<div>${escapeHtml(content)}</div>` },
    refreshActiveChannel: () => {},
    cancelTask: () => {},
    approveTool: async () => {},
    draftContinuation: () => {}
  });
  const messages = [
    { kind: 'ai_task', task_id: 'tsk_open_approval', content: '发起 AI 开发任务：运行测试' },
    {
      kind: 'ai_progress',
      task_id: 'tsk_open_approval',
      content: JSON.stringify({
        type: 'tool_approval_required',
        tool: 'run_command',
        approval_id: 'tap_1_1',
        status: 'pending'
      })
    }
  ];
  const context = devTasks.buildContext(messages);
  assert.strictEqual(devTasks.hasOpenTasks(messages, context), true, 'pending approval without result should be considered open');
}

function testDevTasksUsesPersistedTaskStatus() {
  const sandbox = loadAsset('server/src/assets/pc_app_dev_tasks.js');
  const devTasks = sandbox.window.ElonPcDevTasks.create({
    clean,
    escapeHtml,
    markdown: { renderMessage: (content) => `<div>${escapeHtml(content)}</div>` },
    refreshActiveChannel: () => {},
    cancelTask: () => {},
    approveTool: async () => {},
    draftContinuation: () => {}
  });
  const messages = [
    {
      kind: 'ai_task',
      task_id: 'tsk_interrupted_status',
      task_status: 'interrupted',
      task_error: 'server restarted before task finished',
      content: '发起 AI 开发任务：继续恢复'
    },
    {
      kind: 'ai_progress',
      task_id: 'tsk_interrupted_status',
      task_status: 'interrupted',
      content: JSON.stringify({
        type: 'tool_approval_required',
        tool: 'run_command',
        approval_id: 'tap_stale',
        status: 'pending'
      })
    }
  ];
  const context = devTasks.buildContext(messages);
  const startHtml = devTasks.renderMessage(messages[0], context);
  const approvalHtml = devTasks.renderMessage(messages[1], context);

  assert.strictEqual(devTasks.hasOpenTasks(messages, context), false, 'terminal task status should close the task even without ai_result');
  assert.ok(startHtml.includes('已中断'), 'task card should use persisted interrupted status');
  assert.ok(startHtml.includes('data-dev-task-action="continue"'), 'terminal status card should offer continue action');
  assert.ok(!approvalHtml.includes('data-decision="approve"'), 'terminal task should not keep stale approve button');
  assert.ok(approvalHtml.includes('已失效'), 'stale approval should show invalid state');
}

function testDevTasksUsesSnapshotAttachState() {
  const sandbox = loadAsset('server/src/assets/pc_app_dev_tasks.js');
  const devTasks = sandbox.window.ElonPcDevTasks.create({
    clean,
    escapeHtml,
    markdown: { renderMessage: (content) => `<div>${escapeHtml(content)}</div>` },
    refreshActiveChannel: () => {},
    cancelTask: () => {},
    approveTool: async () => {},
    draftContinuation: () => {}
  });
  const snapshots = new Map([[
    'tsk_detached',
    {
      task: { id: 'tsk_detached', status: 'interrupted', error: 'server restarted' },
      attach: { status: 'detached', live: false },
      last_event_seq: 12
    }
  ]]);
  const messages = [
    { kind: 'ai_task', task_id: 'tsk_detached', content: '发起 AI 开发任务：恢复现场' }
  ];
  const context = devTasks.buildContext(messages, { snapshots });
  const html = devTasks.renderMessage(messages[0], context);

  assert.strictEqual(devTasks.hasOpenTasks(messages, context), false, 'snapshot terminal status should close task');
  assert.strictEqual(devTasks.openTaskIds(messages, context).length, 0, 'snapshot terminal status should remove task from polling');
  assert.ok(html.includes('现场已脱离'), 'task card should expose detached snapshot state');
  assert.ok(html.includes('data-dev-task-action="continue"'), 'detached terminal task should offer continue action');
}

function testDevTasksToolTimeline() {
  const sandbox = loadAsset('server/src/assets/pc_app_dev_tasks.js');
  const devTasks = sandbox.window.ElonPcDevTasks.create({
    clean,
    escapeHtml,
    markdown: { renderMessage: (content) => `<div>${escapeHtml(content)}</div>` },
    refreshActiveChannel: () => {},
    cancelTask: () => {},
    draftContinuation: () => {}
  });

  const call = {
    kind: 'ai_progress',
    task_id: 'tsk_tools',
    content: JSON.stringify({
      type: 'tool_call',
      tool: 'run_command',
      status: 'running',
      args: { program: 'git', args: ['status', '--short'] }
    })
  };
  const result = {
    kind: 'ai_progress',
    task_id: 'tsk_tools',
    content: JSON.stringify({
      type: 'tool_result',
      tool: 'run_command',
      status: 'ok',
      result: 'exit=0\\nstdout:\\n'
    })
  };
  const runtimeStatus = {
    kind: 'ai_progress',
    task_id: 'tsk_tools',
    content: JSON.stringify({
      type: 'runtime_status',
      runtime: 'api-runtime',
      phase: 'thinking',
      message: '正在调用模型生成下一步计划',
      turn: 1
    })
  };
  const context = devTasks.buildContext([
    { kind: 'ai_task', task_id: 'tsk_tools', content: '发起 AI 开发任务：检查状态' },
    runtimeStatus,
    call,
    result
  ]);
  const runtimeHtml = devTasks.renderMessage(runtimeStatus, context);
  const callHtml = devTasks.renderMessage(call, context);
  const resultHtml = devTasks.renderMessage(result, context);

  assert.ok(runtimeHtml.includes('运行阶段'), 'runtime status progress should render as runtime card');
  assert.ok(runtimeHtml.includes('运行时正在思考'), 'runtime status should show phase title');
  assert.ok(runtimeHtml.includes('api-runtime'), 'runtime status should show runtime label');
  assert.ok(callHtml.includes('工具调用'), 'tool call progress should render as tool card');
  assert.ok(callHtml.includes('run_command'), 'tool call card should show tool name');
  assert.ok(callHtml.includes('&quot;program&quot;: &quot;git&quot;'), 'tool call card should show escaped args');
  assert.ok(resultHtml.includes('工具结果'), 'tool result progress should render as tool result card');
  assert.ok(resultHtml.includes('exit=0'), 'tool result card should show output');
}

async function testDevTasksToolApprovalButtons() {
  const calls = [];
  const sandbox = loadAsset('server/src/assets/pc_app_dev_tasks.js');
  const devTasks = sandbox.window.ElonPcDevTasks.create({
    clean,
    escapeHtml,
    markdown: { renderMessage: (content) => `<div>${escapeHtml(content)}</div>` },
    refreshActiveChannel: () => {},
    cancelTask: () => {},
    approveTool: async (taskId, approvalId, decision) => {
      calls.push({ taskId, approvalId, decision });
    },
    draftContinuation: () => {}
  });

  const approval = {
    kind: 'ai_progress',
    task_id: 'tsk_approval',
    content: JSON.stringify({
      type: 'tool_approval_required',
      tool: 'apply_patch',
      approval_id: 'tap_1_1',
      status: 'pending',
      args: { files: ['src/main.rs'], patch_chars: 42 },
      diff: { preview: '--- a/src/main.rs\\n+++ b/src/main.rs\\n-<old>\\n+new', files: ['src/main.rs'] }
    })
  };
  const context = devTasks.buildContext([
    { kind: 'ai_task', task_id: 'tsk_approval', content: '发起 AI 开发任务：改代码' },
    approval
  ]);
  const html = devTasks.renderMessage(approval, context);

  assert.ok(html.includes('工具审批'), 'approval event should render as approval card');
  assert.ok(html.includes('data-dev-task-action="tool-approval"'), 'approval card should expose approval buttons');
  assert.ok(html.includes('data-approval-id="tap_1_1"'), 'approval button should keep approval id');
  assert.ok(html.includes('&lt;old&gt;'), 'diff preview should be HTML escaped');

  const approvedDecision = {
    kind: 'ai_progress',
    task_id: 'tsk_approval',
    content: JSON.stringify({
      type: 'tool_approval_decision',
      tool: 'apply_patch',
      approval_id: 'tap_1_1',
      decision: 'approve',
      status: 'approved'
    })
  };
  const decidedContext = devTasks.buildContext([
    { kind: 'ai_task', task_id: 'tsk_approval', content: '发起 AI 开发任务：改代码' },
    approval,
    approvedDecision
  ]);
  const decidedApprovalHtml = devTasks.renderMessage(approval, decidedContext);
  assert.ok(decidedApprovalHtml.includes('apply_patch 已批准'), 'replayed approval card should show recovered approved state');
  assert.ok(!decidedApprovalHtml.includes('data-dev-task-action="tool-approval"'), 'decided approval card should not expose approval buttons');

  const deniedDecision = {
    kind: 'ai_progress',
    task_id: 'tsk_approval',
    content: JSON.stringify({
      type: 'tool_approval_decision',
      tool: 'apply_patch',
      approval_id: 'tap_1_1',
      decision: 'deny',
      status: 'denied'
    })
  };
  const deniedApprovalHtml = devTasks.renderMessage(approval, devTasks.buildContext([
    { kind: 'ai_task', task_id: 'tsk_approval', content: '发起 AI 开发任务：改代码' },
    approval,
    deniedDecision
  ]));
  assert.ok(deniedApprovalHtml.includes('apply_patch 已拒绝'), 'replayed approval card should show recovered denied state');
  assert.ok(!deniedApprovalHtml.includes('data-dev-task-action="tool-approval"'), 'denied approval card should not expose approval buttons');

  const writeApproval = {
    kind: 'ai_progress',
    task_id: 'tsk_write_approval',
    content: JSON.stringify({
      type: 'tool_approval_required',
      tool: 'write_file',
      approval_id: 'tap_1_2',
      status: 'pending',
      args: { path: 'docs/note.md', content_chars: 6 },
      diff: {
        source: 'write_file',
        preview: '--- /dev/null\\n+++ b/docs/note.md\\n+<new>',
        files: ['docs/note.md']
      }
    })
  };
  const writeContext = devTasks.buildContext([
    { kind: 'ai_task', task_id: 'tsk_write_approval', content: '发起 AI 开发任务：写文档' },
    writeApproval
  ]);
  const writeHtml = devTasks.renderMessage(writeApproval, writeContext);
  assert.ok(writeHtml.includes('write_file'), 'write_file approval should render tool name');
  assert.ok(writeHtml.includes('docs/note.md'), 'write_file approval should render file chip');
  assert.ok(writeHtml.includes('Diff 预览'), 'write_file approval should render diff preview');
  assert.ok(writeHtml.includes('&lt;new&gt;'), 'write_file diff preview should be HTML escaped');

  const expiredDecision = {
    kind: 'ai_progress',
    task_id: 'tsk_write_approval',
    content: JSON.stringify({
      type: 'tool_approval_decision',
      tool: 'write_file',
      approval_id: 'tap_1_2',
      decision: 'timeout',
      status: 'expired'
    })
  };
  const expiredHtml = devTasks.renderMessage(expiredDecision, devTasks.buildContext([
    { kind: 'ai_task', task_id: 'tsk_write_approval', content: '发起 AI 开发任务：写文档' },
    writeApproval,
    expiredDecision
  ]));
  assert.ok(expiredHtml.includes('write_file 已过期'), 'expired approval decision should not look approved');
  assert.ok(expiredHtml.includes('审批已过期'), 'expired approval decision should explain final state');

  const canceledDecision = {
    kind: 'ai_progress',
    task_id: 'tsk_write_approval',
    content: JSON.stringify({
      type: 'tool_approval_decision',
      tool: 'write_file',
      approval_id: 'tap_1_2',
      decision: 'canceled',
      status: 'canceled'
    })
  };
  const canceledHtml = devTasks.renderMessage(writeApproval, devTasks.buildContext([
    { kind: 'ai_task', task_id: 'tsk_write_approval', content: '发起 AI 开发任务：写文档' },
    writeApproval,
    canceledDecision
  ]));
  assert.ok(canceledHtml.includes('write_file 已取消'), 'canceled decision should close replayed approval card');
  assert.ok(!canceledHtml.includes('data-dev-task-action="tool-approval"'), 'canceled approval card should not expose approval buttons');

  let handler = null;
  const buttons = [{
    dataset: { taskId: 'tsk_approval', approvalId: 'tap_1_1', decision: 'approve' },
    disabled: false,
    addEventListener: (_event, callback) => { handler = callback; },
    closest: () => ({ querySelectorAll: () => buttons })
  }];
  devTasks.bindActions({
    querySelectorAll(selector) {
      return selector === '[data-dev-task-action="tool-approval"]' ? buttons : [];
    }
  });
  assert.ok(handler, 'bindActions should register approval click handler');
  await handler();
  assert.deepStrictEqual(calls, [{
    taskId: 'tsk_approval',
    approvalId: 'tap_1_1',
    decision: 'approve'
  }]);
}

async function testTaskSnapshotsPollsSnapshotEndpoint() {
  let scheduled = null;
  const sandbox = loadAsset('server/src/assets/pc_app_task_snapshots.js', {
    setTimeout: (callback) => {
      scheduled = callback;
      return 7;
    },
    clearTimeout: () => {},
    console: { warn: () => {} }
  });
  const state = {
    activeKind: 'project',
    activeProjectId: 'p1',
    activeChannelId: 'ch-dev',
    activeChannelKind: 'ai_development'
  };
  const calls = [];
  let rendered = null;
  const snapshots = sandbox.window.ElonPcTaskSnapshots.create({
    state,
    clean,
    sameId: (a, b) => String(a || '') === String(b || ''),
    devTasks: { openTaskIds: () => ['tsk_live'] },
    api: async (path) => {
      calls.push(path);
      return {
        task: { id: 'tsk_live', status: 'running' },
        messages: [{ kind: 'ai_task', task_id: 'tsk_live', content: '发起 AI 开发任务：测试快照' }],
        events: [{ seq: 5, event: { type: 'tool_call' } }],
        last_event_seq: 5,
        attach: { status: 'live', live: true }
      };
    },
    renderMessages: (messages, scope) => {
      rendered = { messages, scope };
    },
    refreshActiveChannel: async () => {}
  });

  assert.strictEqual(snapshots.schedule([], 'project', {}), true, 'snapshot scheduler should accept active dev tasks');
  assert.ok(scheduled, 'snapshot scheduler should create a timer');
  await scheduled();

  assert.ok(calls[0].includes('/api/projects/p1/channels/ch-dev/ai-tasks/tsk_live/snapshot'), 'scheduler should call task snapshot endpoint');
  assert.ok(calls[0].includes('since=0'), 'first snapshot poll should start from cursor 0');
  assert.strictEqual(rendered.scope, 'project', 'snapshot response should rerender project messages');
  assert.strictEqual(snapshots.contextExtras().snapshots.get('tsk_live').attach.status, 'live', 'snapshot attach state should be cached for task cards');
}

async function testTaskSnapshotsMergeLocalJournal() {
  let scheduled = null;
  const sandbox = loadAsset('server/src/assets/pc_app_task_snapshots.js', {
    setTimeout: (callback) => {
      scheduled = callback;
      return 8;
    },
    clearTimeout: () => {},
    console: { warn: () => {} }
  });
  const state = {
    activeKind: 'project',
    activeProjectId: 'p1',
    activeChannelId: 'ch-dev',
    activeChannelKind: 'ai_development'
  };
  const snapshots = sandbox.window.ElonPcTaskSnapshots.create({
    state,
    clean,
    sameId: (a, b) => String(a || '') === String(b || ''),
    devTasks: { openTaskIds: () => ['tsk_local'] },
    api: async () => ({
      task: { id: 'tsk_local', status: 'running' },
      pc_req_id: 'req-local-1',
      messages: [{ kind: 'ai_task', task_id: 'tsk_local', content: '发起 AI 开发任务：测试本机恢复' }],
      events: [],
      last_event_seq: 1,
      attach: { status: 'detached', live: false }
    }),
    localNodeApi: async (path) => {
      assert.ok(path.includes('/api/task-journal/req-local-1'), 'local journal endpoint should use cloud pc_req_id mapping');
      return {
        ok: true,
        task_id: 'req-local-1',
        record: { req_id: 'req-local-1', status: 'running' },
        events: [{ seq: 2, event: { type: 'started', req_id: 'req-local-1' } }],
        last_event_seq: 2,
        attach: { status: 'live', live: true, source: 'local_journal' },
        resume: {
          status: 'live',
          can_reconnect: true,
          can_cancel: true,
          can_stream_live_output: false,
          can_replay_journal_events: true,
          next_action: 'wait_or_cancel',
          strategy: { kind: 'control_handle_reconnect', label: '重连本机控制句柄' }
        }
      };
    },
    renderMessages: () => {},
    refreshActiveChannel: async () => {}
  });

  assert.strictEqual(snapshots.schedule([], 'project', {}), true, 'snapshot scheduler should accept local journal tasks');
  await scheduled();
  const snapshot = snapshots.contextExtras().snapshots.get('tsk_local');
  assert.strictEqual(snapshot.attach.status, 'live', 'local journal live state should override detached cloud attach');
  assert.strictEqual(snapshot.attach.source, 'local_journal', 'local journal source should be retained');
  assert.strictEqual(snapshot.local_journal.last_event_seq, 2, 'local journal cursor should be cached');
  assert.strictEqual(snapshot.resume.next_action, 'wait_or_cancel', 'local resume contract should be cached');
}

async function testTaskSnapshotsReplaysLocalJournalMessages() {
  let scheduled = null;
  let rendered = null;
  const sandbox = loadAsset('server/src/assets/pc_app_task_snapshots.js', {
    setTimeout: (callback) => {
      scheduled = callback;
      return 9;
    },
    clearTimeout: () => {},
    console: { warn: () => {} }
  });
  const state = {
    activeKind: 'project',
    activeProjectId: 'p1',
    activeChannelId: 'ch-dev',
    activeChannelKind: 'ai_development'
  };
  const snapshots = sandbox.window.ElonPcTaskSnapshots.create({
    state,
    clean,
    sameId: (a, b) => String(a || '') === String(b || ''),
    devTasks: { openTaskIds: () => ['tsk_replay'] },
    api: async () => ({
      task: { id: 'tsk_replay', status: 'running' },
      pc_req_id: 'req-replay',
      messages: [{ kind: 'ai_task', task_id: 'tsk_replay', content: '发起 AI 开发任务：回放本机事件' }],
      events: [],
      last_event_seq: 1,
      attach: { status: 'live', live: true }
    }),
    localNodeApi: async () => ({
      ok: true,
      task_id: 'req-replay',
      events: [
        { seq: 2, event: { type: 'cli_chunk', req_id: 'req-replay', text: '本机 stdout\\n' } },
        { seq: 3, event: { type: 'tool_event', req_id: 'req-replay', text: JSON.stringify({ type: 'tool_call', tool: 'run_command' }) } },
        {
          seq: 4,
          event: {
            type: 'tool_event',
            req_id: 'req-replay',
            text: JSON.stringify({ type: 'tool_approval_required', tool: 'write_file', approval_id: 'tap_local' }),
            event: { type: 'tool_approval_required', tool: 'write_file', approval_id: 'tap_local' }
          }
        }
      ],
      last_event_seq: 4,
      attach: { status: 'live', live: true, source: 'local_journal' },
      resume: { status: 'live', can_replay_journal_events: true, next_action: 'wait_or_cancel' }
    }),
    renderMessages: (messages) => {
      rendered = messages;
    },
    refreshActiveChannel: async () => {}
  });

  assert.strictEqual(snapshots.schedule([], 'project', {}), true, 'snapshot scheduler should accept replay task');
  await scheduled();
  assert.ok(rendered.some((message) => clean(message.content).includes('本机 stdout')), 'local stdout should be appended as progress');
  assert.ok(rendered.some((message) => message.content.includes('"tool_call"')), 'local tool event should be appended as progress');
  assert.ok(rendered.some((message) => message.content.includes('[本机回放] write_file 等待审批')), 'local-only approval should be read-only replay text');
}

function testDevTasksUsesLocalJournalAttachLabel() {
  const sandbox = loadAsset('server/src/assets/pc_app_dev_tasks.js');
  const devTasks = sandbox.window.ElonPcDevTasks.create({
    clean,
    escapeHtml,
    markdown: { renderMessage: (content) => `<div>${escapeHtml(content)}</div>` },
    refreshActiveChannel: () => {},
    cancelTask: () => {},
    draftContinuation: () => {}
  });

  const messages = [
    { kind: 'ai_task', task_id: 'tsk_local', content: '发起 AI 开发任务：测试本机标签' }
  ];
  const snapshots = new Map([[
    'tsk_local',
    {
      task: { id: 'tsk_local', status: 'running' },
      attach: { status: 'live', live: true, source: 'local_journal' },
      resume: {
        status: 'live',
        can_stream_live_output: false,
        can_replay_journal_events: true,
        can_resume_codex_session: true,
        codex_session: { id: 'session-uuid', scope_key: 'scope-a', updated_at_ms: 9 },
        next_action: 'wait_or_cancel',
        strategy: { kind: 'control_handle_reconnect', label: '重连本机控制句柄' }
      }
    }
  ]]);
  const context = devTasks.buildContext(messages, { snapshots });
  const html = devTasks.renderMessage(messages[0], context);
  assert.ok(html.includes('本机现场可连接'), 'task card should label local journal live attach state');
  assert.ok(html.includes('本机事件可回放'), 'task card should expose local journal replay');
  assert.ok(html.includes('Codex 会话可续接'), 'task card should expose codex session resume capability');
  assert.ok(html.includes('原 CLI 终端不可重接'), 'task card should explain original CLI terminal cannot be reattached');
}

function testDevTasksUsesResumeContractForSnapshotContinue() {
  const sandbox = loadAsset('server/src/assets/pc_app_dev_tasks.js');
  const devTasks = sandbox.window.ElonPcDevTasks.create({
    clean,
    escapeHtml,
    markdown: { renderMessage: (content) => `<div>${escapeHtml(content)}</div>` },
    refreshActiveChannel: () => {},
    cancelTask: () => {},
    approveTool: async () => {},
    draftContinuation: () => {}
  });

  const messages = [
    { kind: 'ai_task', task_id: 'tsk_detached_local', content: '发起 AI 开发任务：继续修复 Win 客户端' },
    {
      kind: 'ai_progress',
      task_id: 'tsk_detached_local',
      content: JSON.stringify({
        type: 'tool_approval_required',
        tool: 'run_command',
        approval_id: 'tap_detached',
        status: 'pending'
      })
    }
  ];
  const snapshots = new Map([[
    'tsk_detached_local',
    {
      task: { id: 'tsk_detached_local', status: 'running' },
      attach: { status: 'detached', live: false, source: 'local_journal' },
      resume: {
        status: 'detached',
        can_reconnect: false,
        can_cancel: false,
        can_stream_live_output: false,
        can_replay_journal_events: true,
        next_action: 'continue_from_snapshot',
        strategy: { kind: 'snapshot_continue', label: '基于快照继续' }
      }
    }
  ]]);
  const context = devTasks.buildContext(messages, { snapshots });
  const taskHtml = devTasks.renderMessage(messages[0], context);
  const approvalHtml = devTasks.renderMessage(messages[1], context);

  assert.strictEqual(devTasks.hasOpenTasks(messages, context), false, 'detached resume contract should close polling');
  assert.strictEqual(devTasks.openTaskIds(messages, context).length, 0, 'detached resume contract should remove task from open IDs');
  assert.ok(taskHtml.includes('需要基于快照继续'), 'detached task should not look normally running');
  assert.ok(taskHtml.includes('基于快照继续'), 'detached task should expose snapshot continuation mode');
  assert.ok(taskHtml.includes('原 CLI 终端不可重接'), 'detached task should explain CLI terminal reattach limitation');
  assert.ok(taskHtml.includes('data-dev-task-action="continue"'), 'detached task should offer continue action');
  assert.ok(!taskHtml.includes('data-dev-task-action="cancel"'), 'detached task should not offer stop action');
  assert.ok(!approvalHtml.includes('data-decision="approve"'), 'detached task should close stale approval buttons');
}

function testDevTasksRequiresActiveApprovalHandle() {
  const sandbox = loadAsset('server/src/assets/pc_app_dev_tasks.js');
  const devTasks = sandbox.window.ElonPcDevTasks.create({
    clean,
    escapeHtml,
    markdown: { renderMessage: (content) => `<div>${escapeHtml(content)}</div>` },
    refreshActiveChannel: () => {},
    approveTool: async () => {},
    cancelTask: () => {}
  });
  const messages = [
    { kind: 'ai_task', task_id: 'tsk_live_approval', content: '发起 AI 开发任务：审批测试' },
    {
      kind: 'ai_progress',
      task_id: 'tsk_live_approval',
      content: JSON.stringify({
        type: 'tool_approval_required',
        tool: 'run_command',
        approval_id: 'tap_live',
        status: 'pending'
      })
    }
  ];
  const staleSnapshots = new Map([[
    'tsk_live_approval',
    {
      task: { id: 'tsk_live_approval', status: 'running' },
      attach: { status: 'live', live: true, source: 'local_journal' },
      resume: {
        status: 'live',
        can_reconnect: true,
        can_cancel: true,
        can_approve_tools: false,
        active_approval_ids: [],
        can_stream_live_output: false,
        can_replay_journal_events: true,
        next_action: 'wait_or_cancel',
        run_handle: { id: 'req-live', route: 'route_c_server_runtime', os_pid: 4321 },
        strategy: { kind: 'control_handle_reconnect', label: '重连本机控制句柄' }
      }
    }
  ]]);
  const staleContext = devTasks.buildContext(messages, { snapshots: staleSnapshots });
  const taskHtml = devTasks.renderMessage(messages[0], staleContext);
  const staleApprovalHtml = devTasks.renderMessage(messages[1], staleContext);
  assert.ok(taskHtml.includes('PID 4321'), 'live task card should expose the run handle pid');
  assert.ok(staleApprovalHtml.includes('本机没有活动审批等待器'), 'live task without waiter should explain stale approval');
  assert.ok(!staleApprovalHtml.includes('data-decision="approve"'), 'live task without waiter must not expose approval buttons');

  const activeSnapshots = new Map([[
    'tsk_live_approval',
    {
      task: { id: 'tsk_live_approval', status: 'running' },
      attach: { status: 'live', live: true, source: 'local_journal' },
      resume: {
        status: 'live',
        can_reconnect: true,
        can_cancel: true,
        can_approve_tools: true,
        active_approval_ids: ['tap_live'],
        can_stream_live_output: false,
        can_replay_journal_events: true,
        next_action: 'wait_or_cancel',
        run_handle: { id: 'req-live', route: 'route_c_server_runtime' },
        strategy: { kind: 'control_handle_reconnect', label: '重连本机控制句柄' }
      }
    }
  ]]);
  const activeContext = devTasks.buildContext(messages, { snapshots: activeSnapshots });
  const activeApprovalHtml = devTasks.renderMessage(messages[1], activeContext);
  assert.ok(activeApprovalHtml.includes('data-decision="approve"'), 'active waiter should keep approval buttons available');
  assert.ok(activeApprovalHtml.includes('data-approval-id="tap_live"'), 'approval button should keep the active approval id');
}

function testProjectReadinessChecklist() {
  const sandbox = loadAsset('server/src/assets/pc_app_project_readiness.js');
  const create = (state) => sandbox.window.ElonPcProjectReadiness.create({
    state,
    $: () => null,
    clean,
    escapeHtml,
    api: async () => ({}),
    openSettings: () => {},
    selectNode: () => {},
    selectProject: () => {},
    selectProjectChannel: () => {}
  });

  const readyState = {
    nodes: [{
      node_id: 'node-1',
      online: true,
      route_a_ready: true,
      allowed_clis: ['codex'],
      device_name: 'PC',
      dev_runtime: {
        local_tool_contract: {
          supported_tools: ['list_dir', 'read_file', 'read_file_range', 'write_file', 'apply_patch', 'run_command'],
          approval_required_tools: ['write_file', 'apply_patch', 'run_command']
        }
      }
    }],
    projectSpace: { channels: [{ id: 'ch-dev', kind: 'ai_development' }] }
  };
  const readyHtml = create(readyState).renderMemberPanel({
    id: 'p1',
    role: 'owner',
    node_id: 'node-1',
    workspace_path: 'D:/demo',
    runtime_permission: 'project_write'
  });

  assert.ok(readyHtml.includes('可以开发'), 'ready project should be marked developable');
  assert.ok(readyHtml.includes('AI 开发频道可用'), 'ready checklist should include development channel');
  assert.ok(readyHtml.includes('Route A · codex'), 'ready checklist should show selected route');
  assert.ok(readyHtml.includes('read_file_range'), 'readiness should expose Route B/C file-range tool contract');
  assert.ok(readyHtml.includes('apply_patch'), 'readiness should expose Route B/C patch tool contract');
  assert.ok(readyHtml.includes('run_command'), 'readiness should expose Route B/C command tool contract');
  assert.ok(readyHtml.includes('需确认'), 'readiness should expose approval-required tools');

  const fullAccessHtml = create(readyState).renderMemberPanel({
    id: 'p1',
    role: 'owner',
    node_id: 'node-1',
    workspace_path: 'D:/demo',
    runtime_permission: 'full_access'
  });
  assert.ok(fullAccessHtml.includes('B/C 保留白名单'), 'full access copy should explain Route B/C boundaries');

  const failedRouteAHtml = create({
    nodes: [{ node_id: 'node-1', online: true, route_a_ready: false, allowed_clis: ['codex'] }],
    projectSpace: { channels: [{ id: 'ch-dev', kind: 'ai_development' }] }
  }).renderMemberPanel({
    id: 'p1',
    role: 'owner',
    node_id: 'node-1',
    workspace_path: 'D:/demo',
    runtime_permission: 'project_write'
  });
  assert.ok(!failedRouteAHtml.includes('可以开发'), 'failed Route A probe alone should not mark project developable');
  assert.ok(failedRouteAHtml.includes('codex CLI 探测未通过'), 'readiness should explain failed Route A probe');

  const blockedHtml = create({ nodes: [], projectSpace: { channels: [] } }).renderMemberPanel({ id: 'p2', role: 'owner' });
  assert.ok(blockedHtml.includes('未绑定本机'), 'unbound project should not be marked ready');
  assert.ok(blockedHtml.includes('未绑定本机项目目录'), 'blocked checklist should explain missing workspace');
  assert.ok(blockedHtml.includes('未找到 AI 开发频道'), 'blocked checklist should explain missing dev channel');
}

function testDevComposerRouteLabels() {
  let inserted = null;
  const fakeParent = {
    insertBefore(element) {
      inserted = element;
    }
  };
  const sandbox = loadAsset('server/src/assets/pc_app_dev_composer.js', {
    document: {
      createElement: () => ({ hidden: true, className: '', innerHTML: '', querySelectorAll: () => [] })
    }
  });
  const state = {
    activeKind: 'project',
    activeChannelKind: 'ai_development',
    activeProjectId: 'p1',
    projects: [{ id: 'p1', node_id: 'node-1', workspace_path: 'D:/demo', runtime_permission: 'full_access' }],
    nodes: [{ node_id: 'node-1', online: true, api_runtime_ready: true, allowed_clis: [] }]
  };
  const composer = sandbox.window.ElonPcDevComposer.create({
    state,
    els: { composer: { parentElement: fakeParent } },
    clean,
    escapeHtml,
    openSettings: () => {},
    selectNode: () => {},
    openModelPicker: () => {}
  });

  composer.render();
  assert.ok(inserted, 'composer bar should be inserted');
  assert.strictEqual(inserted.hidden, false, 'composer bar should be visible in AI development channel');
  assert.ok(inserted.className.includes('full-access'), 'composer should expose full access tone');
  assert.ok(inserted.innerHTML.includes('完全访问（本机确认）'), 'composer should label full access as locally confirmed');
  assert.ok(inserted.innerHTML.includes('Route B · 本机 API runtime'), 'composer should show API runtime route');
  assert.ok(inserted.innerHTML.includes('data-dev-composer-route="route_b"'), 'composer should expose Route B selector');
  assert.strictEqual(composer.selectedRouteForRequest(), '', 'auto route should not be sent to backend');
}

function testDevComposerSkipsFailedRouteAProbe() {
  let inserted = null;
  const fakeParent = {
    insertBefore(element) {
      inserted = element;
    }
  };
  const sandbox = loadAsset('server/src/assets/pc_app_dev_composer.js', {
    document: {
      createElement: () => ({ hidden: true, className: '', innerHTML: '', querySelectorAll: () => [] })
    }
  });
  const state = {
    activeKind: 'project',
    activeChannelKind: 'ai_development',
    activeProjectId: 'p1',
    projects: [{ id: 'p1', node_id: 'node-1', workspace_path: 'D:/demo', runtime_permission: 'project_write' }],
    nodes: [{
      node_id: 'node-1',
      online: true,
      route_a_ready: false,
      server_runtime_ready: true,
      allowed_clis: ['codex']
    }]
  };
  const composer = sandbox.window.ElonPcDevComposer.create({
    state,
    els: { composer: { parentElement: fakeParent } },
    clean,
    escapeHtml,
    openSettings: () => {},
    selectNode: () => {},
    openModelPicker: () => {}
  });

  composer.render();
  assert.ok(inserted.innerHTML.includes('Route C · 服务器模型'), 'auto route should skip failed Route A and show Route C');
  assert.ok(inserted.innerHTML.includes('codex CLI 登录/版本探测未通过'), 'Route A button should explain failed probe');
  assert.strictEqual(composer.selectedRouteForRequest(), '', 'auto fallback should remain an automatic backend choice');
}

function testDevComposerFallsBackFromUnavailableStoredRoute() {
  let inserted = null;
  const fakeParent = {
    insertBefore(element) {
      inserted = element;
    }
  };
  const sandbox = loadAsset('server/src/assets/pc_app_dev_composer.js', {
    localStorage: createMemoryStorage({ elon_pc_dev_runtime_route: 'route_a' }),
    document: {
      createElement: () => ({ hidden: true, className: '', innerHTML: '', querySelectorAll: () => [] })
    }
  });
  const state = {
    activeKind: 'project',
    activeChannelKind: 'ai_development',
    activeProjectId: 'p1',
    projects: [{ id: 'p1', node_id: 'node-1', workspace_path: 'D:/demo', runtime_permission: 'project_write' }],
    nodes: [{
      node_id: 'node-1',
      online: true,
      route_a_ready: false,
      server_runtime_ready: true,
      allowed_clis: ['codex']
    }]
  };
  const composer = sandbox.window.ElonPcDevComposer.create({
    state,
    els: { composer: { parentElement: fakeParent } },
    clean,
    escapeHtml,
    openSettings: () => {},
    selectNode: () => {},
    openModelPicker: () => {}
  });

  composer.render();
  assert.ok(inserted.innerHTML.includes('已跳过失效偏好'), 'composer should explain stored route fallback');
  assert.ok(inserted.innerHTML.includes('aria-pressed="true"'), 'composer should keep a visible active route');
  assert.strictEqual(composer.selectedRouteForRequest(), '', 'unavailable stored route must not be forced to backend');
}

function testDevComposerForcedRoutePreference() {
  let inserted = null;
  const fakeParent = {
    insertBefore(element) {
      inserted = element;
    }
  };
  const sandbox = loadAsset('server/src/assets/pc_app_dev_composer.js', {
    localStorage: createMemoryStorage({ elon_pc_dev_runtime_route: 'route_c' }),
    document: {
      createElement: () => ({ hidden: true, className: '', innerHTML: '', querySelectorAll: () => [] })
    }
  });
  const state = {
    activeKind: 'project',
    activeChannelKind: 'ai_development',
    activeProjectId: 'p1',
    projects: [{ id: 'p1', node_id: 'node-1', workspace_path: 'D:/demo', runtime_permission: 'project_write' }],
    nodes: [{
      node_id: 'node-1',
      online: true,
      route_a_ready: true,
      server_runtime_ready: true,
      allowed_clis: ['codex']
    }]
  };
  const composer = sandbox.window.ElonPcDevComposer.create({
    state,
    els: { composer: { parentElement: fakeParent } },
    clean,
    escapeHtml,
    openSettings: () => {},
    selectNode: () => {},
    openModelPicker: () => {}
  });

  composer.render();
  assert.ok(inserted.innerHTML.includes('Route C · 服务器模型'), 'forced Route C should control the displayed route');
  assert.strictEqual(composer.selectedRouteForRequest(), 'route_c', 'forced route should be sent to backend');
}

function testLocalAdminTokenWiring() {
  const webRs = fs.readFileSync(path.join(repoRoot, 'server/src/web.rs'), 'utf8');
  const routerRs = fs.readFileSync(path.join(repoRoot, 'server/src/router.rs'), 'utf8');
  assert.ok(webRs.includes('pc_app_task_snapshots.js'), 'PC task snapshot asset should be embedded in web.rs');
  assert.ok(routerRs.includes('/assets/pc_app_task_snapshots.js'), 'PC task snapshot asset should be routed');

  const pcApp = fs.readFileSync(path.join(repoRoot, 'server/src/assets/pc_app.js'), 'utf8');
  assert.ok(pcApp.includes('X-Elon-Local-Admin-Token'), 'PC app should send local admin token header');
  assert.ok(pcApp.includes('refreshLocalAdminToken'), 'PC app should refresh the local admin token');
  assert.ok(pcApp.includes('resp.status === 403'), 'PC app should retry once after a stale local token');
  assert.ok(pcApp.includes('/api/full-access/grants'), 'PC app should write local full-access grants');
  assert.ok(pcApp.includes('localNodeApi, clean'), 'PC app should pass protected local node API into task snapshots');
  assert.ok(pcApp.includes('confirm_full_access: true'), 'full-access grant request should include explicit confirmation');
  assert.ok(pcApp.includes('ensureProjectInfoBeforeRegister'), 'project registration should inspect local folders before submit');
  assert.ok(pcApp.includes('正在自动读取目录信息'), 'project registration should explain auto inspection progress');
  assert.ok(pcApp.includes('settings-project-meta-row'), 'project inspection metadata should render as structured rows');
  assert.ok(pcApp.includes('registration.can_register'), 'project registration should read readiness from local inspect');
  assert.ok(pcApp.includes('autofill_fields'), 'project registration should display auto-filled fields');
  assert.ok(pcApp.includes('目录信息不足'), 'project registration should block submission when required fields are missing');
  assert.ok(pcApp.includes('/api/client-maintenance/diagnostics/export'), 'PC app should export client diagnostics through local node');
  assert.ok(pcApp.includes('exportClientDiagnosticsBtn'), 'PC app should wire the diagnostics export button');
  assert.ok(pcApp.includes('openClientTaskJournalBtn'), 'PC settings should keep task journal separate from runtime logs');
  assert.ok(pcApp.includes('openClientLauncherLogsBtn'), 'PC settings should expose launcher logs separately');
  assert.ok(pcApp.includes("openClientMaintenanceTarget('logs'"), 'PC settings should open the runtime logs target');
  assert.ok(pcApp.includes("openClientMaintenanceTarget('launcher_logs'"), 'PC settings should open launcher logs target');
  assert.ok(pcApp.includes("openClientMaintenanceTarget('task_journal'"), 'PC settings should still open task journal explicitly');
  assert.ok(pcApp.includes('logs_dir'), 'PC settings should display the client runtime logs directory');
  assert.ok(pcApp.includes('launcher_logs_dir'), 'PC settings should display the client launcher logs directory');
  assert.ok(pcApp.includes('clientPackageLatest'), 'PC settings should keep latest Windows client package metadata');
  assert.ok(pcApp.includes('客户端已是最新'), 'PC settings should compare installed and latest client package versions');
  assert.ok(pcApp.includes('positionRailTooltip'), 'PC rail should position its custom hover tooltip');
  assert.ok(pcApp.includes('aria-describedby'), 'PC rail icons should expose the shared tooltip to assistive tech');
  assert.ok(pcApp.includes("button.removeAttribute('title')"), 'PC rail icons should avoid duplicate native tooltips');

  const pcAppNode = fs.readFileSync(path.join(repoRoot, 'server/src/assets/pc_app_node.js'), 'utf8');
  assert.ok(pcAppNode.includes('admissionAvailability'), 'PC node page should read Route C admission availability');
  assert.ok(pcAppNode.includes('admission_availability'), 'PC node page should keep snake_case admission compatibility');
  assert.ok(pcAppNode.includes('routeCLimitedReasonText'), 'PC node page should explain Route C limited reasons');
  assert.ok(pcAppNode.includes('秒后重试'), 'PC node page should show Route C retry-after hints');

  const pcAppHtml = fs.readFileSync(path.join(repoRoot, 'server/src/assets/pc_app.html'), 'utf8');
  assert.ok(pcAppHtml.includes('exportClientDiagnosticsBtn'), 'PC settings should expose diagnostics export entry');
  assert.ok(pcAppHtml.includes('openClientTaskJournalBtn'), 'PC settings should expose task journal as its own button');
  assert.ok(pcAppHtml.includes('openClientLauncherLogsBtn'), 'PC settings should expose launcher logs as its own button');
  assert.ok(pcAppHtml.includes('打开运行日志'), 'PC settings should expose runtime logs as a user-facing action');
  assert.ok(pcAppHtml.includes('打开启动器日志'), 'PC settings should expose launcher logs as a user-facing action');

  const pcAppCss = fs.readFileSync(path.join(repoRoot, 'server/src/assets/pc_app.css'), 'utf8');
  assert.ok(pcAppCss.includes('.settings-project-meta-row'), 'PC app CSS should style structured project inspection metadata');
  assert.ok(pcAppCss.includes('.settings-project-meta-row.is-warning'), 'PC app CSS should style project registration warnings');
  assert.ok(pcAppCss.includes('.settings-project-meta-row.is-error'), 'PC app CSS should style blocked project registration state');
  assert.ok(pcAppCss.includes('.rail-avatar::before'), 'PC rail should render a Discord-like active indicator');
  assert.ok(pcAppCss.includes('.rail-avatar:focus-visible'), 'PC rail should have a keyboard focus state');
  assert.ok(pcAppCss.includes('border-radius: inherit'), 'PC rail images should clip to the current icon shape');
  assert.ok(pcAppCss.includes('--rail-tooltip-arrow-y'), 'PC rail tooltip arrow should track clamped tooltip position');

  const nodeAgentMain = fs.readFileSync(path.join(repoRoot, 'server/src/node_agent_main.rs'), 'utf8');
  assert.ok(nodeAgentMain.includes('node_agent_task_journal_api::routes()'), 'task journal API should be mounted behind local admin guard');
  assert.ok(nodeAgentMain.includes('/api/client-maintenance/diagnostics/export'), 'node agent should mount diagnostics export route');

  const nodeAdmin = fs.readFileSync(path.join(repoRoot, 'server/src/node_agent_admin.html'), 'utf8');
  assert.ok(nodeAdmin.includes('X-Elon-Local-Admin-Token'), 'standalone node admin page should send local admin token header');
  assert.ok(nodeAdmin.includes('localFetch'), 'standalone node admin page should route API calls through localFetch');
  assert.ok(nodeAdmin.includes('apiModelInput'), 'standalone node admin page should expose Route B model input');
  assert.ok(nodeAdmin.includes('api_base: apiBase || null'), 'standalone node admin page should save Route B API base');
  assert.ok(nodeAdmin.includes('logs_dir'), 'standalone node admin page should display runtime logs directory');
  assert.ok(nodeAdmin.includes('launcher_logs_dir'), 'standalone node admin page should display launcher logs directory');
  assert.ok(nodeAdmin.includes("openMaintenanceTarget('logs'"), 'standalone node admin page should open runtime logs');
  assert.ok(nodeAdmin.includes("openMaintenanceTarget('launcher_logs'"), 'standalone node admin page should open launcher logs');
  assert.ok(nodeAdmin.includes("openMaintenanceTarget('task_journal'"), 'standalone node admin page should keep task journal separate');
  assert.ok(nodeAdmin.includes('diagnostics_dir'), 'standalone node admin page should expose diagnostics directory target');

  const nativeNodeAdmin = fs.readFileSync(path.join(repoRoot, 'server/src/assets/pc_app_node_admin.js'), 'utf8');
  assert.ok(nativeNodeAdmin.includes('nodeApiRuntimeModel'), 'PC node panel should expose Route B model input');
  assert.ok(nativeNodeAdmin.includes('api_runtime_ready'), 'PC node panel should show Route B readiness');
  assert.ok(nativeNodeAdmin.includes('api_base: apiBase || null'), 'PC node panel should persist Route B API base');
  assert.ok(nativeNodeAdmin.includes('nodeOpenDiagnosticsDir'), 'PC node panel should expose diagnostics directory entry');
  assert.ok(nativeNodeAdmin.includes('diagnostics_dir'), 'PC node panel should open diagnostics directory target');
  assert.ok(nativeNodeAdmin.includes('logs_dir'), 'PC node panel should display runtime logs directory');
  assert.ok(nativeNodeAdmin.includes('open_client_logs'), 'PC node panel should expose runtime logs action');
  assert.ok(nativeNodeAdmin.includes('launcher_logs_dir'), 'PC node panel should display launcher logs directory');
  assert.ok(nativeNodeAdmin.includes('open_launcher_logs'), 'PC node panel should expose launcher logs action');
  assert.ok(nativeNodeAdmin.includes('loadLatestClientPackageVersion'), 'PC node panel should fetch latest client package metadata');
  assert.ok(nativeNodeAdmin.includes('clientUpdateLine'), 'PC node panel should compare installed and latest client versions');

  const doctor = fs.readFileSync(path.join(repoRoot, 'server/src/assets/pc_app_doctor.js'), 'utf8');
  assert.ok(doctor.includes('localNodeApi(path, options || {})'), 'doctor project should reuse the protected local node API');
}

(async () => {
  testDevTasksContinueAction();
  testDevTasksHasOpenPendingApproval();
  testDevTasksUsesPersistedTaskStatus();
  testDevTasksUsesSnapshotAttachState();
  testDevTasksUsesLocalJournalAttachLabel();
  testDevTasksUsesResumeContractForSnapshotContinue();
  testDevTasksRequiresActiveApprovalHandle();
  testDevTasksToolTimeline();
  await testDevTasksToolApprovalButtons();
  await testTaskSnapshotsPollsSnapshotEndpoint();
  await testTaskSnapshotsMergeLocalJournal();
  await testTaskSnapshotsReplaysLocalJournalMessages();
  testProjectReadinessChecklist();
  testDevComposerRouteLabels();
  testDevComposerSkipsFailedRouteAProbe();
  testDevComposerFallsBackFromUnavailableStoredRoute();
  testDevComposerForcedRoutePreference();
  testLocalAdminTokenWiring();

  console.log('pc-dev-assets tests passed');
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
