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
assert.equal(chatCapabilities.length, 8)
assert.equal(chatCapabilities.every((capability) => capability.runtimeEnabled), true)
assert.equal(chatCapabilities.every((capability) => (
  capability.activation === 'preset_then_background_verify'
)), true)

const googleCapabilities = localAiPrivateTransportCapabilities(google)
assert.equal(googleCapabilities.length, 6)
assert.equal(googleCapabilities.every((capability) => capability.runtimeEnabled), true)

const incompleteGoogle = localAiPrivateTransportCapabilities(provider('google-ai-mode', [
  'snapshot', 'send_prompt',
]))
assert.equal(incompleteGoogle.filter((capability) => capability.runtimeEnabled).length, 4)

const copy = localAiPrivateTransportStatusCopy(chatgpt)
assert.match(copy.copy, /8\/8/)
assert.match(copy.copy, /无需等待官网扫描/)
assert.match(copy.detail, /私有流与完成态结算/)
assert.match(copy.detail, /官网快照单飞行刷新/)
assert.match(copy.detail, /缓存先接与官网发送确认单飞行/)

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
  return { id, adapterActions, adapterVersion: 1 }
}
