const assert = require('node:assert/strict')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const ts = require('typescript')

const projectRoot = path.resolve(__dirname, '..')
const temporaryDirectory = fs.mkdtempSync(path.join(os.tmpdir(), 'elon-pwa-design-'))

function compile(relativeSource, relativeOutput) {
  const sourceFile = path.join(projectRoot, relativeSource)
  const outputFile = path.join(temporaryDirectory, relativeOutput)
  fs.mkdirSync(path.dirname(outputFile), { recursive: true })
  const compiled = ts.transpileModule(fs.readFileSync(sourceFile, 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
    fileName: sourceFile,
    reportDiagnostics: true,
  })
  const errors = compiled.diagnostics.filter((diagnostic) => diagnostic.category === ts.DiagnosticCategory.Error)
  assert.equal(errors.length, 0, errors.map((diagnostic) => diagnostic.messageText).join('\n'))
  fs.writeFileSync(outputFile, compiled.outputText)
  return outputFile
}

const storage = new Map()
global.window = {
  location: { origin: 'https://elon.example' },
  setTimeout: (callback, delay) => setTimeout(callback, delay),
  clearTimeout: (timer) => clearTimeout(timer),
  localStorage: {
    getItem: (key) => storage.get(key) ?? null,
    setItem: (key, value) => storage.set(key, value),
    removeItem: (key) => storage.delete(key),
  },
}

