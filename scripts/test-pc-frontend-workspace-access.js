const assert = require('assert');
const fs = require('fs');
const path = require('path');

const repoRoot = path.resolve(__dirname, '..');
const pcRoot = path.join(repoRoot, 'pc-frontend');
const ts = require(path.join(pcRoot, 'node_modules', 'typescript'));
const originalTsLoader = require.extensions['.ts'];

require.extensions['.ts'] = function loadTsModule(module, filename) {
  const source = fs.readFileSync(filename, 'utf8');
  const output = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.CommonJS,
      target: ts.ScriptTarget.ES2020,
    },
    fileName: filename,
  });
  module._compile(output.outputText, filename);
};

try {
  const {
    runtimePermissionLabel,
    sameWorkspacePath,
    workspaceAccessStatus,
  } = require(path.join(pcRoot, 'src', 'features', 'projects', 'workspaceAccessModel.ts'));

  assert.strictEqual(sameWorkspacePath('E:\\一龙项目', 'e:/一龙项目/'), true, 'Windows paths should match across slash and case variants');
  assert.strictEqual(sameWorkspacePath('/opt/Elon', '/opt/elon'), false, 'non-Windows paths should remain case-sensitive');
  assert.strictEqual(runtimePermissionLabel('danger_full_access'), '完整本机命令行');
  assert.strictEqual(runtimePermissionLabel('full_access'), '完全访问');
  assert.strictEqual(runtimePermissionLabel('project_write'), '项目目录写入');

  const missing = workspaceAccessStatus({
    loadState: 'ready',
    fullAccessRequired: true,
    localNodeIsBound: true,
    hasBoundNode: true,
  });
  assert.strictEqual(missing.label, '等待本机确认', 'full access without a local grant must remain blocked');

  const granted = workspaceAccessStatus({
    loadState: 'ready',
    matchingGrant: { project_id: 'elon-self', workspace_path: 'E:\\一龙项目' },
    fullAccessRequired: true,
    localNodeIsBound: true,
    hasBoundNode: true,
  });
  assert.strictEqual(granted.label, '已授权', 'matching local grant should make Route A ready');

  const panelSource = fs.readFileSync(path.join(pcRoot, 'src', 'features', 'projects', 'WorkspaceAccessPanel.tsx'), 'utf8');
  const detailSource = fs.readFileSync(path.join(pcRoot, 'src', 'features', 'projects', 'ProjectDetailPage.tsx'), 'utf8');
  for (const endpoint of ['/api/project-folder/pick', '/api/register-project', '/api/full-access/grants']) {
    assert.ok(panelSource.includes(endpoint), `workspace access panel should call ${endpoint}`);
  }
  assert.ok(panelSource.includes('confirm_full_access: true'), 'local grant must include explicit full-access confirmation');
  assert.ok(detailSource.includes('<WorkspaceAccessPanel'), 'project workspace tab should render the access panel');

  console.log('pc-frontend workspace-access tests passed');
} finally {
  require.extensions['.ts'] = originalTsLoader;
}
