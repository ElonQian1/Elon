const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const filename = path.resolve(__dirname, '../src/features/user-browser/localAiAdapterTiming.ts')
const source = fs.readFileSync(filename, 'utf8')
const output = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.CommonJS,
    target: ts.ScriptTarget.ES2020,
    esModuleInterop: true,
  },
  fileName: filename,
}).outputText
const compiled = new Module(filename, module)
compiled.filename = filename
compiled.paths = module.paths
compiled._compile(output, filename)

const {
  LOCAL_AI_RESULT_POLL_INTERVAL_MS,
  localAiAdapterResultAttempts,
  localAiAdapterResultTimeoutMs,
} = compiled.exports

const receiptFilename = path.resolve(__dirname, '../src/features/user-browser/localAiCommandReceipt.ts')
const receiptOutput = ts.transpileModule(fs.readFileSync(receiptFilename, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  fileName: receiptFilename,
}).outputText
const receiptModule = new Module(receiptFilename, module)
receiptModule.filename = receiptFilename
receiptModule.paths = module.paths
receiptModule._compile(receiptOutput, receiptFilename)
const { findMatchingLocalAiCommandReceipt } = receiptModule.exports

const prewarmFilename = path.resolve(__dirname, '../src/features/user-browser/localAiCapabilityPrewarmPolicy.ts')
const prewarmOutput = ts.transpileModule(fs.readFileSync(prewarmFilename, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  fileName: prewarmFilename,
}).outputText
const prewarmModule = new Module(prewarmFilename, module)
prewarmModule.filename = prewarmFilename
prewarmModule.paths = module.paths
const privateSignalFilename = path.resolve(__dirname, '../src/features/user-browser/localAiPrivateStreamSignal.ts')
const privateSignalOutput = ts.transpileModule(fs.readFileSync(privateSignalFilename, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  fileName: privateSignalFilename,
}).outputText
const privateSignalModule = new Module(privateSignalFilename, module)
privateSignalModule.filename = privateSignalFilename
privateSignalModule.paths = module.paths
privateSignalModule._compile(privateSignalOutput, privateSignalFilename)
const prewarmDefaultRequire = prewarmModule.require.bind(prewarmModule)
prewarmModule.require = (id) => id === './localAiPrivateStreamSignal'
  ? privateSignalModule.exports
  : prewarmDefaultRequire(id)
prewarmModule._compile(prewarmOutput, prewarmFilename)
const {
  LOCAL_AI_CAPABILITY_PREWARM_COOLDOWN_MS,
  LocalAiCapabilityPrewarmCooldown,
  localAiCapabilityPrewarmEligible,
  localAiCapabilityPrewarmSupportsModel,
} = prewarmModule.exports

assert.equal(LOCAL_AI_RESULT_POLL_INTERVAL_MS, 200)
assert.equal(localAiAdapterResultTimeoutMs('snapshot'), 5_000)
assert.equal(localAiAdapterResultTimeoutMs('send_prompt'), 12_000)
assert.equal(localAiAdapterResultTimeoutMs('new_conversation'), 12_000)
assert.equal(localAiAdapterResultTimeoutMs('list_conversations'), 12_000)
assert.equal(localAiAdapterResultAttempts('send_prompt'), 60)
assert.ok(localAiAdapterResultTimeoutMs('send_prompt') > 7_000)
assert.ok(localAiAdapterResultTimeoutMs('list_conversations') > 10_000)

const recent = [
  { action: 'collect_model_options', requestId: 'mcp_old', ok: true },
  { action: 'send_prompt', requestId: 'mcp_send', ok: true },
]
const latest = { action: 'collect_composer_tools', requestId: 'mcp_latest', ok: true }
assert.equal(findMatchingLocalAiCommandReceipt(latest, recent, 'collect_composer_tools', 'mcp_latest'), latest)
assert.equal(findMatchingLocalAiCommandReceipt(latest, recent, 'send_prompt', 'mcp_send'), recent[1])
assert.equal(findMatchingLocalAiCommandReceipt(latest, recent, 'send_prompt', 'mcp_missing'), undefined)

const readySession = {
  providerId: 'chatgpt', windowStatus: 'minimized', rendererStatus: 'active', loading: false,
}
const readySnapshot = { composerReady: true, streaming: false }
const modelActions = ['list_model_options', 'collect_model_options']
assert.equal(localAiCapabilityPrewarmEligible({
  providerId: 'chatgpt', adapterActions: modelActions, sessionState: readySession,
  snapshot: readySnapshot, foregroundBlocked: false,
}), true)
assert.equal(localAiCapabilityPrewarmEligible({
  providerId: 'chatgpt', adapterActions: ['snapshot_ui_manifest'], sessionState: readySession,
  snapshot: readySnapshot, foregroundBlocked: false,
}), true)
assert.equal(localAiCapabilityPrewarmSupportsModel(modelActions), true)
assert.equal(localAiCapabilityPrewarmSupportsModel(['list_model_options']), false)
for (const override of [
  { foregroundBlocked: true },
  { sessionState: { ...readySession, loading: true } },
  { sessionState: { ...readySession, windowStatus: 'closed' } },
  { snapshot: { ...readySnapshot, streaming: true } },
  { snapshot: { ...readySnapshot, privateStreamObserved: true, privateStreamState: 'streaming' } },
  { adapterActions: ['list_model_options'] },
]) {
  assert.equal(localAiCapabilityPrewarmEligible({
    providerId: 'chatgpt', adapterActions: modelActions, sessionState: readySession,
    snapshot: readySnapshot, foregroundBlocked: false, ...override,
  }), false)
}
const cooldown = new LocalAiCapabilityPrewarmCooldown(LOCAL_AI_CAPABILITY_PREWARM_COOLDOWN_MS, 2)
assert.equal(cooldown.claim('owner-a', 1_000), true)
assert.equal(cooldown.claim('owner-a', 1_000 + LOCAL_AI_CAPABILITY_PREWARM_COOLDOWN_MS - 1), false)
assert.equal(cooldown.claim('owner-a', 1_000 + LOCAL_AI_CAPABILITY_PREWARM_COOLDOWN_MS), true)
assert.equal(cooldown.claim('owner-b', 62_000), true)
assert.equal(cooldown.claim('owner-c', 62_000), true)
assert.equal(cooldown.claim('owner-a', 62_001), true)

process.stdout.write('PASS local AI action-specific adapter receipt timing\n')
require('./test-local-ai-adapter-result-waiter.cjs')
