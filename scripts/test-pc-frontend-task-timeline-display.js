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
    buildTaskTimeline,
  } = require(path.join(pcRoot, 'src', 'features', 'dev', 'taskTimelineModel.ts'));
  const {
    buildTimelineDisplay,
  } = require(path.join(pcRoot, 'src', 'features', 'dev', 'taskTimelineDisplayModel.ts'));
  const {
    taskStageActionModel,
  } = require(path.join(pcRoot, 'src', 'features', 'dev', 'taskStageActionModel.ts'));
  const {
    taskCompletionMetaModel,
  } = require(path.join(pcRoot, 'src', 'features', 'dev', 'taskCompletionMetaModel.ts'));
  const {
    taskTerminalActionModel,
  } = require(path.join(pcRoot, 'src', 'features', 'dev', 'taskTerminalActionModel.ts'));

  const mixedTimeline = buildTaskTimeline([
    {
      id: 'order-dispatch',
      kind: 'ai_progress',
      task_id: 'tsk-order',
      content: '{"type":"pc_dispatch_started","status":"running","message":"已获得 PC 会话执行权。"}',
    },
    {
      id: 'order-note-1',
      kind: 'ai_progress',
      task_id: 'tsk-order',
      assistant_progress_event: true,
      content: '第一步：先读取入口规则。',
    },
    {
      id: 'order-cmd-1',
      kind: 'ai_progress',
      task_id: 'tsk-order',
      content: '{"type":"tool_call","tool":"shell","args":{"command":"Get-Content CODEX.md"}}',
    },
    {
      id: 'order-cmd-1-legacy-echo',
      kind: 'ai_progress',
      task_id: 'tsk-order',
      content: 'AI 执行命令：Get-Content CODEX.md',
    },
    {
      id: 'order-result-1',
      kind: 'ai_progress',
      task_id: 'tsk-order',
      content: '{"type":"tool_result","tool":"shell","result":"CODEX Project Entry"}',
    },
    {
      id: 'order-result-1-legacy-echo',
      kind: 'ai_progress',
      task_id: 'tsk-order',
      content: '命令执行完毕',
    },
    {
      id: 'order-note-2',
      kind: 'ai_progress',
      task_id: 'tsk-order',
      assistant_progress_event: true,
      content: '第二步：再读取服务器版本。',
    },
    {
      id: 'order-cmd-2',
      kind: 'ai_progress',
      task_id: 'tsk-order',
      content: '{"type":"tool_call","tool":"shell","args":{"command":"curl.exe http://127.0.0.1:8080/api/server/version"}}',
    },
    {
      id: 'order-result-2',
      kind: 'ai_progress',
      task_id: 'tsk-order',
      content: '{"type":"tool_result","tool":"shell","result":"{\\"versionName\\":\\"0.3.1365\\"}"}',
    },
    {
      id: 'order-result-2-legacy-echo',
      kind: 'ai_progress',
      task_id: 'tsk-order',
      content: '命令执行完毕。',
    },
  ]);

  const display = buildTimelineDisplay(mixedTimeline, {});
  const displayOrder = display.primaryBlocks.map((block) => (
    block.type === 'commands'
      ? block.items.map((item) => item.process && item.process.commandText).join('\n')
      : block.item.kind === 'node' ? block.item.title : block.item.detail || block.item.title
  ));

  assert.deepStrictEqual(
    displayOrder,
    [
      '第一步：先读取入口规则。',
      'Get-Content CODEX.md',
      '第二步：再读取服务器版本。',
      'curl.exe http://127.0.0.1:8080/api/server/version',
    ],
    'expanded timeline should keep public replies and commands in chronological order',
  );

  assert.deepStrictEqual(
    taskStageActionModel('heartbeat', 'running', false),
    { canContinue: false, canOpenNode: false, continueLabel: '' },
    'normal heartbeat waiting must not offer a continue action',
  );
  assert.deepStrictEqual(
    taskStageActionModel('heartbeat', 'failed', true),
    { canContinue: true, canOpenNode: true, continueLabel: '检查并继续' },
    'only a stale heartbeat should offer recovery actions',
  );
  assert.deepStrictEqual(
    taskStageActionModel('recovery-timeout', 'failed', true),
    { canContinue: true, canOpenNode: true, continueLabel: '重试恢复' },
  );
  assert.deepStrictEqual(
    taskStageActionModel('tool-timeout', 'failed', true),
    { canContinue: true, canOpenNode: true, continueLabel: '重试任务' },
  );
  assert.deepStrictEqual(
    taskStageActionModel('finished', 'failed', false),
    { canContinue: false, canOpenNode: false, continueLabel: '' },
    'terminal recovery belongs below the failure reason, not inside the processed status row',
  );
  assert.deepStrictEqual(
    taskStageActionModel('latest', 'running', false),
    { canContinue: false, canOpenNode: false, continueLabel: '' },
    'an active command failure stays under AI control instead of asking the user to intervene',
  );

  assert.deepStrictEqual(
    taskCompletionMetaModel({
      items: [{
        event: {
          type: 'usage',
          model: 'gpt-5.5',
          input_tokens: 18000,
          output_tokens: 2000,
        },
      }],
    }),
    { model: 'gpt-5.5', usage: '输入 18,000 · 输出 2,000' },
    'structured usage should render as completion metadata',
  );

  assert.deepStrictEqual(
    taskTerminalActionModel(
      'canceled',
      'AI 开发任务通信中断。任务已停止以避免重复执行。',
      'PC 节点通信中断。',
    ),
    { visible: true, label: '继续任务', requiresNode: true },
    'a disconnected PC task should wait for the node and then resume',
  );
  assert.deepStrictEqual(
    taskTerminalActionModel('canceled', '用户已停止本轮任务。'),
    { visible: true, label: '继续任务', requiresNode: false },
    'an accidentally canceled task should be resumable',
  );
  assert.deepStrictEqual(
    taskTerminalActionModel('failed', 'PC CLI 没有返回收尾回复；本轮结果无法确认完成。'),
    { visible: true, label: '继续生成回复', requiresNode: true },
  );
  assert.deepStrictEqual(
    taskTerminalActionModel('failed', '平台 AI runtime 返回 502 Bad Gateway。'),
    { visible: true, label: '重试任务', requiresNode: false },
  );
  assert.deepStrictEqual(
    taskTerminalActionModel('done', '任务已完成。'),
    { visible: false, label: '', requiresNode: false },
  );
  assert.deepStrictEqual(
    taskCompletionMetaModel({
      items: [{
        event: {
          type: 'usage',
          model: 'codex',
          message: '输入 18k tokens，输出 2k tokens。',
        },
      }],
    }),
    { model: 'codex', usage: '输入 18k tokens，输出 2k tokens。' },
    'legacy usage messages should remain readable below the final reply',
  );

  assert.deepStrictEqual(
    display.grouped.connection.map((item) => item.title),
    [],
    'healthy connection events should not add a second diagnostic fold',
  );

  const timeoutTimeline = buildTaskTimeline([
    {
      id: 'timeout-dispatch',
      kind: 'ai_progress',
      task_id: 'tsk-timeout',
      content: '{"type":"pc_dispatch_started","status":"running","message":"已获得 PC 会话执行权。"}',
    },
    {
      id: 'timeout-status',
      kind: 'ai_progress',
      task_id: 'tsk-timeout',
      content: '{"type":"runtime_status","status":"error","phase":"pc_cli_no_output_timeout","message":"没有收到公开输出。"}',
    },
  ]);
  const timeoutDisplay = buildTimelineDisplay(timeoutTimeline, {});
  assert.deepStrictEqual(
    timeoutDisplay.grouped.connection.map((item) => item.title),
    ['已派发到 PC 节点'],
    'stuck tasks should retain connection evidence inside diagnostics',
  );

  const displayWithoutCommands = buildTimelineDisplay(mixedTimeline, { hideCommands: true });
  assert.deepStrictEqual(
    displayWithoutCommands.primaryBlocks.map((block) => (
      block.type === 'commands'
        ? 'unexpected command block'
        : block.item.detail || block.item.title
    )),
    [
      '第一步：先读取入口规则。',
      '第二步：再读取服务器版本。',
    ],
    'timeline should be able to hide command blocks when command summaries are already surfaced',
  );

  console.log('pc-frontend task-timeline display tests passed');
} finally {
  if (originalTsLoader) require.extensions['.ts'] = originalTsLoader;
  else delete require.extensions['.ts'];
}
