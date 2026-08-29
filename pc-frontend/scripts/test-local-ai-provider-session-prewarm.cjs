const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const policyFilename = path.resolve(
  __dirname,
  '../src/features/user-browser/localAiProviderSessionPrewarmPolicy.ts',
)
const output = ts.transpileModule(fs.readFileSync(policyFilename, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  fileName: policyFilename,
}).outputText
const compiled = new Module(policyFilename, module)
compiled.filename = policyFilename
compiled.paths = module.paths
compiled._compile(output, policyFilename)

const {
  LOCAL_AI_PROVIDER_SESSION_PREWARM_FAILURE_COOLDOWN_MS,
  LOCAL_AI_PROVIDER_SESSION_PREWARM_SUCCESS_COOLDOWN_MS,
  LocalAiProviderSessionPrewarmGate,
  localAiProviderSessionPrewarmEligible,
} = compiled.exports

const selectedState = {
  providerId: 'chatgpt',
  windowStatus: 'ready',
  loading: false,
  lastError: null,
}
const base = {
  enabled: true,
  ownerKey: 'owner-a',
  selectedProviderId: 'chatgpt',
  candidateProviderId: 'google-ai-mode',
  selectedState,
  documentVisible: true,
}
assert.equal(localAiProviderSessionPrewarmEligible(base), true)
for (const override of [
  { enabled: false },
  { ownerKey: '' },
  { candidateProviderId: 'chatgpt' },
  { selectedState: { ...selectedState, loading: true } },
  { selectedState: { ...selectedState, windowStatus: 'opening' } },
  { selectedState: { ...selectedState, lastError: 'failed' } },
  { documentVisible: false },
]) {
  assert.equal(localAiProviderSessionPrewarmEligible({ ...base, ...override }), false)
}

const gate = new LocalAiProviderSessionPrewarmGate(2)
assert.equal(gate.claim('owner-a:google', 1_000), true)
assert.equal(gate.claim('owner-a:google', 1_000), false)
gate.release('owner-a:google', true, 1_000)
assert.equal(gate.claim('owner-a:google', 1_000 + LOCAL_AI_PROVIDER_SESSION_PREWARM_SUCCESS_COOLDOWN_MS - 1), false)
assert.equal(gate.claim('owner-a:google', 1_000 + LOCAL_AI_PROVIDER_SESSION_PREWARM_SUCCESS_COOLDOWN_MS), true)
gate.release('owner-a:google', false, 2_000_000)
assert.equal(gate.claim('owner-a:google', 2_000_000 + LOCAL_AI_PROVIDER_SESSION_PREWARM_FAILURE_COOLDOWN_MS - 1), false)
assert.equal(gate.claim('owner-a:google', 2_000_000 + LOCAL_AI_PROVIDER_SESSION_PREWARM_FAILURE_COOLDOWN_MS), true)

const backendSource = fs.readFileSync(path.resolve(
  __dirname,
  '../src/features/user-browser/useAiWebChatBackend.ts',
), 'utf8')
const hookSource = fs.readFileSync(path.resolve(
  __dirname,
  '../src/features/user-browser/useLocalAiProviderSessionPrewarm.ts',
), 'utf8')
assert.match(backendSource, /useLocalAiProviderSessionPrewarm\(\{[\s\S]*?selectedState: controller\.sessionState/)
assert.match(hookSource, /openLocalAiWebSession\(provider\.id, ownerKey, \{ showWindow: false \}\)/)
assert.match(hookSource, /localAiWarmSessionReusable/)
assert.match(hookSource, /document\.visibilityState !== 'visible'/)

process.stdout.write('PASS local AI hidden provider session prewarm contract\n')
