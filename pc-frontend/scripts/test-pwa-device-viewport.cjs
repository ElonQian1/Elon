const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const ts = require('typescript')

const projectRoot = path.resolve(__dirname, '..')

function loadTypescriptModule(relativePath) {
  const filename = path.join(projectRoot, relativePath)
  const output = ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
    fileName: filename,
    reportDiagnostics: true,
  })
  const errors = output.diagnostics.filter((diagnostic) => diagnostic.category === ts.DiagnosticCategory.Error)
  assert.equal(errors.length, 0, errors.map((diagnostic) => diagnostic.messageText).join('\n'))
  const loaded = { exports: {} }
  Function('require', 'module', 'exports', output.outputText)(require, loaded, loaded.exports)
  return loaded.exports
}

const viewportModel = loadTypescriptModule(
  'src/features/ui-tuner/source-preview/pwaDeviceViewport.ts',
)

assert.deepEqual(
  {
    width: viewportModel.DEFAULT_PWA_DEVICE_VIEWPORT.width,
    height: viewportModel.DEFAULT_PWA_DEVICE_VIEWPORT.height,
  },
  { width: 412, height: 915 },
  '现代真机逻辑视口必须取代固定 320×640 默认值',
)
assert.ok(
  viewportModel.PWA_DEVICE_PRESETS.some((preset) => preset.width === 360 && preset.height === 800),
  '必须覆盖常见 Android 小屏',
)
assert.ok(
  viewportModel.PWA_DEVICE_PRESETS.some((preset) => preset.width === 390 && preset.height === 844),
  '必须覆盖现代 iPhone CSS viewport',
)
assert.ok(
  viewportModel.PWA_DEVICE_PRESETS.some((preset) => preset.width === 320 && preset.height === 640),
  '320×640 只保留为小屏兼容预设',
)

const custom = viewportModel.updatePwaDeviceViewportSize(
  viewportModel.DEFAULT_PWA_DEVICE_VIEWPORT,
  400,
  824,
)
assert.equal(custom.presetId, viewportModel.PWA_CUSTOM_PRESET_ID)
assert.deepEqual({ width: custom.width, height: custom.height }, { width: 400, height: 824 })

const portrait = viewportModel.pwaDeviceViewportFromPreset('iphone-14', custom)
const landscape = viewportModel.rotatePwaDeviceViewport(portrait)
assert.deepEqual({ width: landscape.width, height: landscape.height }, { width: 844, height: 390 })
assert.deepEqual(
  landscape.safeArea,
  { top: 0, right: 47, bottom: 0, left: 34 },
  '旋转必须同步旋转安全区参考线',
)

const storage = new Map()
const storageAdapter = {
  getItem: (key) => storage.get(key) ?? null,
  setItem: (key, value) => storage.set(key, value),
}
viewportModel.savePwaDeviceViewport({ ...custom, showSafeArea: true }, storageAdapter)
assert.deepEqual(
  viewportModel.readPwaDeviceViewport(storageAdapter),
  { ...custom, showSafeArea: true },
  '最近使用的自定义 viewport 必须持久化',
)
assert.doesNotThrow(() => viewportModel.savePwaDeviceViewport(custom, {
  setItem() {
    throw new Error('storage disabled')
  },
}))
storage.set(viewportModel.PWA_DEVICE_VIEWPORT_STORAGE_KEY, '{broken')
assert.deepEqual(
  viewportModel.readPwaDeviceViewport(storageAdapter),
  viewportModel.DEFAULT_PWA_DEVICE_VIEWPORT,
  '损坏的本地状态必须安全回退',
)

const unknownPreset = viewportModel.normalizePwaDeviceViewport({
  presetId: 'future-device',
  width: 430,
  height: 932,
})
assert.equal(unknownPreset.presetId, viewportModel.PWA_CUSTOM_PRESET_ID)
assert.equal(unknownPreset.width, 430)
assert.equal(unknownPreset.height, 932)

const bounded = viewportModel.normalizePwaDeviceViewport({
  presetId: viewportModel.PWA_CUSTOM_PRESET_ID,
  width: 99999,
  height: 1,
  deviceScaleFactor: 99,
})
assert.deepEqual(
  { width: bounded.width, height: bounded.height, dpr: bounded.deviceScaleFactor },
  { width: 1440, height: 240, dpr: 4 },
  '自定义输入必须受无头浏览器与画布边界约束',
)

console.log('PWA device viewport tests passed')
