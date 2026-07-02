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

try {
  const {
    buildDisplayMessages,
    buildMessageGroups,
    buildTaskProcessMessageMap,
    hasRunningTask,
  } = require(path.join(pcRoot, 'src', 'features', 'conversation', 'messageFlow.ts'));
  const {
    statusForTask,
  } = require(path.join(pcRoot, 'src', 'features', 'dev', 'devTaskUtils.ts'));
  const {
    buildTaskTimeline,
    timelineSummary,
  } = require(path.join(pcRoot, 'src', 'features', 'dev', 'taskTimelineModel.ts'));

  const taskMessages = [
    { id: 'pcm-task', kind: 'ai_task', task_id: 'tsk-1', content: '发起 AI 开发任务：修复会话 UI' },
    { id: 'pcm-progress', kind: 'ai_progress', task_id: 'tsk-1', content: '{"type":"tool_call","tool":"rg"}' },
    { id: 'pcm-result', kind: 'ai_result', task_id: 'tsk-1', content: '最终答案：已修复。' },
  ];
  const conversationMessages = [
    { id: 'msg-user', role: 'user', conversation_id: 'conv-1', task_id: 'tsk-1', content: '修复会话 UI' },
    { id: 'msg-assistant', role: 'assistant', conversation_id: 'conv-1', task_id: 'tsk-1', content: '已完成。' },
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
    ['msg-user', 'pcm-task', 'pcm-progress', 'pcm-result'],
    'member conversation task rows should keep the real user turn and attach structured task process rows',
  );
  assert.strictEqual(hasRunningTask(merged), false, 'terminal ai_result should close the task');

  const groups = buildMessageGroups(merged, true);
  assert.strictEqual(groups.length, 1, 'structured task process should render as one task group');
  assert.strictEqual(groups[0].type, 'task');
  assert.strictEqual(groups[0].taskId, 'tsk-1');
  assert.deepStrictEqual(
    groups[0].messages.map((message) => message.id),
    ['msg-user', 'pcm-task', 'pcm-progress', 'pcm-result'],
    'task group should preserve the real user request, progress, and final result together',
  );

  const inFlightTaskMessages = taskMessages.slice(0, 2);
  const mergedWithAssistantFallback = buildDisplayMessages({
    sessionView: 'conv-1',
    channelMessages: [],
    conversationMessages,
    conversationLoading: false,
    taskMessagesById: buildTaskProcessMessageMap([inFlightTaskMessages]),
  });
  assert.deepStrictEqual(
    mergedWithAssistantFallback.map((message) => message.id),
    ['msg-user', 'pcm-task', 'pcm-progress', 'task-result-msg-assistant'],
    'assistant conversation reply should remain visible when channel ai_result is not available yet',
  );
  assert.strictEqual(
    mergedWithAssistantFallback[3].kind,
    'ai_result',
    'assistant fallback should render inside the structured task group as the final result',
  );
  assert.strictEqual(hasRunningTask(mergedWithAssistantFallback), false, 'assistant fallback should close the task');
  const fallbackGroups = buildMessageGroups(mergedWithAssistantFallback, true);
  assert.strictEqual(fallbackGroups.length, 1, 'assistant fallback should stay connected to the task group');
  assert.deepStrictEqual(
    fallbackGroups[0].messages.map((message) => message.id),
    ['msg-user', 'pcm-task', 'pcm-progress', 'task-result-msg-assistant'],
    'task group should include the fallback final answer',
  );

  const conversationOnlyGroups = buildMessageGroups(conversationMessages, true);
  assert.strictEqual(conversationOnlyGroups.length, 1, 'task-linked conversation rows should form one task thread');
  assert.strictEqual(conversationOnlyGroups[0].type, 'task');
  assert.deepStrictEqual(
    conversationOnlyGroups[0].messages.map((message) => message.id),
    ['msg-user', 'msg-assistant'],
    'conversation-only task thread should keep user and assistant turns connected',
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

  const runningMessages = taskMessages.slice(0, 2);
  assert.strictEqual(hasRunningTask(runningMessages), true, 'task without result should remain running');
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
    '已等待 90s',
    'collapsed heartbeat should keep the latest wait duration',
  );
  assert.strictEqual(
    timelineSummary(timeline, 'tsk-heartbeat', 'tsk_hear...'),
    '5 步过程 · 合并 2 条等待状态 · tsk_hear...',
    'timeline summary should expose compacted step count and heartbeat compaction',
  );
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
