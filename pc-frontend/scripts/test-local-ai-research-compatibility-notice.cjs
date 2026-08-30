const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const filename = path.resolve(
  __dirname,
  '../src/features/user-browser/localAiResearchCompatibilityNotice.ts',
)
const output = ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  fileName: filename,
}).outputText
const compiled = new Module(filename, module)
compiled.filename = filename
compiled.paths = module.paths
compiled._compile(output, filename)
const { localAiResearchCompatibilityNotice } = compiled.exports

assert.equal(localAiResearchCompatibilityNotice(undefined), null)
assert.equal(localAiResearchCompatibilityNotice(status({ captureCount: 0 })), null)
assert.equal(localAiResearchCompatibilityNotice(status({ compatibility: 'rich_compatible' })), null)

const unsupported = localAiResearchCompatibilityNotice(status({
  compatibility: 'renderer_upgrade_required', unsupportedRichCount: 3,
}))
assert.match(unsupported.title, /Win 渲染适配待更新/)
assert.match(unsupported.detail, /3 类/)
assert.match(unsupported.detail, /正文和已识别卡片继续显示/)

const google = localAiResearchCompatibilityNotice(status({ compatibility: 'structure_observed' }))
assert.match(google.title, /Google 富内容结构已识别/)
assert.match(google.detail, /不会把占位误报为完整内容/)

for (const compatibility of ['upstream_changed', 'parse_error', 'incomplete']) {
  const drift = localAiResearchCompatibilityNotice(status({
    compatibility, acceptedFrameCount: 2, decodedFrameCount: 9,
  }))
  assert.match(drift.title, /解析适配待更新/)
  assert.match(drift.detail, /2\/9 帧/)
}

const hook = fs.readFileSync(path.resolve(
  __dirname,
  '../src/features/user-browser/useLocalAiResearchCompatibility.ts',
), 'utf8')
assert.match(hook, /RESEARCH_STATUS_SETTLEMENT_DELAYS_MS = \[0, 1_400, 3_600\]/)
assert.match(hook, /if \(!enabled \|\| !providerId \|\| !ownerKey \|\| streaming\) return/)
assert.match(hook, /getLocalAiWebResearchCaptureStatus\(providerId, ownerKey\)/)
assert.match(hook, /semanticUpdatedAtMs/)
assert.match(hook, /active = false/)

const notice = fs.readFileSync(path.resolve(
  __dirname,
  '../src/features/ai/AiWebClientUpgradeNotice.tsx',
), 'utf8')
assert.match(notice, /localAiResearchCompatibilityNotice\(web\.researchStatus\)/)
assert.match(notice, /查看官网完整内容/)
assert.match(notice, /下载新版/)

process.stdout.write('PASS upstream rich-response drift becomes a native Win upgrade notice\n')

function status(overrides = {}) {
  return {
    captureCount: 1,
    analyzedCaptureCount: 1,
    latestAnalyzedAtMs: 100,
    compatibility: 'text_compatible',
    decodedFrameCount: 4,
    acceptedFrameCount: 4,
    assistantFrameCount: 1,
    textLength: 20,
    richKinds: [],
    contentTypes: [],
    unsupportedRichCount: 0,
    completed: true,
    truncated: false,
    privateNetworkObservationCount: 0,
    privateVoiceObservationCount: 0,
    privateObservationLatestAtMs: 0,
    privateVoiceChannels: [],
    ...overrides,
  }
}
