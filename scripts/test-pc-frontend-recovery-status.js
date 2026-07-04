const assert = require('assert');
const fs = require('fs');
const path = require('path');

const repoRoot = path.resolve(__dirname, '..');
const pcRoot = path.join(repoRoot, 'pc-frontend');
const ts = require(path.join(pcRoot, 'node_modules', 'typescript'));

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

const { statusForTask } = require(path.join(pcRoot, 'src', 'features', 'dev', 'devTaskUtils.ts'));
const { buildTaskTimeline } = require(path.join(pcRoot, 'src', 'features', 'dev', 'taskTimelineModel.ts'));

assert.deepStrictEqual(
  statusForTask({ status: 'recovering', progressCount: 1, result: null }),
  { tone: 'running', label: '正在恢复通信' },
  'recovering task should stay non-terminal and use explicit recovery copy',
);

const serverUpdateTimeline = buildTaskTimeline([{
  id: 'srv-update',
  kind: 'ai_progress',
  task_id: 'tsk-server-update',
  content: JSON.stringify({
    type: 'runtime_status',
    phase: 'server_updating',
    runtime: '一龙',
    status: 'recovering',
    message: '服务器正在更新升级，通信临时中断，会自动恢复。任务现场已保留，正在等待 Win 端节点重连和过程回放。',
    auto_recover: true,
  }),
}]);
assert.strictEqual(serverUpdateTimeline.stage.key, 'server-update');
assert.strictEqual(serverUpdateTimeline.stage.label, '服务器正在更新升级');
assert.ok(serverUpdateTimeline.stage.detail.includes('通信临时中断，会自动恢复'));
assert.strictEqual(serverUpdateTimeline.stage.stuck, false);
assert.ok(!serverUpdateTimeline.diagnostics.some((item) => item.title === '只收到等待状态'));

const winUpdateTimeline = buildTaskTimeline([{
  id: 'win-update',
  kind: 'ai_progress',
  task_id: 'tsk-win-update',
  content: JSON.stringify({
    type: 'runtime_status',
    phase: 'win_client_updating',
    runtime: 'Win 端',
    status: 'recovering',
    message: 'Win 端正在更新升级，通信临时中断，会自动恢复。',
    auto_recover: true,
  }),
}]);
assert.strictEqual(winUpdateTimeline.stage.key, 'win-update');
assert.strictEqual(winUpdateTimeline.stage.label, 'Win 端正在更新升级');
assert.ok(winUpdateTimeline.stage.detail.includes('通信临时中断，会自动恢复'));

console.log('pc-frontend recovery-status tests passed');