try {
  const output = compile(
    'src/features/ui-tuner/source-preview/pwaDesignDraft.ts',
    'source-preview/pwaDesignDraft.js',
  )
  const {
    buildPwaDraftCliPackage,
    createPwaDesignDraft,
    parsePwaDesignDraft,
    pwaDraftStorageKey,
    readPwaDesignDraft,
    savePwaDesignDraft,
    stablePwaIdentityKey,
  } = require(output)

  const project = { id: 'project-1', workspaceIdentity: 'D:/project', sourceRevision: 'abc123' }
  const route = {
    path: '/web/project/1',
    search: '?ui_tuner_preview=1&tab=design',
    hash: '#details',
    href: 'https://elon.example/web/project/1?tab=design#details',
    title: '真实项目页',
    screenKey: 'page:projectPage|title:真实项目页',
    screenTitle: '真实项目页',
    viewport: { width: 390.4, height: 844.2 },
  }
  const draft = createPwaDesignDraft(project, route)
  assert.equal(draft.schemaVersion, 2)
  assert.equal(draft.artifactVersion, 'elon.pwa.cross-platform-draft.v2')
  assert.equal(draft.route.search, '?tab=design', '运行时 preview 参数不能污染正式 route')
  assert.deepEqual(draft.viewport, { width: 390, height: 844 })
  assert.equal(draft.pageSource.kind, 'authenticated-pwa')
  assert.equal(draft.pageSource.origin, 'https://elon.example')
  assert.equal(draft.route.screenKey, route.screenKey)
  assert.equal(draft.route.screenTitle, route.screenTitle)
  assert.ok(draft.createdAt && draft.updatedAt)

  savePwaDesignDraft(draft)
  assert.deepEqual(readPwaDesignDraft(project, route), draft, '版本化 Artifact 必须可从本地恢复')
  assert.ok(storage.has(pwaDraftStorageKey(project, route)))

  assert.equal(stablePwaIdentityKey({ testId: 'checkout-submit' }), 'test:checkout-submit')
  assert.equal(stablePwaIdentityKey({ id: 'payButton', selector: 'body > button:nth-of-type(2)' }), 'id:payButton')
  assert.equal(
    stablePwaIdentityKey({ selector: 'body > div:nth-of-type(2)' }),
    'selector-evidence:body > div:nth-of-type(2)',
    '易变 selector 只能作为运行时证据，不能伪装成稳定源码身份',
  )

  const migrated = parsePwaDesignDraft(JSON.stringify({
    schemaVersion: 1,
    kind: 'elon.pwa.manual_style_draft',
    project,
    route: { path: '/web', search: '', hash: '' },
    viewport: { width: 390, height: 844 },
    revision: 3,
    updatedAt: '2026-07-18T00:00:00.000Z',
    elements: {
      '#payButton': {
        identity: {
          key: '#payButton', selector: '#payButton', strategy: 'id', confidence: 'high',
          confidenceScore: .95, needsBinding: false, uiNode: '', id: 'payButton', ariaLabel: '',
          role: 'button', text: '支付', tag: 'button', classNames: [],
        },
        originalStyle: { computed: { height: '40px' }, authored: {}, inlineStyle: null },
        styleDiff: { height: '48px', borderRadius: '12px' },
        revision: 2,
        updatedAt: '2026-07-18T00:00:00.000Z',
      },
    },
  }))
  assert.equal(migrated.schemaVersion, 2)
  assert.equal(migrated.elements['id:payButton'].afterStyle.height, '48px')
  assert.equal(migrated.elements['id:payButton'].binding.needsBinding, true)
  const payElement = migrated.elements['id:payButton']

  const sameWebRoute = {
    path: '/web', search: '?ui_tuner_preview=1', hash: '',
    href: 'https://elon.example/web?ui_tuner_preview=1',
    viewport: { width: 390, height: 844 },
  }
  const homeRoute = {
    ...sameWebRoute,
    screenKey: 'page:chatPage|title:好友',
    screenTitle: '好友',
  }
  const projectARoute = {
    ...sameWebRoute,
    screenKey: 'page:projectPage|title:一龙项目',
    screenTitle: '一龙项目',
  }
  const projectBRoute = {
    ...sameWebRoute,
    screenKey: 'page:projectPage|title:演示项目',
    screenTitle: '演示项目',
  }
  assert.notEqual(
    pwaDraftStorageKey(project, homeRoute),
    pwaDraftStorageKey(project, projectARoute),
    '同 /web、同 viewport 的不同 screenKey 必须使用不同 storage key',
  )
  assert.notEqual(
    pwaDraftStorageKey(project, projectARoute),
    pwaDraftStorageKey(project, projectBRoute),
    '同 projectPage 的不同项目标题必须隔离草稿',
  )

  const legacyWithoutScreen = {
    ...draft,
    route: { path: '/web', search: '', hash: '' },
  }
  storage.set(pwaDraftStorageKey(project, homeRoute), JSON.stringify(legacyWithoutScreen))
  assert.equal(
    readPwaDesignDraft(project, homeRoute),
    null,
    '缺少 screenKey 的旧 schema v2 草稿不得静默应用到已识别画面',
  )
  storage.delete(pwaDraftStorageKey(project, homeRoute))

  const modelOutput = compile(
    'src/features/ui-tuner/source-preview/pwaDesignSessionModel.ts',
    'source-preview/pwaDesignSessionModel.js',
  )
  const { PwaDesignSessionModel } = require(modelOutput)
  const isolatedModel = new PwaDesignSessionModel()
  const homeSession = isolatedModel.restore(project, homeRoute)
  assert.equal(homeSession.restored, false)
  isolatedModel.update('id:payButton:home', (elements) => ({
    ...elements,
    'id:payButton': {
      ...payElement,
      styleDiff: { height: '51px' },
      afterStyle: { ...payElement.afterStyle, height: '51px' },
    },
  }))
  const projectASession = isolatedModel.restore(project, projectARoute)
  assert.equal(projectASession.restored, false, '首页草稿不得应用到项目页')
  assert.deepEqual(projectASession.draft.elements, {})
  isolatedModel.update('id:payButton:project-a', (elements) => ({
    ...elements,
    'id:payButton': {
      ...payElement,
      styleDiff: { height: '61px' },
      afterStyle: { ...payElement.afterStyle, height: '61px' },
    },
  }))
  assert.deepEqual(
    isolatedModel.restore(project, projectBRoute).draft.elements,
    {},
    '项目标题不同，即使都是 projectPage 也不得复用草稿',
  )
  assert.equal(
    isolatedModel.restore(project, homeRoute).draft.elements['id:payButton'].styleDiff.height,
    '51px',
    '返回原 screenKey 必须恢复首页草稿',
  )
  assert.equal(
    isolatedModel.restore(project, projectARoute).draft.elements['id:payButton'].styleDiff.height,
    '61px',
    '返回原项目 screenKey 必须恢复项目草稿',
  )
  isolatedModel.dispose()

  const modelRoute = {
    ...route,
    path: '/web/project/1/session-model',
    href: 'https://elon.example/web/project/1/session-model?tab=design#details',
  }
  const model = new PwaDesignSessionModel()
  const initialSession = model.restore(project, modelRoute)
  assert.equal(initialSession.restored, false)
  assert.equal(initialSession.draft.revision, 0)

  const edited = model.update('id:payButton:height', (elements) => ({
    ...elements,
    'id:payButton': {
      ...payElement,
      styleDiff: { height: '52px' },
      afterStyle: { ...payElement.afterStyle, height: '52px' },
    },
  }))
  assert.equal(edited.revision, 1, 'one property write should advance the draft revision exactly once')
  assert.equal(edited.elements['id:payButton'].styleDiff.height, '52px')
  assert.equal(readPwaDesignDraft(project, modelRoute).revision, 1, 'property writes should persist immediately')

  const reloadedModel = new PwaDesignSessionModel()
  const reloaded = reloadedModel.restore(project, modelRoute)
  assert.equal(reloaded.restored, true, 'reload should recover the route-scoped draft')
  assert.equal(reloaded.draft.revision, 1)
  assert.equal(reloaded.draft.elements['id:payButton'].styleDiff.height, '52px')

  const undone = model.undo()
  assert.equal(undone.revision, 2, 'undo should create a new monotonic draft revision')
  assert.deepEqual(undone.elements, {})
  assert.equal(model.canRedo, true)
  const redone = model.redo()
  assert.equal(redone.revision, 3, 'redo should create a new monotonic draft revision')
  assert.equal(redone.elements['id:payButton'].styleDiff.height, '52px')
  assert.equal(readPwaDesignDraft(project, modelRoute).revision, 3, 'redo should persist the restored property')
  model.dispose()
  reloadedModel.dispose()

  const cliPackage = buildPwaDraftCliPackage(migrated)
  assert.equal(cliPackage.kind, 'elon_ui_tuner_pwa_cli_package')
  assert.equal(cliPackage.capabilities.PWA_CODE_GENERATION, true)
  assert.equal(JSON.stringify(cliPackage).includes('base64,'), false, 'CLI 包不得内嵌截图 Base64')

  compile(
    'src/features/ui-tuner/source-preview/sourcePreviewTree.ts',
    'source-preview/sourcePreviewTree.js',
  )
  fs.writeFileSync(
    path.join(temporaryDirectory, 'source-preview/sourcePreviewApi.js'),
    'exports.commitSourcePreview = async () => ({ ok: true, sourceRevision: "next" })',
  )
  const writebackOutput = compile(
    'src/features/ui-tuner/source-preview/pwaDesignWriteback.ts',
    'source-preview/pwaDesignWriteback.js',
  )
  const { planPwaDesignWriteback } = require(writebackOutput)
  const boundDraft = {
    ...migrated,
    elements: {
      'id:payButton': {
        ...payElement,
        binding: {
          status: 'CANDIDATE', bindingConfidence: 'high', needsBinding: true,
          pwaCandidates: [{ platform: 'pwa', stableKey: 'id:payButton', confidence: .95, reason: '稳定 id' }],
          androidCandidates: [{
            platform: 'android', stableKey: 'android/pay', file: 'app/src/main/res/layout/pay.xml',
            resourceId: '@+id/payButton', confidence: 1, reason: 'resourceId 精确匹配',
          }],
        },
      },
    },
  }
  const androidRoot = {
    key: 'android/root', name: 'Root', resourceId: '', source: { layoutFile: 'app/src/main/res/layout/pay.xml', attributes: {}, startTagStart: 0, startTagEnd: 20 }, children: [{
      key: 'android/pay', name: 'payButton', resourceId: '@+id/payButton',
      source: { layoutFile: 'app/src/main/res/layout/pay.xml', attributes: { 'android:id': '@+id/payButton' }, startTagStart: 40, startTagEnd: 100 },
      children: [],
    }],
  }
  const plan = planPwaDesignWriteback(boundDraft, androidRoot)
  assert.equal(plan.strategy, 'DETERMINISTIC_THEN_CODEX')
  assert.equal(plan.targets.android, 'DETERMINISTIC')
  assert.equal(plan.targets.pwa, 'CODEX_REQUIRED')
  assert.deepEqual(plan.deterministic[0].changes, { height: '48dp', borderRadius: '12dp' })

  const contextOutput = compile(
    'src/features/ui-tuner/source-preview/pwaDesignContext.ts',
    'source-preview/pwaDesignContext.js',
  )
  const { buildPwaDesignContextPack } = require(contextOutput)
  const contextPack = buildPwaDesignContextPack({
    draft: boundDraft,
    root: androidRoot,
    selection: null,
    plan,
    deterministicResult: { applied: 1, sourceRevision: 'next', changedFiles: ['app/src/main/res/layout/pay.xml'] },
  })
  assert.equal(contextPack.kind, 'elon_ui_tuner_codex_context')
  assert.equal(contextPack.pwaDesign.capabilities.PWA_CODE_GENERATION, true)
  assert.equal(contextPack.pwaDesign.contextPolicy.fullRepositoryIncluded, false)
  assert.equal(contextPack.pwaDesign.contextPolicy.fullDomIncluded, false)
  assert.ok(contextPack.pwaDesign.compactSourceBundle.length <= 16)
  assert.equal(JSON.stringify(contextPack).includes('base64,'), false, '低 Token Context Pack 只能引用截图路径')

  const inspectorSource = fs.readFileSync(
    path.join(projectRoot, 'src/features/ui-tuner/source-preview/PwaStyleInspector.tsx'),
    'utf8',
  )
  assert.match(inspectorSource, /让 AI 同步到 APK 与 PWA/)
  assert.match(inspectorSource, /data-testid="pwa-cross-platform-sync"/)
  assert.match(inspectorSource, /\['starting', 'running'\]\.includes\(session\.syncState\.phase\)/)
  assert.match(inspectorSource, /确定性写回优先/)
  assert.match(inspectorSource, /需要 AI 建立绑定/)
  assert.match(inspectorSource, /PWA 目标/)
  assert.match(inspectorSource, /APK 目标/)

  const previewSurfaceSource = fs.readFileSync(
    path.join(projectRoot, 'src/features/ui-tuner/source-preview/PwaInteractivePreviewSurface.tsx'),
    'utf8',
  )
  const bridgeSource = fs.readFileSync(
    path.join(projectRoot, '../server/src/assets/ui_tuner_pwa_bridge.js'),
    'utf8',
  )
  assert.match(previewSurfaceSource, /当前画面：\{route\.screenTitle/)
  assert.match(bridgeSource, /document\.querySelector\('\.page\.active\[id\]'\)/)
  assert.match(bridgeSource, /document\.querySelector\('#topTitle'\)/)
  assert.match(bridgeSource, /getAttribute\('data-ui-screen'\)/)
  assert.match(bridgeSource, /if \(signature === lastRouteSignature\) return;/, '相同 screen route 必须去重')
  assert.match(bridgeSource, /window\.setTimeout\(\(\) => \{[\s\S]*?postRoute\(reason\);[\s\S]*?\}, 80\);/, '画面 Mutation 必须防抖')
  const observerAttributeFilter = bridgeSource.match(/attributeFilter:\s*\[[^\]]*\]/)?.[0] ?? ''
  assert.match(observerAttributeFilter, /'class'/)
  assert.match(observerAttributeFilter, /'aria-hidden'/)
  assert.match(observerAttributeFilter, /'data-ui-screen'/)
  assert.doesNotMatch(observerAttributeFilter, /['"]style['"]/, '样式预览不得触发 route-changed')
  const previewSurfaceCss = fs.readFileSync(
    path.join(projectRoot, 'src/features/ui-tuner/source-preview/SourcePreview.module.css'),
    'utf8',
  )
  assert.match(
    previewSurfaceSource,
    /className=\{styles\.pwaDraftBadge\}>[^<]+<\/div>\s*<div className=\{styles\.pwaDeviceViewport\}[^>]*>\s*<iframe/,
    '设计模式与正常交互模式必须共用 badge 在外、iframe 独占 viewport 的非覆盖布局',
  )
  for (const className of ['pwaWorkflowGuide', 'pwaPreviewToolbar', 'pwaRouteStatus', 'pwaDraftBadge']) {
    const rule = previewSurfaceCss.match(new RegExp(`\\.${className}\\s*\\{[^}]*\\}`))?.[0] ?? ''
    assert.ok(rule, `${className} 必须保留明确布局规则`)
    assert.doesNotMatch(rule, /position\s*:\s*(?:absolute|fixed|sticky)/, `${className} 不得覆盖 iframe 真实内容`)
  }

  console.log('pwa design artifact: all assertions passed')
} finally {
  delete global.window
  fs.rmSync(temporaryDirectory, { recursive: true, force: true })
}
