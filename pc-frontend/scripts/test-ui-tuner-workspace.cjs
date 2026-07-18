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
  const renderModeOutput = compile(
    'src/features/ui-tuner/rendering/renderMode.ts',
    'rendering/renderMode.js',
  )
  const { deriveUiTunerRenderMode } = require(renderModeOutput)
  assert.deepEqual(
    deriveUiTunerRenderMode({
      workspaceMode: 'evidence',
      hasAndroidPixels: true,
      runtimeDocument: false,
    }),
    { androidVisual: true, runtimeEditable: false },
    '普通 APK 真机截图也必须进入无覆盖的 Android 真实画面模式',
  )
  assert.deepEqual(
    deriveUiTunerRenderMode({
      workspaceMode: 'evidence',
      hasAndroidPixels: true,
      runtimeDocument: true,
    }),
    { androidVisual: true, runtimeEditable: true },
    '接入调试 Runtime 的 Android 画面应同时允许 LIVE 编辑',
  )
  assert.deepEqual(
    deriveUiTunerRenderMode({
      workspaceMode: 'source',
      hasAndroidPixels: true,
      runtimeDocument: true,
    }),
    { androidVisual: false, runtimeEditable: false },
    '源码数字孪生模式不得误用 Android 真实画面交互状态',
  )

  const projectRootOutput = compile(
    'src/features/ui-tuner/source-preview/sourcePreviewProjectRoot.ts',
    'source-preview/sourcePreviewProjectRoot.js',
  )
  const { androidProjectRootCandidates } = require(projectRootOutput)
  assert.deepEqual(
    androidProjectRootCandidates('D:\\projects\\elon'),
    [
      'D:\\projects\\elon',
      'D:\\projects\\elon\\android\\app',
      'D:\\projects\\elon\\android',
      'D:\\projects\\elon\\app',
    ],
    '仓库根应自动下探常见 Android app 模块，避免切到草稿后误报没有 XML',
  )
  assert.deepEqual(
    androidProjectRootCandidates('D:\\projects\\elon-task-20260718-014941-19416-daa8ba21'),
    [
      'D:\\projects\\elon-task-20260718-014941-19416-daa8ba21',
      'D:\\projects\\elon-task-20260718-014941-19416-daa8ba21\\android\\app',
      'D:\\projects\\elon-task-20260718-014941-19416-daa8ba21\\android',
      'D:\\projects\\elon-task-20260718-014941-19416-daa8ba21\\app',
      'D:\\projects\\elon',
      'D:\\projects\\elon\\android\\app',
      'D:\\projects\\elon\\android',
      'D:\\projects\\elon\\app',
    ],
    '已收尾的 task worktree 路径应自动回退主项目，避免草稿入口永久卡死',
  )

  compile(
    'src/features/ui-tuner/source-preview/sourcePreviewTree.ts',
    'source-preview/sourcePreviewTree.js',
  )
  const pwaMappingOutput = compile(
    'src/features/ui-tuner/source-preview/pwaNodeMapping.ts',
    'source-preview/pwaNodeMapping.js',
  )
  const { matchPwaSourceNode } = require(pwaMappingOutput)
  const mappedPassword = matchPwaSourceNode({
    key: 'root', name: 'ScrollView', resourceId: '', style: { text: '', contentDescription: '' }, children: [
      { key: 'password', name: 'loginPasswordInput', resourceId: '@+id/loginPasswordInput', style: { text: '', contentDescription: '' }, children: [] },
      { key: 'account', name: 'loginAccountInput', resourceId: '@+id/loginAccountInput', style: { text: '', contentDescription: '' }, children: [] },
    ],
  }, { key: '', uiNode: '', id: 'passwordInput', ariaLabel: '', role: '', text: '', tag: 'INPUT', classNames: [] })
  assert.equal(mappedPassword?.key, 'password', 'PWA passwordInput 应安全映射 Android loginPasswordInput')

  const selectionOutput = compile(
    'src/features/ui-tuner/workspace/uiWorkspaceSelection.ts',
    'workspace/uiWorkspaceSelection.js',
  )
  const { findEvidenceSelection, findSourceSelection } = require(selectionOutput)
  const sourceButton = {
    key: 'source/pay-button',
    resourceId: '@+id/pay_button',
    tag: 'Button',
    name: '立即支付',
    source: { layoutFile: 'res/layout/checkout.xml' },
    style: { text: '立即支付' },
    children: [],
  }
  const sourceRoot = {
    key: 'source/root',
    tag: 'LinearLayout',
    name: '结算页',
    source: { layoutFile: 'res/layout/checkout.xml' },
    style: { text: '' },
    children: [sourceButton],
  }
  assert.equal(
    findSourceSelection(sourceRoot, { resourceId: 'com.elon.app:id/pay_button' }),
    sourceButton.key,
    'Android 真帧选区切到本地草稿后应按 resource-id 定位同一组件',
  )
  assert.equal(
    findEvidenceSelection([
      { id: 'title', name: '标题', text: '订单', runtime: { resourceId: 'id/title' } },
      { id: 'pay', name: '立即支付', text: '立即支付', runtime: { resourceId: 'id/pay_button' } },
    ], { resourceId: '@+id/pay_button' }),
    'pay',
    '本地草稿选区返回 Android 真帧时应保留语义选中对象',
  )

  const clientIdOutput = compile(
    'src/features/ui-tuner/device/deviceLeaseClientId.ts',
    'device/deviceLeaseClientId.js',
  )
  const { createDeviceLeaseClientId } = require(clientIdOutput)
  assert.equal(
    createDeviceLeaseClientId({ randomUUID: () => '11111111-2222-3333-4444-555555555555' }),
    'uit_11111111222233334444555555555555',
  )
  assert.match(
    createDeviceLeaseClientId({}, () => 1_700_000_000_000, () => 0.123456),
    /^uit_[a-z0-9]+_[a-z0-9]+$/,
    '公网 HTTP 不提供 crypto.randomUUID 时仍须生成稳定格式的租约客户端 ID',
  )

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
    setCanvasFocusMode,
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
  const focused = setCanvasFocusMode({
    designPaneOpen: true,
    splitRatio: 62,
    leftPanelOpen: false,
    rightPanelOpen: true,
    focusMode: false,
  }, true)
  assert.equal(focused.focusMode, true)
  assert.deepEqual(
    setCanvasFocusMode(focused, false),
    {
      designPaneOpen: true,
      splitRatio: 62,
      leftPanelOpen: false,
      rightPanelOpen: true,
      focusMode: false,
    },
    '退出专注必须恢复进入前的面板偏好，而不是重置用户选择',
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

    const tabsOutput = compile(
      'src/features/ui-tuner/inspector/UiInspectorTabs.tsx',
      'inspector/UiInspectorTabs.js',
      true,
    )
    fs.writeFileSync(path.join(temporaryDirectory, 'inspector/UiInspectorTabs.module.css'), '')
    const { UiInspectorTabs } = require(tabsOutput)
    const tabsMarkup = renderToStaticMarkup(React.createElement(UiInspectorTabs, {
      value: 'design',
      onChange() {},
    }))
    assert.match(tabsMarkup, /aria-label="属性面板模式"/)
    assert.match(tabsMarkup, /aria-selected="true"[^>]*>设计</)
    assert.match(tabsMarkup, />AI</)
    assert.match(tabsMarkup, />检查</)

    const progressOutput = compile(
      'src/features/ui-tuner/workspace/UiDesignProgressBar.tsx',
      'workspace/UiDesignProgressBar.js',
      true,
    )
    fs.writeFileSync(path.join(temporaryDirectory, 'workspace/UiDesignProgressBar.module.css'), '')
    const { AndroidUiDesignProgress, SourceUiDesignProgress } = require(progressOutput)
    const sourceProgressMarkup = renderToStaticMarkup(React.createElement(SourceUiDesignProgress, {
      hasDocument: true,
      pendingCount: 2,
      saveState: 'preview',
    }))
    assert.match(sourceProgressMarkup, /aria-label="设计落地进度"/)
    assert.match(sourceProgressMarkup, /2 个组件已调整/)
    assert.match(sourceProgressMarkup, /等待真帧校准/)
    const androidProgressMarkup = renderToStaticMarkup(React.createElement(AndroidUiDesignProgress, {
      draftStatus: 'confirmed',
      liveUi: {
        state: 'connected',
        session: { historyCount: 1 },
        commitResult: { status: 'SOURCE_SAVED' },
        buildVerifyResult: { status: 'BUILD_VERIFIED' },
      },
    }))
    assert.match(androidProgressMarkup, /Runtime 已连接/)
    assert.match(androidProgressMarkup, /已保存源码/)
    assert.match(androidProgressMarkup, /无 Patch 验证通过/)
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
  assert.match(
    pageCss,
    /\.layersPanel\s*\{[^}]*grid-column:\s*1\s*;/s,
    '隐藏面板后组件树位置不能依赖 DOM 自动排布',
  )
  assert.match(
    pageCss,
    /\.stage\s*\{[^}]*grid-column:\s*2\s*;/s,
    '隐藏面板后画布必须始终留在可伸展的中间列',
  )
  assert.match(
    pageCss,
    /\.inspector\s*\{[^}]*grid-column:\s*3\s*;/s,
    '隐藏面板后属性栏位置不能依赖 DOM 自动排布',
  )
  assert.match(
    pageCss,
    /\.focusModeExit\s*\{[^}]*position:\s*fixed\s*;/s,
    '专注模式必须保留不依赖工具栏布局的固定退出入口',
  )
  const pageSource = fs.readFileSync(
    path.join(projectRoot, 'src/features/ui-tuner/UiTunerPage.tsx'),
    'utf8',
  )
  assert.match(pageSource, /<FocusModeExitButton/)
  assert.match(pageSource, /active=\{workspaceLayout\.focusMode\}/)
  assert.match(pageSource, /onExit=\{workspaceLayout\.exitFocusMode\}/)
  assert.match(pageSource, /onRecaptureDevice=\{\(\) => \{ void captureDeviceSnapshot\(\) \}\}/)
  assert.match(pageSource, /realRenderer=\{renderMode\.androidVisual\}/)
  assert.match(pageSource, /runtimeEditable=\{renderMode\.runtimeEditable\}/)
  assert.match(pageSource, /<UiWorkspaceModeBar/)
  const workspaceModeSource = fs.readFileSync(
    path.join(projectRoot, 'src/features/ui-tuner/workspace/UiWorkspaceModeBar.tsx'),
    'utf8',
  )
  assert.match(workspaceModeSource, /Android 真帧/)
  assert.match(workspaceModeSource, /本地草稿/)
  const inspectorSource = fs.readFileSync(
    path.join(projectRoot, 'src/features/ui-tuner/UiTunerInspector.tsx'),
    'utf8',
  )
  assert.match(inspectorSource, /window\.confirm\('移除后将暂时看不到真机底图/)
  assert.match(inspectorSource, /真机画面底图已移除/)
  assert.match(inspectorSource, /重新读取真机画面/)
  assert.match(inspectorSource, /<UiInspectorTabs/)
  assert.match(inspectorSource, /<UiDesignGateway/)
  assert.match(inspectorSource, /supportsTypography/)
  const gatewaySource = fs.readFileSync(
    path.join(projectRoot, 'src/features/ui-tuner/inspector/UiDesignGateway.tsx'),
    'utf8',
  )
  assert.match(gatewaySource, /正在自动准备 · 无需选择模式/)
  assert.match(gatewaySource, /连接较慢时先进入草稿，不阻塞设计/)
  assert.match(gatewaySource, /你不需要等待 Runtime/)
  assert.doesNotMatch(gatewaySource, /连接 LIVE Runtime/)
  assert.doesNotMatch(gatewaySource, /让 AI 建立绑定/)
  const automaticSetupSource = fs.readFileSync(
    path.join(projectRoot, 'src/features/ui-tuner/inspector/useAutomaticDesignSetup.ts'),
    'utf8',
  )
  assert.match(automaticSetupSource, /prepareRuntimeRef\.current\(\)/)
  assert.match(automaticSetupSource, /useDraftRef\.current\(\)/)
  assert.match(automaticSetupSource, /RUNTIME_BACKGROUND_FALLBACK_MS = 8000/)
  assert.doesNotMatch(automaticSetupSource, /setupKey \|\| runtimeBusy\) return undefined/)
  assert.match(inspectorSource, /useAutomaticDesignSetup\(/)
  const sourceInspectorSource = fs.readFileSync(
    path.join(projectRoot, 'src/features/ui-tuner/source-preview/SourcePreviewInspector.tsx'),
    'utf8',
  )
  assert.match(sourceInspectorSource, /\['text', 'button', 'input'\]\.includes\(node\.kind\)/)
  assert.match(sourceInspectorSource, /转到 Android 真帧校准/)
  assert.match(sourceInspectorSource, /只发送当前组件，不重复读取整棵源码树/)
  const sourceModeBarSource = fs.readFileSync(
    path.join(projectRoot, 'src/features/ui-tuner/source-preview/SourcePreviewModeBar.tsx'),
    'utf8',
  )
  assert.match(sourceModeBarSource, /继续编辑草稿/)
  assert.match(sourceModeBarSource, /结构草图 · 非外观预览/)
  assert.match(sourceModeBarSource, /PWA 交互草稿/)
  const sourceSurfaceSource = fs.readFileSync(
    path.join(projectRoot, 'src/features/ui-tuner/source-preview/SourceDrivenPreviewSurface.tsx'),
    'utf8',
  )
  assert.match(sourceSurfaceSource, /source-preview-fidelity-gate/)
  assert.match(sourceSurfaceSource, /这不是你的真实页面/)
  assert.match(sourceSurfaceSource, /查看 Android 真帧/)
  assert.match(sourceSurfaceSource, /高级：查看结构草图/)
  assert.match(sourceSurfaceSource, /UNKNOWN_FIDELITY/)
  assert.match(sourceSurfaceSource, /!fidelity\.safeForDefaultPreview/)
  assert.match(sourceSurfaceSource, /PwaInteractivePreviewSurface/)
  assert.match(sourceSurfaceSource, /pwaPreview\?\.available/)
  const pwaSurfaceSource = fs.readFileSync(
    path.join(projectRoot, 'src/features/ui-tuner/source-preview/PwaInteractivePreviewSurface.tsx'),
    'utf8',
  )
  assert.match(pwaSurfaceSource, /真实 PWA 页面 · 手工草稿/)
  assert.match(pwaSurfaceSource, /开始设计\/修改页面/)
  assert.match(pwaSurfaceSource, /正常使用/)
  assert.match(pwaSurfaceSource, /pwaDeviceFrame/)
  const sourceWorkspaceSource = fs.readFileSync(
    path.join(projectRoot, 'src/features/ui-tuner/source-preview/SourcePreviewWorkspace.tsx'),
    'utf8',
  )
  assert.match(sourceWorkspaceSource, /renderer\.beginLocalDraft\(\); session\.apply\(patch\)/)
  const focusExitSource = fs.readFileSync(
    path.join(projectRoot, 'src/features/ui-tuner/workspace/FocusModeExitButton.tsx'),
    'utf8',
  )
  assert.match(focusExitSource, /className=\{styles\.focusModeExit\}/)
  assert.match(focusExitSource, /onClick=\{onExit\}/)
  const layoutSource = fs.readFileSync(
    path.join(projectRoot, 'src/features/ui-tuner/workspace/useUiTunerWorkspaceLayout.ts'),
    'utf8',
  )
  assert.match(
    layoutSource,
    /event\.key\s*!==\s*'Escape'/,
    '持久化专注模式必须支持 Esc 紧急退出',
  )
  assert.match(
    pageCss,
    /\.toolbarActions button\s*\{[^}]*flex:\s*0 0 auto\s*;[^}]*white-space:\s*nowrap\s*;/s,
    '顶部工具按钮不得因可用宽度变小而逐字竖排',
  )
  assert.match(
    pageCss,
    /\.toolbar\s*\{[^}]*height:\s*auto\s*;[^}]*min-height:\s*48px\s*;/s,
    '顶部工具栏空间不足时必须允许内容撑高，不能裁掉换行后的按钮',
  )
  assert.match(
    pageCss,
    /\.toolbarActions\s*\{[^}]*flex-wrap:\s*wrap\s*;[^}]*overflow:\s*visible\s*;/s,
    '左右侧栏同时打开时，顶部操作必须自动换行并保持全部可见',
  )
  assert.match(
    pageCss,
    /\.viewControls\s*\{[^}]*flex:\s*0 0 auto\s*;/s,
    '缩放控件组必须保持完整宽度并由工具栏统一横向滚动',
  )

  console.log('ui tuner workspace: all assertions passed')
} finally {
  fs.rmSync(temporaryDirectory, { recursive: true, force: true })
}
