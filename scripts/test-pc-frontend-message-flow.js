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
    ['pcm-task', 'pcm-progress', 'pcm-result'],
    'member conversation task rows should be replaced by structured task process rows',
  );
  assert.strictEqual(hasRunningTask(merged), false, 'terminal ai_result should close the task');

  const groups = buildMessageGroups(merged, true);
  assert.strictEqual(groups.length, 1, 'structured task process should render as one task group');
  assert.strictEqual(groups[0].type, 'task');
  assert.strictEqual(groups[0].taskId, 'tsk-1');
  assert.deepStrictEqual(
    groups[0].messages.map((message) => message.id),
    ['pcm-task', 'pcm-progress', 'pcm-result'],
    'task group should preserve user request, progress, and final result together',
  );

  const runningMessages = taskMessages.slice(0, 2);
  assert.strictEqual(hasRunningTask(runningMessages), true, 'task without result should remain running');

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
