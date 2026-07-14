const assert = require('node:assert/strict')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { Module } = require('node:module')
const React = require('react')
const { renderToStaticMarkup } = require('react-dom/server')
const ts = require('typescript')

const projectRoot = path.resolve(__dirname, '..')
const temporaryDirectory = fs.mkdtempSync(path.join(os.tmpdir(), 'elon-runtime-draft-'))
const outputFile = path.join(temporaryDirectory, 'runtimeDraftModel.js')
process.env.NODE_PATH = [path.join(projectRoot, 'node_modules'), process.env.NODE_PATH]
  .filter(Boolean)
  .join(path.delimiter)
Module._initPaths()

try {
  const sourceFile = path.join(projectRoot, 'src/features/ui-tuner/live/runtimeDraftModel.ts')
  const source = fs.readFileSync(sourceFile, 'utf8')
  const compiled = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.CommonJS,
      target: ts.ScriptTarget.ES2022,
      esModuleInterop: true,
    },
    fileName: sourceFile,
    reportDiagnostics: true,
  })
  const errors = compiled.diagnostics.filter((diagnostic) => (
    diagnostic.category === ts.DiagnosticCategory.Error
  ))
  assert.equal(errors.length, 0, errors.map((diagnostic) => diagnostic.messageText).join('\n'))
  fs.writeFileSync(outputFile, compiled.outputText)

  const {
    EMPTY_RUNTIME_DRAFT_STATE,
    acknowledgeRuntimeDraft,
    applyRuntimeDraftOperations,
    confirmRuntimeDraftFrame,
    projectRuntimeVisual,
    runtimeDraftStatus,
  } = require(outputFile)

  const baseFrame = frameAt(1_700_000_000_000)
  const node = createNode()
  const visual = projectRuntimeVisual(node, {
    width: { type: 'dp', value: 120 },
    height: { type: 'dp', value: 48 },
    textSize: { type: 'sp', value: 16 },
    backgroundColor: { type: 'argb', value: '#CC112233' },
    'cornerRadius.all': { type: 'dp', value: 12 },
  })

  assert.equal(visual.rect.width, 315, 'dp 宽度必须乘 density')
  assert.equal(visual.rect.height, 126, 'dp 高度必须乘 density')
  assert.equal(visual.fontSize, 52.5, 'sp 字号必须乘 density 与 fontScale')
  assert.equal(visual.borderRadius, 31.5, 'dp 圆角必须乘 density')
  assert.equal(visual.background, '#112233CC', 'Android AARRGGBB 必须转换为 CSS RRGGBBAA')

  const first = applyRuntimeDraftOperations(
    EMPTY_RUNTIME_DRAFT_STATE,
    node,
    [{ property: 'height', value: { type: 'dp', value: 52 } }],
    baseFrame,
  )
  const second = applyRuntimeDraftOperations(
    first,
    node,
    [{ property: 'height', value: { type: 'dp', value: 56 } }],
    baseFrame,
  )
  const staleAck = acknowledgeRuntimeDraft(second, node.runtimeNodeId, first.revision, appliedAck())
  assert.equal(staleAck, second, '旧 ACK 不能覆盖较新的本地草稿')
  assert.equal(runtimeDraftStatus(staleAck), 'local')

  const originalNow = Date.now
  Date.now = () => 1_700_000_000_000
  try {
    const acked = acknowledgeRuntimeDraft(second, node.runtimeNodeId, second.revision, appliedAck())
    assert.equal(runtimeDraftStatus(acked), 'calibrating')
    const earlyFrame = confirmRuntimeDraftFrame(acked, frameAt(1_700_000_000_100))
    assert.equal(runtimeDraftStatus(earlyFrame), 'calibrating', 'ACK 前后的旧帧不能清除草稿层')
    const confirmed = confirmRuntimeDraftFrame(acked, frameAt(1_700_000_000_200))
    assert.equal(runtimeDraftStatus(confirmed), 'confirmed', 'Android 新帧到达后才清除本地草稿层')
    assert.equal(Object.keys(confirmed.nodes).length, 0)
    const numericFrame = {
      ...frameAt(1_700_000_000_200),
      capturedAt: '1700000000200',
    }
    const confirmedByNumericTimestamp = confirmRuntimeDraftFrame(acked, numericFrame)
    assert.equal(
      runtimeDraftStatus(confirmedByNumericTimestamp),
      'confirmed',
      '节点端返回毫秒时间戳字符串时也必须完成真机帧校准',
    )
  } finally {
    Date.now = originalNow
  }

  const rejected = acknowledgeRuntimeDraft(first, node.runtimeNodeId, first.revision, {
    ...appliedAck(),
    status: 'REJECTED',
    error: 'unsupported property',
  })
  assert.equal(runtimeDraftStatus(rejected), 'rejected')
  assert.ok(rejected.nodes[node.runtimeNodeId], 'Android 拒绝时 PC 草稿必须保留供用户修正')

  const shortcutFile = path.join(projectRoot, 'src/features/ui-tuner/comparison/canvasZoomShortcuts.ts')
  const shortcutOutput = path.join(temporaryDirectory, 'canvasZoomShortcuts.js')
  const shortcutCompiled = ts.transpileModule(fs.readFileSync(shortcutFile, 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
    fileName: shortcutFile,
  })
  fs.writeFileSync(shortcutOutput, shortcutCompiled.outputText)
  const { canvasZoomCommand } = require(shortcutOutput)
  const shortcut = (key, overrides = {}) => canvasZoomCommand({
    key,
    ctrlKey: true,
    metaKey: false,
    altKey: false,
    ...overrides,
  })
  assert.equal(shortcut('+'), 'zoom-in')
  assert.equal(shortcut('='), 'zoom-in')
  assert.equal(shortcut('-'), 'zoom-out')
  assert.equal(shortcut('0'), 'actual-size')
  assert.equal(shortcut('+', { ctrlKey: false }), null, '普通文字输入不能误触画布缩放')
  assert.equal(shortcut('+', { ctrlKey: false, metaKey: true }), 'zoom-in', 'macOS Command 快捷键必须可用')

  const layerFile = path.join(projectRoot, 'src/features/ui-tuner/live/RuntimeDraftLayer.tsx')
  const layerOutput = path.join(temporaryDirectory, 'RuntimeDraftLayer.js')
  const layerCompiled = ts.transpileModule(fs.readFileSync(layerFile, 'utf8'), {
    compilerOptions: {
      module: ts.ModuleKind.CommonJS,
      target: ts.ScriptTarget.ES2022,
      jsx: ts.JsxEmit.ReactJSX,
      esModuleInterop: true,
    },
    fileName: layerFile,
  })
  fs.writeFileSync(layerOutput, layerCompiled.outputText)
  fs.writeFileSync(path.join(temporaryDirectory, 'RuntimeDraftLayer.module.css'), '')
  const previousCssLoader = require.extensions['.css']
  require.extensions['.css'] = (module) => {
    module.exports = {
      __esModule: true,
      default: new Proxy({}, { get: (_target, property) => String(property) }),
    }
  }
  try {
    const { RuntimeDraftLayer } = require(layerOutput)
    const styledDraft = applyRuntimeDraftOperations(
      EMPTY_RUNTIME_DRAFT_STATE,
      node,
      [
        { property: 'height', value: { type: 'dp', value: 48 } },
        { property: 'textSize', value: { type: 'sp', value: 16 } },
        { property: 'backgroundColor', value: { type: 'argb', value: '#CC112233' } },
      ],
      baseFrame,
    )
    const markup = renderToStaticMarkup(React.createElement(RuntimeDraftLayer, {
      canvasBackground: '#000000',
      frame: baseFrame,
      nodes: [node],
      state: styledDraft,
    }))
    assert.match(markup, /aria-label="PC 本地即时预览层"/)
    assert.match(markup, /data-runtime-draft-node="rn_button"/)
    assert.match(markup, /height:126px/)
    assert.match(markup, /font-size:52\.5px/)
    assert.match(markup, /background:#112233CC/i)
    assert.match(markup, />立即支付<\/span>/)

    const surfaceFile = path.join(projectRoot, 'src/features/ui-tuner/UiTunerCanvasSurface.tsx')
    const surfaceOutput = path.join(temporaryDirectory, 'UiTunerCanvasSurface.js')
    const surfaceCompiled = ts.transpileModule(fs.readFileSync(surfaceFile, 'utf8'), {
      compilerOptions: {
        module: ts.ModuleKind.CommonJS,
        target: ts.ScriptTarget.ES2022,
        jsx: ts.JsxEmit.ReactJSX,
        esModuleInterop: true,
      },
      fileName: surfaceFile,
    })
    fs.mkdirSync(path.join(temporaryDirectory, 'live'), { recursive: true })
    fs.writeFileSync(
      path.join(temporaryDirectory, 'live/RuntimeDraftLayer.js'),
      'exports.RuntimeDraftLayer = function RuntimeDraftLayer() { return null }',
    )
    fs.writeFileSync(path.join(temporaryDirectory, 'UiTunerPage.module.css'), '')
    fs.writeFileSync(surfaceOutput, surfaceCompiled.outputText)
    const { UiTunerCanvasSurface } = require(surfaceOutput)
    const surfaceMarkup = renderToStaticMarkup(React.createElement(UiTunerCanvasSurface, {
      canvas: { name: '测试画布', width: 1080, height: 2400, background: '#000000' },
      filterResult: { visible: [] },
      liveFrame: null,
      liveNodes: [],
      runtimeDraftState: EMPTY_RUNTIME_DRAFT_STATE,
      runtimeDraftStatus: 'confirmed',
      realRenderer: true,
      runtimeConnected: true,
      runtimeGestureActive: false,
      runtimeCanMove: false,
      runtimeCanResize: false,
      scrollerRef: { current: null },
      selectedId: null,
      viewScale: 0.5,
      viewportControls: {
        actualSize() {}, fitCanvasToStage() {}, fitToStage: false,
        viewScaleLabel: '50%', zoomIn() {}, zoomOut() {},
      },
      onCanvasKeyDown() {}, onClearSelection() {}, onElementPointerDown() {}, onSelectElement() {},
    }))
    const statusIndex = surfaceMarkup.indexOf('Android LIVE · PC 本地即时渲染')
    const zoomIndex = surfaceMarkup.indexOf('aria-label="画布快捷缩放"')
    const canvasIndex = surfaceMarkup.indexOf('tabindex="0"')
    assert.ok(statusIndex >= 0 && zoomIndex >= 0 && canvasIndex >= 0)
    assert.ok(statusIndex < canvasIndex && zoomIndex < canvasIndex, '状态和缩放控件必须位于设备画布外')
    assert.match(surfaceMarkup, /恢复画布到 100%[^>]*>50%<\/button>/)

    const pageCss = fs.readFileSync(
      path.join(projectRoot, 'src/features/ui-tuner/UiTunerPage.module.css'),
      'utf8',
    )
    const runtimeSurfaceCss = pageCss.match(/\.runtimeSurfaceLive,[\s\S]*?\n}/)?.[0] ?? ''
    assert.doesNotMatch(runtimeSurfaceCss, /position:\s*sticky/)
    assert.doesNotMatch(runtimeSurfaceCss, /margin:[^;]*-/)
  } finally {
    if (previousCssLoader) require.extensions['.css'] = previousCssLoader
    else delete require.extensions['.css']
  }

  console.log('runtime draft model: all assertions passed')
} finally {
  fs.rmSync(temporaryDirectory, { recursive: true, force: true })
}

function createNode() {
  return {
    runtimeNodeId: 'rn_button',
    definitionId: 'checkout.pay_button',
    screenId: 'checkout',
    kind: 'android.widget.Button',
    className: 'android.widget.Button',
    text: '立即支付',
    geometry: {
      boundsInDisplayPx: { left: 40, top: 100, right: 460, bottom: 226, width: 420, height: 126 },
      density: 2.625,
      fontScale: 1.25,
      rotation: 0,
      visible: true,
    },
    properties: {},
    capabilities: {},
  }
}

function frameAt(milliseconds) {
  return {
    dataUrl: 'data:image/png;base64,AA==',
    width: 1080,
    height: 2400,
    bytes: 1,
    capturedAt: new Date(milliseconds).toISOString(),
  }
}

function appliedAck() {
  return {
    status: 'APPLIED',
    requestId: 'request-1',
    newTreeRevision: 2,
  }
}
