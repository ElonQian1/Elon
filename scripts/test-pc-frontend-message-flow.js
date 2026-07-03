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
    { id: 'pcm-assistant-1', kind: 'ai_progress', task_id: 'tsk-1', content: '{"type":"assistant_message","text":"我先读取规则入口。","model_used":"codex"}' },
    { id: 'pcm-progress', kind: 'ai_progress', task_id: 'tsk-1', content: '{"type":"tool_call","tool":"rg"}' },
    { id: 'pcm-assistant-2', kind: 'ai_progress', task_id: 'tsk-1', content: '{"type":"assistant_message","text":"规则已读完，接着运行只读命令。","model_used":"codex"}' },
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
  assert.strictEqual(merged[2].kind, 'assistant', 'Codex reply fragments should render as assistant messages');
  assert.strictEqual(merged[2].task_id, undefined, 'reply fragments should not be swallowed by task grouping');

  const groups = buildMessageGroups(merged, true);
  assert.strictEqual(groups.length, 6, 'assistant bubbles should split process groups into visible conversation turns');
  assert.deepStrictEqual(
    groups.map((group) => group.type),
    ['single', 'task', 'single', 'task', 'single', 'task'],
    'process groups should be consecutive segments, not a global task_id bucket',
  );
  assert.deepStrictEqual(
    groups.map((group) => group.type === 'task' ? group.messages.map((message) => message.id) : [group.message.id]),
    [
      ['msg-user'],
      ['pcm-task'],
      ['assistant-progress-pcm-assistant-1'],
      ['pcm-progress'],
      ['assistant-progress-pcm-assistant-2'],
      ['pcm-result'],
    ],
    'task process, Codex reply fragments, and final result should stay in chronological order',
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
    'assistant conversation reply should remain a normal bubble when channel ai_result is not available yet',
  );
  assert.strictEqual(hasRunningTask(mergedWithAssistantFallback), false, 'assistant fallback should close the task');
  const fallbackGroups = buildMessageGroups(mergedWithAssistantFallback, true);
  assert.strictEqual(fallbackGroups.length, 5, 'assistant fallback should not be folded into the process panel');
  assert.deepStrictEqual(
    fallbackGroups.map((group) => group.type === 'task' ? group.messages.map((message) => message.id) : [group.message.id]),
    [['msg-user'], ['pcm-task'], ['assistant-progress-pcm-assistant-1'], ['pcm-progress'], ['msg-assistant']],
    'fallback final answer should stay in the chat stream after the process rows',
  );

  const conversationOnlyGroups = buildMessageGroups(conversationMessages, true);
  assert.strictEqual(conversationOnlyGroups.length, 2, 'task-linked conversation rows should remain normal chat bubbles');
  assert.deepStrictEqual(
    conversationOnlyGroups.map((group) => group.type === 'task' ? group.messages.map((message) => message.id) : [group.message.id]).flat(),
    ['msg-user', 'msg-assistant'],
    'conversation-only task messages should preserve user and assistant turns without a process group',
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
    '5 步过程 · 合并 2 条等待状态 · 未收到 CLI 输出 · tsk_hear...',
    'timeline summary should expose compacted wait states and missing CLI output',
  );
  assert.strictEqual(timeline.coverage.heartbeat, true, 'waiting should be visible in coverage');
  assert.strictEqual(timeline.coverage.command, false, 'pure waiting should not pretend command output exists');
  assert.ok(
    timeline.diagnostics.some((item) => item.title === '只收到等待状态'),
    'pure waiting timeline should explain that no public CLI output arrived',
  );

  const validationTimeline = buildTaskTimeline([
    {
      id: 'v1',
      kind: 'ai_progress',
      task_id: 'tsk-validation',
      content: '{"type":"tool_call","tool":"shell","args":{"command":"powershell -ExecutionPolicy Bypass -File scripts\\\\cargo-dev.ps1 check --manifest-path server\\\\Cargo.toml"}}',
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
    [['test', '运行测试/构建'], ['test', '验证完成']],
    'validation commands should render as test/build process rows',
  );
  assert.strictEqual(
    timelineSummary(validationTimeline, 'tsk-validation', 'tsk_val...'),
    '2 步过程 · 有命令 · 有测试/构建 · tsk_val...',
    'summary should mention command and validation coverage',
  );
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
    'timeline coverage should still record that Codex produced assistant output',
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
