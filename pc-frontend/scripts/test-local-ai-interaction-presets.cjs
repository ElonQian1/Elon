const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const previous = Module._extensions['.ts']
Module._extensions['.ts'] = function compileTypeScript(loaded, filename) {
  const output = ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
    fileName: filename,
  }).outputText
  loaded._compile(output, filename)
}

const policy = require(path.resolve(
  __dirname,
  '../src/features/user-browser/localAiInteractionPresets.ts',
))
if (previous) Module._extensions['.ts'] = previous
else delete Module._extensions['.ts']

const models = policy.localAiBuiltInComposerOptions('chatgpt', 'model')
assert.deepEqual(models.map((item) => item.label), ['高级', '自动'])
assert.equal(models[0].opensSubmenu, true)
assert.equal(models.every((item) => policy.isLocalAiInteractionPreset(item.id)), true)
assert.deepEqual(
  policy.localAiBuiltInComposerOptions('chatgpt', 'tools').map((item) => item.semantic),
  ['image_generation', 'web_search'],
)
assert.deepEqual(policy.localAiBuiltInFeatures('chatgpt').map((item) => item.kind), ['images'])
assert.deepEqual(policy.localAiBuiltInFeatures('google-ai-mode'), [])

const liveAuto = {
  id: 'official-model-auto', label: 'ChatGPT 自动', selected: false,
  kind: 'model', semantic: 'model', opensSubmenu: false,
}
assert.equal(policy.resolveLocalAiComposerPreset(models[1], [liveAuto]).id, liveAuto.id)
const webSearch = policy.localAiBuiltInComposerOptions('chatgpt', 'tools')[1]
assert.equal(policy.resolveLocalAiComposerPreset(webSearch, [{
  id: 'official-search', label: '搜索网页', selected: false,
  kind: 'tool', semantic: 'web_search', opensSubmenu: false,
}]).id, 'official-search')
assert.equal(policy.resolveLocalAiComposerPreset(webSearch, []), null)

const builtInFeature = policy.localAiBuiltInFeatures('chatgpt')[0]
assert.equal(policy.resolveLocalAiFeaturePreset(builtInFeature, [{
  id: 'official-images', label: '图片', kind: 'images', selected: false,
}]).id, 'official-images')

const now = 25_000_000
assert.equal(policy.localAiStableInteractionNeedsRefresh(models, now, now), true)
assert.equal(policy.localAiStableInteractionNeedsRefresh([liveAuto], now, now), false)
assert.equal(policy.localAiStableInteractionNeedsRefresh(
  [liveAuto], now - policy.LOCAL_AI_STABLE_INTERACTION_FRESH_MS, now,
), true)

const state = {
  composerEvents: {
    model: { type: 'composer_controls_snapshot', section: 'model', currentModel: 'Auto', options: [liveAuto] },
    tools: { type: 'composer_controls_snapshot', section: 'tools', currentModel: 'Auto', options: [] },
  },
  featureEvent: { type: 'navigation_snapshot', features: [{
    id: 'official-images', label: '图像', kind: 'images', selected: false,
  }] },
}
assert.equal(policy.localAiComposerSnapshotFromState(state, 'model').options[0].id, liveAuto.id)
assert.equal(policy.localAiComposerSnapshotFromState(state, 'tools').section, 'tools')
assert.equal(policy.localAiFeatureSnapshotFromState(state).features[0].id, 'official-images')

const component = fs.readFileSync(path.resolve(
  __dirname,
  '../src/features/user-browser/AiWebComposerControls.tsx',
), 'utf8')
assert.match(component, /presetFlight\.current/)
assert.match(component, /refreshComposerControls\(section\)/)
assert.match(component, /resolveLocalAiComposerPreset\(option, live\)/)
assert.match(component, /if \(resolved\) await web\.controller\.run\(action, resolved\.id\)/)
assert.match(component, /refreshFeatureNavigation\(\)/)
assert.match(component, /resolveLocalAiFeaturePreset\(option, live\)/)

process.stdout.write('PASS APK-aligned Win interaction preset cache and one-intent reconciliation\n')
