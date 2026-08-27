const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const catalogFilename = path.resolve(
  __dirname,
  '../src/features/user-browser/localAiPrivateTransportCatalog.ts',
)
const catalogSource = fs.readFileSync(catalogFilename, 'utf8')
const output = ts.transpileModule(catalogSource, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  fileName: catalogFilename,
}).outputText
const compiled = new Module(catalogFilename, module)
compiled.filename = catalogFilename
compiled.paths = module.paths
compiled._compile(output, catalogFilename)

const {
  localAiPrivateTransportCapabilities,
  localAiPrivateTransportStatusCopy,
} = compiled.exports

const chatgpt = provider('chatgpt', [
  'snapshot', 'send_prompt', 'stop_generation', 'list_conversations',
  'open_conversation', 'invoke_ui_control',
])
const google = provider('google-ai-mode', [
  'snapshot', 'send_prompt', 'list_conversations', 'open_conversation',
])

const chatCapabilities = localAiPrivateTransportCapabilities(chatgpt)
assert.equal(chatCapabilities.length, 10)
assert.equal(chatCapabilities.every((capability) => capability.runtimeEnabled), true)
assert.equal(chatCapabilities.every((capability) => (
  capability.activation === 'preset_then_background_verify'
)), true)

const googleCapabilities = localAiPrivateTransportCapabilities(google)
assert.equal(googleCapabilities.length, 8)
assert.equal(googleCapabilities.every((capability) => capability.runtimeEnabled), true)

const incompleteGoogle = localAiPrivateTransportCapabilities(provider('google-ai-mode', [
  'snapshot', 'send_prompt',
]))
assert.equal(incompleteGoogle.filter((capability) => capability.runtimeEnabled).length, 5)

const copy = localAiPrivateTransportStatusCopy(chatgpt)
assert.match(copy.copy, /10\/10/)
assert.match(copy.copy, /无需等待官网扫描/)
assert.match(copy.detail, /私有流与完成态结算/)
assert.match(copy.detail, /官网快照单飞行刷新/)
assert.match(copy.detail, /稳定回执与不确定发送对账/)
assert.match(copy.detail, /独立会话正文与富内容缓存/)
assert.match(copy.detail, /私有流原生事件即时刷新/)

const live = localAiPrivateTransportStatusCopy(chatgpt, health({
  prefetchReady: true,
  privateLatencyMs: 428,
  successes: 7,
}), 10_000)
assert.match(live.copy, /实时可用/)
assert.match(live.copy, /428ms/)
assert.match(live.copy, /成功 7 次/)

const cooling = localAiPrivateTransportStatusCopy(chatgpt, health({
  cooldownRemainingMs: 31_100,
  lastOutcome: 'timeout',
}), 10_000)
assert.match(cooling.copy, /约 32 秒/)
assert.match(cooling.copy, /上次请求超时/)
assert.match(cooling.copy, /立即回退官网/)

const awaitingContext = localAiPrivateTransportStatusCopy(chatgpt, health(), 10_000)
assert.match(awaitingContext.copy, /等待官网会话上下文/)
assert.match(awaitingContext.copy, /不阻塞输入/)

const stale = localAiPrivateTransportStatusCopy(chatgpt, health({ sampledAtMs: 1 }), 200_000)
assert.match(stale.copy, /10\/10/)

const capabilityHook = fs.readFileSync(path.resolve(
  __dirname,
  '../src/features/user-browser/useLocalAiBrowserCapability.ts',
), 'utf8')
const providerPresets = fs.readFileSync(path.resolve(
  __dirname,
  '../src/features/user-browser/localAiWebProviders.ts',
), 'utf8')
assert.match(capabilityHook, /desktopDetected \? 'ready' : 'desktop_required'/)
assert.match(capabilityHook, /desktopDetected \? localAiWebProviderPresets\(\) : \[\]/)
assert.match(capabilityHook, /void verify\(true\)/)
assert.match(capabilityHook, /if \(!preservePreset\)/)
assert.match(providerPresets, /'google-ai-mode'[\s\S]*?'list_conversations'[\s\S]*?'open_conversation'/)

process.stdout.write('PASS Win private transport preset-first catalog\n')

function provider(id, adapterActions) {
  return { id, adapterActions, adapterVersion: 1, desktopRuntimeVersion: 2 }
}

function health(overrides = {}) {
  return {
    version: 1,
    prefetchEnabled: true,
    prefetchReady: false,
    officialFresh: false,
    cooldownRemainingMs: 0,
    officialLatencyMs: 0,
    privateLatencyMs: 0,
    successes: 0,
    failures: 0,
    consecutiveFailures: 0,
    lastOutcome: 'none',
    attemptBudgetMs: 700,
    sampledAtMs: 10_000,
    ...overrides,
  }
}
