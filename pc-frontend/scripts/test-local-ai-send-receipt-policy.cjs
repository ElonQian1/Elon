const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const filename = path.resolve(
  __dirname,
  '../src/features/user-browser/localAiSendReceiptPolicy.ts',
)
const output = ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  fileName: filename,
}).outputText
const compiled = new Module(filename, module)
compiled.filename = filename
compiled.paths = module.paths
compiled._compile(output, filename)

const { localAiSendReceiptDecision } = compiled.exports
const requestId = 'mcp_current1'
const receipt = (ok, id = requestId) => ({
  type: 'command_result',
  action: 'send_prompt',
  ok,
  detail: '',
  requestId: id,
})

assert.equal(localAiSendReceiptDecision({ commandQueued: false, requestId: '' }), 'restore')
assert.equal(localAiSendReceiptDecision({ commandQueued: true, requestId }), 'reconcile')
assert.equal(localAiSendReceiptDecision({
  commandQueued: true,
  requestId,
  receipt: receipt(true, 'mcp_stale1'),
}), 'reconcile')
assert.equal(localAiSendReceiptDecision({
  commandQueued: true,
  requestId,
  receipt: receipt(false),
}), 'rejected')
assert.equal(localAiSendReceiptDecision({
  commandQueued: true,
  requestId,
  receipt: receipt(true),
}), 'accepted')

const dispatchSource = fs.readFileSync(path.resolve(
  __dirname,
  '../src/features/user-browser/dispatchPreparedLocalAiPrompt.ts',
), 'utf8')
assert.match(dispatchSource, /commandQueued = true[\s\S]*?waitForLocalAiAdapterResult/)
assert.match(dispatchSource, /decision === 'reconcile'[\s\S]*?reconcileUncertainSend\(\)/)
assert.match(dispatchSource, /一龙不会自动重放/)
assert.match(dispatchSource, /reconcileUncertainSend[\s\S]*?onResponseRefresh/)

process.stdout.write('PASS local AI stable receipt and uncertain send reconciliation\n')
