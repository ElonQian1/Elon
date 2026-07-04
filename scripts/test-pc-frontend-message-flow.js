const assert = require('assert');
const fs = require('fs');
const path = require('path');

const repoRoot = path.resolve(__dirname, '..');
const pcRoot = path.join(repoRoot, 'pc-frontend');

function loadTypescript() {
  const localTypescript = path.join(pcRoot, 'node_modules', 'typescript');
  if (fs.existsSync(localTypescript)) return require(localTypescript);
  return require('typescript');
}

const ts = loadTypescript();
const originalTsLoader = require.extensions['.ts'];

require.extensions['.ts'] = function loadTsModule(module, filename) {
  const source = fs.readFileSync(filename, 'utf8');
  const output = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.CommonJS,
      target: ts.ScriptTarget.ES2020,
      esModuleInterop: true,
      jsx: ts.JsxEmit.ReactJSX,
    },
    fileName: filename,
  });
  module._compile(output.outputText, filename);
};

async function main() {
try {
  const {
    buildDisplayMessages,
    buildMessageGroups,
    buildTaskProcessMessageMap,
    hasRunningTask,
    hasRunningTaskAwaitingVisibleReply,
  } = require(path.join(pcRoot, 'src', 'features', 'conversation', 'messageFlow.ts'));
  const {
    statusForTask,
  } = require(path.join(pcRoot, 'src', 'features', 'dev', 'devTaskUtils.ts'));
  const {
    buildTaskTimeline,
    timelineSummary,
  } = require(path.join(pcRoot, 'src', 'features', 'dev', 'taskTimelineModel.ts'));
  const {
    buildAgentRunParallelOverview,
    recoveryViewFromEntry,
    recoveryViewFromTask,
  } = require(path.join(pcRoot, 'src', 'features', 'dev', 'agentRunRecoveryModel.ts'));
  const {
    ensureLocalFullAccessGrant,
    normalizeLocalWorkspacePath,
    requiresLocalFullAccessGrant,
  } = require(path.join(pcRoot, 'src', 'features', 'conversation', 'localPcRuntime.ts'));
  const {
    messageFingerprint,
    sameMessageList,
  } = require(path.join(pcRoot, 'src', 'features', 'conversation', 'messageListCompare.ts'));
  const {
    realtimeResources,
  } = require(path.join(pcRoot, 'src', 'features', 'realtime', 'resourceKeys.ts'));
  const {
    REALTIME_DOM_EVENTS,
    REALTIME_SERVER_TYPES,
    legacyDomEventNameForType,
    normalizeRealtimeEvent,
    resourcesForRealtimeEvent,
  } = require(path.join(pcRoot, 'src', 'features', 'realtime', 'realtimeEvents.ts'));

  assert.strictEqual(normalizeLocalWorkspacePath('E:/一龙项目/'), 'e:\\一龙项目', 'workspace path comparison should ignore slash style and case');
  assert.strictEqual(requiresLocalFullAccessGrant('project_write', true), false, 'project-write mode should not request a local full-access grant');
  assert.strictEqual(requiresLocalFullAccessGrant('full_access', true), true, 'Route A full-access mode should require a local grant');
  assert.strictEqual(requiresLocalFullAccessGrant('danger_full_access', false), false, 'non-Route-A execution should keep its own permission policy');

  const existingGrantRequests = [];
  const existingGrantResult = await ensureLocalFullAccessGrant({
    adminUrl: 'http://127.0.0.1:7799/',
    projectId: 'elon-self',
    projectName: '一龙项目',
    workspacePath: 'E:\\一龙项目',
    runtimePermission: 'danger_full_access',
    useLocalRouteA: true,
  }, {
    request: async (requestPath, options) => {
      existingGrantRequests.push({ requestPath, options });
      return { grants: [{ project_id: 'elon-self', workspace_path: 'e:/一龙项目/' }] };
    },
    confirm: () => { throw new Error('existing grants must not prompt again'); },
  });
  assert.strictEqual(existingGrantResult, 'already_granted', 'matching project and canonical workspace should reuse the existing grant');
  assert.strictEqual(existingGrantRequests.length, 1, 'existing grant should only read the local grant list');

  const missingGrantRequests = [];
  let confirmationText = '';
  const missingGrantResult = await ensureLocalFullAccessGrant({
    adminUrl: 'http://127.0.0.1:7799/',
    projectId: 'elon-self',
    projectName: '一龙项目',
    workspacePath: 'E:\\一龙项目',
    runtimePermission: 'danger_full_access',
    useLocalRouteA: true,
  }, {
    request: async (requestPath, options) => {
      missingGrantRequests.push({ requestPath, options });
      return requestPath === '/api/full-access/grants' && !options ? { grants: [] } : { ok: true };
    },
    confirm: (message) => { confirmationText = message; return true; },
  });
  assert.strictEqual(missingGrantResult, 'granted', 'confirmed missing grant should be persisted before dispatch');
  assert.ok(confirmationText.includes('E:\\一龙项目') && confirmationText.includes('任意命令'), 'danger mode should confirm the exact path and risk locally');
  assert.strictEqual(missingGrantRequests.length, 2, 'missing grant should read then write the local grant');
  assert.deepStrictEqual(JSON.parse(missingGrantRequests[1].options.body), {
    project_id: 'elon-self',
    workspace_path: 'E:\\一龙项目',
    confirm_full_access: true,
  }, 'grant request should carry the exact project, path, and explicit confirmation');

  assert.strictEqual(
    sameMessageList(
      [{ id: 'same-id', kind: 'ai_progress', task_id: 'tsk-1', task_status: 'running', content: '等待' }],
      [{ id: 'same-id', kind: 'ai_progress', task_id: 'tsk-1', task_status: 'done', content: '等待' }],
    ),
    false,
    'same id with changed task status must trigger a React state update',
  );
  assert.strictEqual(
    sameMessageList(
      [{ id: 'same-id', role: 'assistant', content: '旧回复', sender_avatar_data_url: 'old' }],
      [{ id: 'same-id', role: 'assistant', content: '新回复', sender_avatar_data_url: 'old' }],
    ),
    false,
    'same id with changed assistant content must refresh the visible bubble',
  );
  assert.notStrictEqual(
    messageFingerprint({ id: 'avatar-msg', role: 'user', content: '你好', sender_avatar_data_url: 'old' }),
    messageFingerprint({ id: 'avatar-msg', role: 'user', content: '你好', sender_avatar_data_url: 'new' }),
    'avatar changes should invalidate memoized message rows',
  );
  const messageRealtimeEvent = normalizeRealtimeEvent({
    type: REALTIME_SERVER_TYPES.projectMessageUpdated,
    projectId: 'project-1',
    channelId: 'channel-1',
    conversationId: 'conv-1',
    taskId: 'tsk-1',
  });
  assert.ok(messageRealtimeEvent, 'known websocket events should normalize into realtime events');
  assert.deepStrictEqual(
    messageRealtimeEvent.resources,
    [
      realtimeResources.projectSpace('project-1'),
      realtimeResources.channelMessages('project-1', 'channel-1'),
      realtimeResources.conversationAny('project-1', 'conv-1'),
      realtimeResources.taskTimeline('project-1', 'tsk-1'),
      realtimeResources.taskAny('project-1'),
    ],
    'project message events should map to every affected visible resource',
  );
  assert.deepStrictEqual(
    resourcesForRealtimeEvent({
      type: REALTIME_SERVER_TYPES.projectTaskDone,
      projectId: 'project-1',
      conversationId: 'conv-1',
      taskId: 'tsk-1',
    }),
    [
      realtimeResources.projectSpace('project-1'),
      realtimeResources.taskAny('project-1'),
      realtimeResources.taskTimeline('project-1', 'tsk-1'),
      realtimeResources.conversationAny('project-1', 'conv-1'),
    ],
    'task_done events should refresh task timelines and the linked conversation',
  );
  assert.deepStrictEqual(
    resourcesForRealtimeEvent({
      type: REALTIME_SERVER_TYPES.projectAiMatterEvent,
      projectId: 'project-1',
      matterId: 'matter-1',
    }),
    [
      realtimeResources.projectSpace('project-1'),
      realtimeResources.groupAiMatter('project-1', 'matter-1'),
    ],
    'group AI matter events should expose project and matter resources',
  );
  assert.deepStrictEqual(
    resourcesForRealtimeEvent({
      type: REALTIME_SERVER_TYPES.projectMembersUpdated,
      projectId: 'project-1',
    }),
    [
      realtimeResources.projectSpace('project-1'),
      realtimeResources.projectMembers('project-1'),
    ],
    'project member events should refresh member-scoped resources',
  );
  assert.deepStrictEqual(
    resourcesForRealtimeEvent({
      type: REALTIME_SERVER_TYPES.presence,
      userId: 'user-1',
    }),
    [realtimeResources.presence('user-1')],
    'presence events should map to a stable user resource key',
  );
  assert.strictEqual(
    legacyDomEventNameForType(REALTIME_SERVER_TYPES.projectMessageUpdated),
    REALTIME_DOM_EVENTS.projectMessageUpdated,
    'normalized realtime events should keep legacy DOM event compatibility',
  );
  assert.strictEqual(normalizeRealtimeEvent({ type: 'unknown_event' }), null, 'unknown websocket events should be ignored by the realtime layer');

  const taskMessages = [
    { id: 'pcm-task', kind: 'ai_task', task_id: 'tsk-1', content: '发起 AI 开发任务：修复会话 UI' },
    { id: 'pcm-assistant-1', kind: 'ai_progress', task_id: 'tsk-1', content: '{"type":"assistant_message","text":"我先读取规则入口。","model_used":"Claude"}' },
    { id: 'pcm-progress', kind: 'ai_progress', task_id: 'tsk-1', content: '{"type":"tool_call","tool":"rg"}' },
    { id: 'pcm-assistant-2', kind: 'ai_progress', task_id: 'tsk-1', content: '{"type":"assistant_message","text":"规则已读完，接着运行只读命令。","model_used":"codex"}' },
    { id: 'pcm-result', kind: 'ai_result', task_id: 'tsk-1', content: '最终答案：已修复。' },
  ];
  const conversationMessages = [
    { id: 'msg-user', role: 'user', conversation_id: 'conv-1', task_id: 'tsk-1', content: '修复会话 UI' },
    { id: 'msg-assistant', role: 'assistant', conversation_id: 'conv-1', task_id: 'tsk-1', content: '已完成。' },
  ];
  const conversationMessagesWithoutAssistantTaskId = [
    conversationMessages[0],
    { id: 'msg-assistant-no-task', role: 'assistant', conversation_id: 'conv-1', content: '已完成。' },
  ];
  const taskMessagesById = buildTaskProcessMessageMap([taskMessages]);

  const merged = buildDisplayMessages({
    sessionView: 'conv-1',
    channelMessages: [],
    conversationMessages,
    conversationLoading: false,
    taskMessagesById,
  });
  assert.deepStrictEqual(
    merged.map((message) => message.id),
    [
      'msg-user',
      'pcm-task',
      'assistant-progress-pcm-assistant-1',
      'pcm-progress',
      'assistant-progress-pcm-assistant-2',
      'pcm-result',
    ],
    'assistant_message progress rows should become normal assistant bubbles between structured process rows',
  );
  assert.strictEqual(hasRunningTask(merged), false, 'terminal ai_result should close the task');
  assert.strictEqual(merged[2].kind, 'assistant', 'AI CLI reply fragments should render as assistant messages');
  assert.strictEqual(merged[2].task_id, undefined, 'reply fragments should not be swallowed by task grouping');
  assert.strictEqual(merged[2].model_used, 'Claude', 'reply fragments should preserve the source CLI label');

  const groups = buildMessageGroups(merged, true);
  assert.strictEqual(groups.length, 1, 'task-linked user, process, public replies, and final answer should render as one task thread');
  assert.deepStrictEqual(
    groups.map((group) => group.type),
    ['task'],
    'task flow should not split one task into scattered process groups',
  );
  assert.deepStrictEqual(
    groups.map((group) => group.type === 'task' ? group.messages.map((message) => message.id) : [group.message.id]),
    [
      ['msg-user', 'pcm-task', 'assistant-progress-pcm-assistant-1', 'pcm-progress', 'assistant-progress-pcm-assistant-2', 'pcm-result'],
    ],
    'task process, AI CLI reply fragments, and final result should stay in one chronological thread',
  );

  const inFlightTaskMessages = taskMessages.slice(0, 3);
  const mergedWithAssistantFallback = buildDisplayMessages({
    sessionView: 'conv-1',
    channelMessages: [],
    conversationMessages,
    conversationLoading: false,
    taskMessagesById: buildTaskProcessMessageMap([inFlightTaskMessages]),
  });
  assert.deepStrictEqual(
    mergedWithAssistantFallback.map((message) => message.id),
    ['msg-user', 'pcm-task', 'assistant-progress-pcm-assistant-1', 'pcm-progress', 'msg-assistant'],
    'assistant conversation reply should remain in chronological order when channel ai_result is not available yet',
  );
  assert.strictEqual(mergedWithAssistantFallback[4].kind, 'ai_result', 'assistant fallback should be promoted to a task result');
  assert.strictEqual(hasRunningTask(mergedWithAssistantFallback), false, 'assistant fallback should close the task');
  const fallbackGroups = buildMessageGroups(mergedWithAssistantFallback, true);
  assert.strictEqual(fallbackGroups.length, 1, 'assistant fallback should stay attached to the task thread');
  assert.deepStrictEqual(
    fallbackGroups.map((group) => group.type === 'task' ? group.messages.map((message) => message.id) : [group.message.id]),
    [['msg-user', 'pcm-task', 'assistant-progress-pcm-assistant-1', 'pcm-progress', 'msg-assistant']],
    'fallback final answer should close the same task thread after process rows',
  );

  const mergedNoTaskAssistantFallback = buildDisplayMessages({
    sessionView: 'conv-1',
    channelMessages: [],
    conversationMessages: conversationMessagesWithoutAssistantTaskId,
    conversationLoading: false,
    taskMessagesById: buildTaskProcessMessageMap([inFlightTaskMessages]),
  });
  assert.strictEqual(mergedNoTaskAssistantFallback.at(-1).kind, 'ai_result', 'assistant reply without task_id should be promoted to task result');
  assert.strictEqual(mergedNoTaskAssistantFallback.at(-1).task_id, 'tsk-1', 'promoted assistant reply should inherit latest visible task_id');
  assert.strictEqual(hasRunningTask(mergedNoTaskAssistantFallback), false, 'promoted assistant reply should close typing state');
  const noTaskFallbackGroups = buildMessageGroups(mergedNoTaskAssistantFallback, true);
  assert.strictEqual(noTaskFallbackGroups.length, 1, 'promoted assistant reply should stay in the same task thread');

  const conversationOnlyGroups = buildMessageGroups(conversationMessages, true);
  assert.strictEqual(conversationOnlyGroups.length, 1, 'task-linked conversation rows should become one task conversation thread');
  assert.deepStrictEqual(
    conversationOnlyGroups.map((group) => group.type === 'task' ? group.messages.map((message) => message.id) : [group.message.id]).flat(),
    ['msg-user', 'msg-assistant'],
    'conversation-only task messages should preserve user and assistant turns in one group',
  );
  assert.strictEqual(
    hasRunningTask([
      conversationMessages[0],
      { id: 'pcm-task-2', kind: 'ai_task', task_id: 'tsk-1', content: '发起 AI 开发任务：修复会话 UI' },
      { id: 'pcm-progress-2', kind: 'ai_progress', task_id: 'tsk-1', content: '处理中' },
      conversationMessages[1],
    ]),
    false,
    'assistant task reply should stop the typing indicator even before channel ai_result refreshes',
  );
  assert.strictEqual(
    hasRunningTask([
      conversationMessages[0],
      { id: 'pcm-task-3', kind: 'ai_task', task_id: 'tsk-1', content: '发起 AI 开发任务：修复会话 UI' },
      { id: 'pcm-progress-3', kind: 'ai_progress', task_id: 'tsk-1', content: '处理中' },
      { id: 'msg-assistant-no-task', role: 'assistant', conversation_id: 'conv-1', content: '已完成。' },
    ]),
    false,
    'assistant reply without task_id should still stop typing for the latest visible task',
  );

  const runningMessages = taskMessages.slice(0, 2);
  assert.strictEqual(hasRunningTask(runningMessages), true, 'task without result should remain running');
  assert.strictEqual(hasRunningTaskAwaitingVisibleReply(runningMessages), true, 'raw process rows without a display bubble should still show typing');
  const publicReplyRunningMessages = buildDisplayMessages({
    sessionView: 'conv-1',
    channelMessages: [],
    conversationMessages: [conversationMessages[0]],
    conversationLoading: false,
    taskMessagesById: buildTaskProcessMessageMap([runningMessages]),
  });
  assert.strictEqual(hasRunningTask(publicReplyRunningMessages), true, 'task can remain running after a public reply fragment');
  assert.strictEqual(
    hasRunningTaskAwaitingVisibleReply(publicReplyRunningMessages),
    false,
    'visible AI CLI reply fragments should hide the global typing placeholder while the task is only settling',
  );
  assert.deepStrictEqual(
    statusForTask({ status: 'running', progressCount: 0, result: null }),
    { tone: 'running', label: '等待AI响应' },
    'running task without progress should not be shown as queued',
  );

  const timeline = buildTaskTimeline([
    {
      id: 'p1',
      kind: 'ai_progress',
      task_id: 'tsk-heartbeat',
      content: 'PC 节点项目已启用本机会话隔离：代码会在你的 PC 节点上创建/复用会话 worktree 后执行。',
    },
    {
      id: 'p2',
      kind: 'ai_progress',
      task_id: 'tsk-heartbeat',
      content: '已派发到 PC 节点 node-usr_5c...33ed36，等待 Codex CLI 输出。',
    },
    {
      id: 'p3',
      kind: 'ai_progress',
      task_id: 'tsk-heartbeat',
      content: 'Codex\nCodex (node-usr_5c-dd33ed36) 正在处理中…（已等待 5s）',
    },
    {
      id: 'p4',
      kind: 'ai_progress',
      task_id: 'tsk-heartbeat',
      content: 'Codex\nCodex (node-usr_5c-dd33ed36) 正在处理中…（已等待 90s）',
    },
    {
      id: 'p5',
      kind: 'ai_progress',
      task_id: 'tsk-heartbeat',
      content: '正在同步 PC 构建产物，准备安装入口。',
    },
    {
      id: 'p6',
      kind: 'ai_progress',
      task_id: 'tsk-heartbeat',
      content: '本轮 PC 工作区没有发现 APK；不会生成安装按钮链接。',
    },
  ]);
  assert.strictEqual(timeline.heartbeatCount, 2, 'repeated waiting heartbeats should be counted');
  assert.deepStrictEqual(
    timeline.items.map((item) => item.kind),
    ['node', 'node', 'heartbeat', 'artifact', 'artifact'],
    'waiting heartbeats should collapse in place without hiding real process steps',
  );
  assert.strictEqual(
    timeline.items[2].meta,
    '累计前面全部流程总耗时 90s',
    'collapsed heartbeat should keep the latest wait duration',
  );
  assert.strictEqual(
    timelineSummary(timeline, 'tsk-heartbeat', 'tsk_hear...'),
    '5 步过程 · 合并 2 条等待状态 · 未收到 CLI 输出 · 卡点：CLI 无公开输出 · tsk_hear...',
    'timeline summary should expose compacted wait states, missing CLI output, and the current stall point',
  );
  assert.strictEqual(timeline.stage.key, 'heartbeat', 'heartbeat-only timeline should identify the CLI-output stall stage');
  assert.strictEqual(timeline.stage.label, '疑似卡在 CLI 输出前', 'long heartbeat waits should be called out as a likely stall');
  assert.strictEqual(timeline.stage.meta, '累计前面全部流程总耗时 90s', 'stage should carry the latest wait duration');
  assert.strictEqual(timeline.stage.stuck, true, 'long heartbeat-only waits should be marked as stuck');
  assert.strictEqual(timeline.coverage.heartbeat, true, 'waiting should be visible in coverage');
  assert.strictEqual(timeline.coverage.command, false, 'pure waiting should not pretend command output exists');
  assert.ok(
    timeline.diagnostics.some((item) => item.title === '只收到等待状态'),
    'pure waiting timeline should explain that no public CLI output arrived',
  );

  const toolTimeoutTimeline = buildTaskTimeline([
    {
      id: 'tt1',
      kind: 'ai_progress',
      task_id: 'tsk-tool-timeout',
      content: '{"type":"tool_call","tool":"shell","args":{"command":"powershell -ExecutionPolicy Bypass -File scripts\\\\publish-server.ps1"}}',
    },
    {
      id: 'tt2',
      kind: 'ai_progress',
      task_id: 'tsk-tool-timeout',
      content: '{"type":"runtime_status","phase":"pc_tool_result_timeout","runtime":"Codex","message":"已等待 1800 秒，但 PC 节点没有返回 shell 命令的工具结果；本轮已停止。","tool":"shell","tool_summary":"powershell -ExecutionPolicy Bypass -File scripts\\\\publish-server.ps1"}',
    },
  ]);
  assert.strictEqual(toolTimeoutTimeline.stage.key, 'tool-timeout', 'pending shell tool should use the tool-result timeout stage');
  assert.strictEqual(toolTimeoutTimeline.stage.label, '工具结果超时', 'tool-result timeout should not be labeled as missing first CLI output');
  assert.strictEqual(
    timelineSummary(toolTimeoutTimeline, 'tsk-tool-timeout', 'tsk_tool...'),
    '2 步过程 · 有命令 · 卡点：工具结果超时 · tsk_tool...',
    'summary should describe a started command waiting for a missing tool result',
  );

  const separatedHeartbeatTimeline = buildTaskTimeline([
    {
      id: 'sh1',
      kind: 'ai_progress',
      task_id: 'tsk-separated-heartbeat',
      content: '已派发到 PC 节点 node-usr_5c...33ed36，等待 Codex CLI 输出。',
    },
    {
      id: 'sh2',
      kind: 'ai_progress',
      task_id: 'tsk-separated-heartbeat',
      content: 'Codex\nAI CLI 正在处理中\n已等待 5s',
    },
    {
      id: 'sh3',
      kind: 'ai_progress',
      task_id: 'tsk-separated-heartbeat',
      content: 'PC 节点 node-usr_5c...33ed36 已确认接收，等待 Codex CLI 输出。',
    },
    {
      id: 'sh4',
      kind: 'ai_progress',
      task_id: 'tsk-separated-heartbeat',
      content: 'Codex\nAI CLI 正在处理中\n已等待 15s',
    },
  ]);
  assert.strictEqual(
    separatedHeartbeatTimeline.items.filter((item) => item.kind === 'heartbeat').length,
    1,
    'separated waiting heartbeats should still keep only the latest visible wait row',
  );
  assert.strictEqual(
    separatedHeartbeatTimeline.items[separatedHeartbeatTimeline.items.length - 1].meta,
    '累计前面全部流程总耗时 15s',
    'visible waiting row should show the latest wait duration',
  );

  const validationTimeline = buildTaskTimeline([
    {
      id: 'v1',
      kind: 'ai_progress',
      task_id: 'tsk-validation',
      content: '{"type":"tool_call","tool":"shell","args":{"command":"powershell -ExecutionPolicy Bypass -File scripts\\\\cargo-dev.ps1 check --manifest-path server\\\\Cargo.toml"}}',
    },
    {
      id: 'v1-text',
      kind: 'ai_progress',
      task_id: 'tsk-validation',
      content: 'AI 执行命令： powershell -ExecutionPolicy Bypass -File scripts\\cargo-dev.ps1 check --manifest-path server\\Cargo.toml',
    },
    {
      id: 'v2',
      kind: 'ai_progress',
      task_id: 'tsk-validation',
      content: '{"type":"tool_result","tool":"shell","result":"Finished `dev` profile target(s) in 2.33s"}',
    },
  ]);
  assert.strictEqual(validationTimeline.coverage.command, true, 'shell command should be covered');
  assert.strictEqual(validationTimeline.coverage.testRun, true, 'check/build/test command should be marked as validation');
  assert.deepStrictEqual(
    validationTimeline.items.map((item) => [item.kind, item.title]),
    [['test', '验证完成']],
    'validation command lifecycle should render as one test/build process row',
  );
  assert.strictEqual(validationTimeline.items[0].process.kind, 'test', 'validation command should expose a structured test process card');
  assert.ok(validationTimeline.items[0].process.subtitle.includes('cargo-dev.ps1 check'), 'merged test card should keep the command summary');
  assert.ok(validationTimeline.items[0].process.commandText.includes('cargo-dev.ps1 check'), 'merged test card should keep the full command separately from output');
  assert.strictEqual(validationTimeline.items[0].process.bodyLabel, '输出', 'merged test card should label command output');
  assert.ok(validationTimeline.items[0].process.body.includes('Finished `dev`'), 'merged test card should show the command output');
  assert.strictEqual(validationTimeline.items[0].process.bodyCollapsed, true, 'merged shell output should default to collapsed');
  assert.strictEqual(
    timelineSummary(validationTimeline, 'tsk-validation', 'tsk_val...'),
    '1 步过程 · 有命令 · 有测试/构建 · 当前：验证完成 · tsk_val...',
    'summary should mention command and validation coverage',
  );
  assert.strictEqual(validationTimeline.stage.label, '最后公开步骤：验证完成', 'completed validation output should become the current public stage');

  const commandEchoBeforeCallTimeline = buildTaskTimeline([
    { id: 'vecho1', kind: 'ai_progress', task_id: 'tsk-command-echo-before-call', content: 'AI 执行命令： powershell -ExecutionPolicy Bypass -File scripts\\cargo-dev.ps1 check --manifest-path server\\Cargo.toml' },
    { id: 'vecho2', kind: 'ai_progress', task_id: 'tsk-command-echo-before-call', content: '{"type":"tool_call","tool":"shell","args":{"command":"powershell -ExecutionPolicy Bypass -File scripts\\\\cargo-dev.ps1 check --manifest-path server\\\\Cargo.toml"}}' },
    { id: 'vecho3', kind: 'ai_progress', task_id: 'tsk-command-echo-before-call', content: '{"type":"tool_result","tool":"shell","result":"Finished `dev` profile target(s) in 2.33s"}' },
  ]);
  assert.deepStrictEqual(commandEchoBeforeCallTimeline.items.map((item) => [item.kind, item.title]), [['test', '验证完成']], 'shell command echo should not render as an extra process row even when it arrives before the structured tool event');

  const fileTimeline = buildTaskTimeline([
    {
      id: 'f1',
      kind: 'ai_progress',
      task_id: 'tsk-file',
      content: JSON.stringify({
        type: 'tool_call',
        tool: 'file_change',
        args: {
          changes: [
            { path: 'pc-frontend/src/features/dev/TaskTimeline.tsx' },
            { path: 'pc-frontend/src/features/dev/TaskTimeline.module.css' },
          ],
        },
      }),
    },
    {
      id: 'f2',
      kind: 'ai_progress',
      task_id: 'tsk-file',
      content: JSON.stringify({
        type: 'tool_result',
        tool: 'file_change',
        status: 'ok',
        result: 'applied',
        diff: {
          files: ['pc-frontend/src/features/dev/TaskTimeline.tsx'],
          preview: 'diff --git a/TaskTimeline.tsx b/TaskTimeline.tsx\n+<ProcessCardView />',
        },
      }),
    },
  ]);
  assert.strictEqual(fileTimeline.coverage.fileChange, true, 'file_change events should be visible in coverage');
  assert.deepStrictEqual(
    fileTimeline.items.map((item) => [item.kind, item.process && item.process.kind]),
    [['file', 'file'], ['file', 'file']],
    'file changes should render as structured file process rows',
  );
  assert.ok(fileTimeline.items[0].process.subtitle.includes('TaskTimeline.tsx'), 'file call card should list target files');
  assert.strictEqual(fileTimeline.items[1].process.bodyLabel, 'Diff 预览', 'file result card should prefer diff previews');
  assert.ok(fileTimeline.items[1].process.body.includes('ProcessCardView'), 'file result card should show diff preview content');

  const liveRecovery = recoveryViewFromEntry({
    task_id: 'tsk_live_1234567890',
    cli_name: 'Codex',
    route: 'route_a',
    status: 'running',
    recommended_action: 'wait_or_cancel',
    reason: '当前本机节点仍持有运行控制句柄。',
    can_cancel: true,
    tty_reconnect: {
      supported: false,
      user_label: '原 CLI 终端不可重接',
      reason: '浏览器不能重新接管原始 CLI TTY。',
    },
  });
  assert.strictEqual(liveRecovery.canCancel, true, 'live control recovery should expose stop action');
  assert.strictEqual(liveRecovery.canContinue, false, 'live control should wait/cancel instead of snapshot continue');
  assert.strictEqual(liveRecovery.stageTitle, '本机正在执行', 'live control should explain the current execution stage');
  assert.ok(liveRecovery.facts.some((fact) => fact.value === '可停止'), 'live recovery facts should say the task can be stopped');

  const staleLiveRecovery = recoveryViewFromEntry({
    task_id: 'tsk_stale_live_1234567890',
    cli_name: 'Codex',
    route: 'route_a',
    status: 'running',
    recommended_action: 'wait_or_cancel',
    can_cancel: true,
    last_heartbeat_ms: 1_000,
    now_ms: 75_000,
  });
  assert.strictEqual(staleLiveRecovery.stageTitle, '疑似卡在本机节点', 'stale live control should identify the likely stuck point');
  assert.strictEqual(staleLiveRecovery.stageTone, 'failed', 'stale live control should use a warning tone');
  assert.strictEqual(staleLiveRecovery.stageMeta, '1分钟前', 'stale live control should show how old the heartbeat is');
  assert.ok(staleLiveRecovery.facts.some((fact) => fact.label === '心跳' && fact.tone === 'failed'), 'stale heartbeat should be visible as a failed fact');

  const detachedRecovery = recoveryViewFromTask({
    task_id: 'tsk_detached_1234567890',
    cli_name: 'Codex',
    route: 'route_a',
    status: 'running',
    cwd: 'D:/demo/project',
    attach: {
      status: 'detached',
      reason: '本机 journal 显示任务未终态，但当前节点已没有运行句柄，只能基于快照继续。',
    },
    resume: {
      status: 'detached',
      next_action: 'continue_from_snapshot',
      can_cancel: false,
      can_replay_journal_events: true,
      reason: '原进程控制句柄已经丢失，需要新开一轮任务并先检查工作区状态。',
      tty_reattach: {
        supported: false,
        user_label: '原 CLI 终端不可重接',
        reason: '原始 CLI TTY 已经脱离当前页面。',
      },
      tool_approval_recovery: {
        status: 'lost_after_restart',
        journal_pending_count: 1,
        reason: '历史审批卡必须失效。',
      },
    },
  });
  assert.strictEqual(detachedRecovery.canCancel, false, 'detached task should not expose cancel');
  assert.strictEqual(detachedRecovery.canContinue, true, 'detached task should expose snapshot continue');
  assert.ok(detachedRecovery.continuePrompt.includes('不要批准已经失效的旧审批'), 'continue draft should guard stale approvals');
  assert.ok(detachedRecovery.facts.some((fact) => fact.value.includes('审批已失效')), 'detached recovery should explain lost approval waiter');

  const parallelOverview = buildAgentRunParallelOverview({
    recoveryEntry: {
      task_id: 'tsk-live-a',
      cli_name: 'Codex',
      route: 'route_a',
      status: 'running',
      recommended_action: 'wait_or_cancel',
      can_cancel: true,
    },
    activeControls: [
      { task_id: 'tsk-live-a', cli_name: 'Codex', route: 'route_a', can_cancel: true },
      { task_id: 'tsk-live-b', cli_name: 'Codex', route: 'route_a', can_cancel: true },
    ],
    sidecarSessions: [
      {
        task_id: 'tsk-sidecar-d',
        cli_name: 'Codex',
        route: 'route_a',
        capabilities: { terminal_attach: true, cancel: true },
      },
    ],
    recentTasks: [
      {
        task_id: 'tsk-detached-c',
        cli_name: 'Codex',
        route: 'route_a',
        attach: { status: 'detached' },
        resume: {
          status: 'detached',
          next_action: 'continue_from_snapshot',
          can_cancel: false,
          can_replay_journal_events: true,
          tool_approval_recovery: { status: 'lost_after_restart', journal_pending_count: 1 },
        },
      },
    ],
  });
  assert.strictEqual(parallelOverview.counts.total, 4, 'parallel overview should dedupe the recommended live task');
  assert.strictEqual(parallelOverview.counts.active, 2, 'parallel overview should count active live controls');
  assert.strictEqual(parallelOverview.counts.staleActive, 0, 'parallel overview should not mark fresh controls as stale');
  assert.strictEqual(parallelOverview.counts.sidecar, 1, 'parallel overview should count sidecar-reconnectable tasks');
  assert.strictEqual(parallelOverview.counts.recoverable, 1, 'parallel overview should count snapshot-continuable tasks');
  assert.strictEqual(parallelOverview.counts.staleApproval, 1, 'parallel overview should surface stale approval waiters');
  assert.strictEqual(parallelOverview.continuity.mode, 'sidecar_reconnect', 'parallel overview should explain restart sidecar continuity');
  assert.ok(parallelOverview.continuity.facts.some((fact) => fact.label === '可重接'), 'parallel overview continuity should expose sidecar reconnect count');
  assert.deepStrictEqual(
    parallelOverview.views.map((view) => view.taskId),
    ['tsk-live-a', 'tsk-live-b', 'tsk-sidecar-d', 'tsk-detached-c'],
    'parallel overview should keep running tasks ahead of sidecar and recoverable detached tasks',
  );

  const staleNodeOverview = buildAgentRunParallelOverview({
    activeControls: [
      {
        task_id: 'tsk-stale-node',
        cli_name: 'Codex',
        route: 'route_a',
        can_cancel: true,
        last_heartbeat_ms: 1_000,
      },
    ],
    nowMs: 75_000,
  });
  assert.strictEqual(staleNodeOverview.counts.staleActive, 1, 'parallel overview should count stale active controls');
  assert.strictEqual(staleNodeOverview.continuity.title, '节点心跳疑似断开', 'stale node overview should lead with reconnect diagnosis');
  assert.strictEqual(staleNodeOverview.continuity.tone, 'failed', 'stale node overview should use failed tone');
  assert.ok(staleNodeOverview.continuity.facts.some((fact) => fact.label === '陈旧心跳' && fact.tone === 'failed'), 'stale node overview should show stale heartbeat facts');

  const assistantOutputTimeline = buildTaskTimeline([
    {
      id: 'assistant-progress',
      kind: 'ai_progress',
      task_id: 'tsk-assistant-progress',
      content: '{"type":"assistant_message","text":"我会先读取规则入口。","model_used":"codex"}',
    },
  ]);
  assert.strictEqual(
    assistantOutputTimeline.items.length,
    0,
    'assistant_message events should not appear inside the folded process timeline',
  );
  assert.strictEqual(
    assistantOutputTimeline.coverage.assistantEvent,
    true,
    'timeline coverage should still record that the AI CLI produced assistant output',
  );
  const settlingTimeline = buildTaskTimeline(
    [
      {
        id: 'settle-1',
        kind: 'ai_progress',
        task_id: 'tsk-settling',
        content: '{"type":"tool_call","tool":"shell","args":{"command":"Get-Date"}}',
      },
      {
        id: 'settle-2',
        kind: 'ai_progress',
        task_id: 'tsk-settling',
        content: '{"type":"tool_result","tool":"shell","result":"exit=0 2026-07-04 00:50:29"}',
      },
      {
        id: 'settle-3',
        kind: 'ai_progress',
        task_id: 'tsk-settling',
        content: 'Codex\\nCodex (GPT-5.5 · 推理 xhigh) 正在处理中…（已等待 25s）',
      },
    ],
    undefined,
    { assistantNoteCount: 1 },
  );
  assert.strictEqual(settlingTimeline.stage.key, 'assistant', 'public reply bubbles should keep later waiting heartbeats from becoming the main stage');
  assert.strictEqual(settlingTimeline.stage.label, '回复已显示，等待收尾', 'settling state should explain that the answer is already visible');
  assert.strictEqual(
    timelineSummary(settlingTimeline, 'tsk-settling', 'tsk_set...'),
    '2 步过程 · 有命令 · 当前：回复已显示，等待收尾 · tsk_set...',
    'summary should describe visible reply settling instead of saying the task is still only processing',
  );
  assert.strictEqual(settlingTimeline.items[0].process.chips.some((chip) => chip.label === 'exit=0'), true, 'merged command result should show exit status as a chip');
  assert.strictEqual(settlingTimeline.items[0].process.commandText, 'Get-Date', 'merged command result should preserve the command line separately');
  assert.strictEqual(settlingTimeline.items[0].process.body, '2026-07-04 00:50:29', 'merged command result should not repeat exit status inside the output body');
  assert.strictEqual(settlingTimeline.items[0].process.bodyCollapsed, true, 'merged command output should default to collapsed');
  const finalEchoTimeline = buildTaskTimeline(
    [
      {
        id: 'echo-1',
        kind: 'ai_progress',
        task_id: 'tsk-final-echo',
        content: '正在同步 PC 构建产物，准备安装入口。',
      },
      {
        id: 'echo-2',
        kind: 'ai_progress',
        task_id: 'tsk-final-echo',
        content: '我看完项目规则后，建议优先做需求成熟度判断。你的项目定位很清楚：让用户通过持续讨论把模糊想法变成 APK。把项目 APK 图标做成硬链路。',
      },
    ],
    {
      id: 'final-echo',
      kind: 'ai_result',
      task_id: 'tsk-final-echo',
      content: '你的项目定位很清楚：让用户通过持续讨论把模糊想法变成 APK。把项目 APK 图标做成硬链路。',
    },
  );
  assert.deepStrictEqual(
    finalEchoTimeline.items.map((item) => item.title),
    ['同步 PC 构建产物'],
    'progress rows that echo the final answer should not duplicate the final reply in the process timeline',
  );

  const plainMessages = [
    { id: 'chat-1', role: 'user', user_id: 'u-1', content: '第一条' },
    { id: 'chat-2', role: 'user', user_id: 'u-1', content: '第二条' },
  ];
  const plainGroups = buildMessageGroups(plainMessages, false);
  assert.strictEqual(plainGroups[1].grouped, true, 'normal chat grouping should keep same-sender grouping');

  console.log('pc-frontend message-flow tests passed');
} finally {
  require.extensions['.ts'] = originalTsLoader;
}
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
