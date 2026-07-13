const assert = require('assert')
const fs = require('fs')
const path = require('path')

const repoRoot = path.resolve(__dirname, '..')
const pcRoot = path.join(repoRoot, 'pc-frontend')
const typescriptPath = path.join(pcRoot, 'node_modules', 'typescript')
const ts = fs.existsSync(typescriptPath) ? require(typescriptPath) : require('typescript')

require.extensions['.ts'] = function loadTsModule(module, filename) {
  const source = fs.readFileSync(filename, 'utf8')
  const output = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.CommonJS,
      target: ts.ScriptTarget.ES2020,
      esModuleInterop: true,
    },
    fileName: filename,
  })
  module._compile(output.outputText, filename)
}

const { recoverySnapshotPhase } = require(path.join(
  pcRoot,
  'src',
  'features',
  'conversation',
  'taskRecoverySnapshotModel.ts',
))

assert.strictEqual(recoverySnapshotPhase({
  taskStatus: 'running',
  journalStatus: 'available',
  resume: { status: 'live', next_action: 'wait_or_cancel' },
  attach: { status: 'live' },
}), null, 'a healthy first task with a live journal must not be mislabeled as connection recovery')

assert.strictEqual(recoverySnapshotPhase({
  taskStatus: 'running',
  journalStatus: 'available',
  resume: { status: 'sidecar_recoverable', next_action: 'attach_sidecar' },
  attach: { status: 'sidecar_recoverable' },
}), null, 'a normally managed sidecar must not imply that the first turn was interrupted')

assert.strictEqual(
  recoverySnapshotPhase({ taskStatus: 'recovering', journalStatus: 'available' }),
  'connection_recovering',
  'an explicitly recovering cloud task should keep the recovery stage',
)
assert.strictEqual(
  recoverySnapshotPhase({ taskStatus: 'running', journalStatus: 'agent_offline_or_timeout' }),
  'connection_recovering',
  'an offline task node should surface connection recovery',
)
assert.strictEqual(recoverySnapshotPhase({
  taskStatus: 'running',
  journalStatus: 'available',
  resume: { status: 'detached', next_action: 'continue_from_snapshot' },
  attach: { status: 'detached' },
}), 'resume_required', 'a detached task should require an explicit snapshot continuation')

console.log('pc-frontend task recovery snapshot tests passed')
