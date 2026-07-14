const assert = require('node:assert/strict')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { Module } = require('node:module')
const React = require('react')
const { renderToStaticMarkup } = require('react-dom/server')
const ts = require('typescript')

const projectRoot = path.resolve(__dirname, '..')
const temporaryDirectory = fs.mkdtempSync(path.join(os.tmpdir(), 'elon-ui-tuner-workspace-'))
process.env.NODE_PATH = [path.join(projectRoot, 'node_modules'), process.env.NODE_PATH]
  .filter(Boolean)
  .join(path.delimiter)
Module._initPaths()

function compile(relativeSource, relativeOutput, jsx = false) {
  const sourceFile = path.join(projectRoot, relativeSource)
  const outputFile = path.join(temporaryDirectory, relativeOutput)
  fs.mkdirSync(path.dirname(outputFile), { recursive: true })
  const compilerOptions = {
    module: ts.ModuleKind.CommonJS,
    target: ts.ScriptTarget.ES2022,
    esModuleInterop: true,
  }
  if (jsx) compilerOptions.jsx = ts.JsxEmit.ReactJSX
  const compiled = ts.transpileModule(fs.readFileSync(sourceFile, 'utf8'), {
    compilerOptions,
    fileName: sourceFile,
    reportDiagnostics: true,
  })
  const errors = compiled.diagnostics.filter((diagnostic) => (
    diagnostic.category === ts.DiagnosticCategory.Error
  ))
  assert.equal(errors.length, 0, errors.map((diagnostic) => diagnostic.messageText).join('\n'))
  fs.writeFileSync(outputFile, compiled.outputText)
  return outputFile
}

try {
  const scaleOutput = compile(
    'src/features/ui-tuner/comparison/canvasViewportScale.ts',
    'comparison/canvasViewportScale.js',
  )
  const { calculateCanvasFitScale, normalizeCanvasScale } = require(scaleOutput)

  assert.equal(normalizeCanvasScale(Number.NaN), 1)
  assert.equal(normalizeCanvasScale(0.01), 0.08)
  assert.equal(normalizeCanvasScale(4), 2)
  assert.equal(
    calculateCanvasFitScale(
      { width: 1162, height: 654 },
      { width: 1080, height: 2400 },
      'stage',
    ),
    0.27,
    '1080×2400 真机在 654px 高的中心画布中应适高到约 27%，不能退回 10%',
  )
  assert.equal(
    calculateCanvasFitScale(
      { width: 1162, height: 654 },
      { width: 1080, height: 2400 },
      'width',
    ),
    1,
    '可视区域比真机画面更宽时适宽最多显示 100%',
  )
  assert.equal(
    calculateCanvasFitScale(
      { width: 500, height: 700 },
      { width: 1080, height: 2400 },
      'stage',
    ),
    0.29,
  )
  assert.equal(
    calculateCanvasFitScale(
      { width: 500, height: 700 },
      { width: 1080, height: 2400 },
      'width',
    ),
    0.46,
  )
  assert.equal(
    calculateCanvasFitScale(
      { width: 0, height: 700 },
      { width: 1080, height: 2400 },
      'stage',
    ),
    null,
  )

  const layoutOutput = compile(
    'src/features/ui-tuner/workspace/useUiTunerWorkspaceLayout.ts',
    'workspace/useUiTunerWorkspaceLayout.js',
  )
  const {
    clampSplitRatio,
    deriveCanvasLayout,
    parseCanvasLayoutState,
  } = require(layoutOutput)

  assert.deepEqual(parseCanvasLayoutState(null), {
    designPaneOpen: true,
    splitRatio: 35,
    leftPanelOpen: true,
    rightPanelOpen: true,
    focusMode: false,
  })
  assert.equal(clampSplitRatio(5), 20)
  assert.equal(clampSplitRatio(95), 80)
  assert.equal(clampSplitRatio(47.4), 47)
  const persisted = parseCanvasLayoutState(JSON.stringify({
    designPaneOpen: false,
    splitRatio: 62,
    leftPanelOpen: false,
    rightPanelOpen: true,
    focusMode: false,
  }))
  assert.deepEqual(persisted, {
    designPaneOpen: false,
    splitRatio: 62,
    leftPanelOpen: false,
    rightPanelOpen: true,
    focusMode: false,
  })
  assert.equal(
    deriveCanvasLayout({ ...persisted, designPaneOpen: true }, false).designPaneOpen,
    false,
    '没有设计稿时不得保留空白设计区域',
  )
  assert.deepEqual(
    deriveCanvasLayout({ ...persisted, focusMode: true }, true),
    {
      designPaneOpen: false,
      splitRatio: 62,
      leftPanelOpen: false,
      rightPanelOpen: false,
      focusMode: true,
    },
    '专注画布必须收起左右侧栏',
  )

  const previousCssLoader = require.extensions['.css']
  require.extensions['.css'] = (module) => {
    module.exports = {
      __esModule: true,
      default: new Proxy({}, { get: (_target, property) => String(property) }),
    }
  }
  try {
    const handleOutput = compile(
      'src/features/ui-tuner/comparison/ComparisonSplitHandle.tsx',
      'comparison/ComparisonSplitHandle.js',
      true,
    )
    fs.writeFileSync(
      path.join(temporaryDirectory, 'comparison/UiTunerComparisonWorkspace.module.css'),
      '',
    )
    const { ComparisonSplitHandle } = require(handleOutput)
    const markup = renderToStaticMarkup(React.createElement(ComparisonSplitHandle, {
      ratio: 35,
      onChange() {},
    }))
    assert.match(markup, /role="separator"/)
    assert.match(markup, /aria-label="调整设计稿与真机画布比例"/)
    assert.match(markup, /aria-orientation="vertical"/)
    assert.match(markup, /aria-valuemin="20"/)
    assert.match(markup, /aria-valuemax="80"/)
    assert.match(markup, /aria-valuenow="35"/)
    assert.match(markup, /tabindex="0"/)
  } finally {
    if (previousCssLoader) require.extensions['.css'] = previousCssLoader
    else delete require.extensions['.css']
  }

  const comparisonCss = fs.readFileSync(
    path.join(projectRoot, 'src/features/ui-tuner/comparison/UiTunerComparisonWorkspace.module.css'),
    'utf8',
  )
  assert.match(
    comparisonCss,
    /\.singlePane\s*\{[^}]*flex:\s*1\s*;/s,
    '无设计稿时真机单画布必须占满剩余高度',
  )
  assert.match(comparisonCss, /var\(--design-pane-ratio,\s*35%\)/)
  assert.match(comparisonCss, /\.designToolsContent\s*\{[^}]*overflow:\s*auto\s*;/s)

  const pageCss = fs.readFileSync(
    path.join(projectRoot, 'src/features/ui-tuner/UiTunerPage.module.css'),
    'utf8',
  )
  assert.match(
    pageCss,
    /\.canvasScroller\s*\{[^}]*flex:\s*1\s*;[^}]*overflow:\s*auto\s*;/s,
    '画布应自行占满并滚动，不能由页面底部截断',
  )
  assert.match(pageCss, /--layers-panel-width/)
  assert.match(pageCss, /\.focusCanvas/)

  console.log('ui tuner workspace: all assertions passed')
} finally {
  fs.rmSync(temporaryDirectory, { recursive: true, force: true })
}
