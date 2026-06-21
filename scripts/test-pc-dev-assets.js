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
  const devTasks = sandbox.window.ElonPcDevTasks.create({
    clean,
    escapeHtml,
    markdown: { renderMessage: (content) => `<div>${escapeHtml(content)}</div>` },
    refreshActiveChannel: () => {},
    cancelTask: () => {},
    draftContinuation: () => {}
  });

  const messages = [
    { kind: 'ai_task', task_id: 'tsk_1234567890', content: '发起 AI 开发任务：修复登录按钮' },
    { kind: 'ai_progress', task_id: 'tsk_1234567890', content: '正在检查项目' },
    { kind: 'ai_result', task_id: 'tsk_1234567890', content: '任务失败：测试未通过' }
  ];
  const context = devTasks.buildContext(messages);
  const html = devTasks.renderMessage(messages[2], context);

  assert.ok(html.includes('data-dev-task-action="continue"'), 'result card should expose continue action');
  assert.ok(html.includes('data-dev-task-action="refresh"'), 'result card should keep refresh action');
  assert.strictEqual(devTasks.hasOpenTasks(messages, context), false, 'finished task should not be considered open');
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
  const context = devTasks.buildContext([
    { kind: 'ai_task', task_id: 'tsk_tools', content: '发起 AI 开发任务：检查状态' },
    call,
    result
  ]);
  const callHtml = devTasks.renderMessage(call, context);
  const resultHtml = devTasks.renderMessage(result, context);

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
    nodes: [{ node_id: 'node-1', online: true, route_a_ready: true, allowed_clis: ['codex'], device_name: 'PC' }],
    projectSpace: { channels: [{ id: 'ch-dev', kind: 'ai_development' }] }
  };
  const readyHtml = create(readyState).renderMemberPanel({
    id: 'p1',
    node_id: 'node-1',
    workspace_path: 'D:/demo',
    runtime_permission: 'project_write'
  });

  assert.ok(readyHtml.includes('可以开发'), 'ready project should be marked developable');
  assert.ok(readyHtml.includes('AI 开发频道可用'), 'ready checklist should include development channel');
  assert.ok(readyHtml.includes('Route A · codex'), 'ready checklist should show selected route');

  const fullAccessHtml = create(readyState).renderMemberPanel({
    id: 'p1',
    node_id: 'node-1',
    workspace_path: 'D:/demo',
    runtime_permission: 'full_access'
  });
  assert.ok(fullAccessHtml.includes('B/C 保留白名单'), 'full access copy should explain Route B/C boundaries');

  const blockedHtml = create({ nodes: [], projectSpace: { channels: [] } }).renderMemberPanel({ id: 'p2' });
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
  assert.ok(inserted.innerHTML.includes('Route B · 本机 API runtime'), 'composer should show API runtime route');
  assert.ok(inserted.innerHTML.includes('data-dev-composer-route="route_b"'), 'composer should expose Route B selector');
  assert.strictEqual(composer.selectedRouteForRequest(), '', 'auto route should not be sent to backend');
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
  const pcApp = fs.readFileSync(path.join(repoRoot, 'server/src/assets/pc_app.js'), 'utf8');
  assert.ok(pcApp.includes('X-Elon-Local-Admin-Token'), 'PC app should send local admin token header');
  assert.ok(pcApp.includes('refreshLocalAdminToken'), 'PC app should refresh the local admin token');
  assert.ok(pcApp.includes('resp.status === 403'), 'PC app should retry once after a stale local token');

  const nodeAdmin = fs.readFileSync(path.join(repoRoot, 'server/src/node_agent_admin.html'), 'utf8');
  assert.ok(nodeAdmin.includes('X-Elon-Local-Admin-Token'), 'standalone node admin page should send local admin token header');
  assert.ok(nodeAdmin.includes('localFetch'), 'standalone node admin page should route API calls through localFetch');

  const doctor = fs.readFileSync(path.join(repoRoot, 'server/src/assets/pc_app_doctor.js'), 'utf8');
  assert.ok(doctor.includes('localNodeApi(path, options || {})'), 'doctor project should reuse the protected local node API');
}

(async () => {
  testDevTasksContinueAction();
  testDevTasksToolTimeline();
  await testDevTasksToolApprovalButtons();
  testProjectReadinessChecklist();
  testDevComposerRouteLabels();
  testDevComposerForcedRoutePreference();
  testLocalAdminTokenWiring();

  console.log('pc-dev-assets tests passed');
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
