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
      id: 'order-result-1',
      kind: 'ai_progress',
      task_id: 'tsk-order',
      content: '{"type":"tool_result","tool":"shell","result":"CODEX Project Entry"}',
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
  ]);

  const displayOrder = buildTimelineDisplay(mixedTimeline, {}).primaryBlocks.map((block) => (
    block.type === 'commands'
      ? block.items.map((item) => item.process && item.process.commandText).join('\n')
      : block.item.kind === 'node' ? block.item.title : block.item.detail || block.item.title
  ));

  assert.deepStrictEqual(
    displayOrder,
    [
      '已派发到 PC 节点',
      '第一步：先读取入口规则。',
      'Get-Content CODEX.md',
      '第二步：再读取服务器版本。',
      'curl.exe http://127.0.0.1:8080/api/server/version',
    ],
    'expanded timeline should keep mixed node, public replies, and commands in chronological order',
  );

  console.log('pc-frontend task-timeline display tests passed');
} finally {
  if (originalTsLoader) require.extensions['.ts'] = originalTsLoader;
  else delete require.extensions['.ts'];
}
