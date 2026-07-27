const assert = require('node:assert/strict')
const crypto = require('node:crypto')
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

;(async () => {
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

  const aiReceiptOutput = compile(
    'src/features/ui-tuner/source-preview/aiWritebackReceipt.ts',
    'source-preview/aiWritebackReceipt.js',
  )
  const { parseAiWritebackReceipt } = require(aiReceiptOutput)
  const aiReceipt = parseAiWritebackReceipt(`完成。\nELON_UI_WRITEBACK_RECEIPT_V1
{
  "schemaVersion": 1,
  "changedFiles": ["web/button.css", "app/src/main/res/layout/pay.xml"],
  "sourceHash": "sha256:after",
  "sourceRevisionBefore": "workspace-sha256:before",
  "sourceRevision": "workspace-sha256:after",
  "targetPlatforms": ["pwa", "apk"],
  "platformResults": {
    "pwa": {"status":"SAVED","changedFiles":["web/button.css"],"sourceRevision":"pwa-r2"},
    "apk": {"status":"SAVED","changedFiles":["app/src/main/res/layout/pay.xml"],"sourceRevision":"apk-r2"}
  }
}
后续由工作台验证。`)
  assert.equal(aiReceipt.sourceHash, 'sha256:after')
  assert.deepEqual(aiReceipt.targetPlatforms, ['pwa', 'apk'])
  assert.equal(parseAiWritebackReceipt('任务完成但没有机器回执'), null)
  assert.equal(parseAiWritebackReceipt(`ELON_UI_WRITEBACK_RECEIPT_V1 {
    "schemaVersion":1,
    "changedFiles":["../escape.css"],
    "sourceHash":"sha256:x",
    "sourceRevisionBefore":"r1",
    "sourceRevision":"r2",
    "targetPlatforms":["pwa"],
    "platformResults":{"pwa":{"status":"SAVED","changedFiles":["../escape.css"],"sourceRevision":"r2"}}
  }`), null, 'AI 回执不得包含越界路径')

  const restoreAckOutput = compile(
    'src/features/ui-tuner/source-preview/pwaDraftRestoreAck.ts',
    'source-preview/pwaDraftRestoreAck.js',
  )
  const {
    beginPwaDraftRestore,
    consumePwaDraftAppliedAck,
    pwaDraftRestoreLabel,
  } = require(restoreAckOutput)
  let restoreState = beginPwaDraftRestore('project-1|screen-a', 7, 1)
  assert.match(pwaDraftRestoreLabel(restoreState), /正在恢复本页草稿/)
  const pendingAck = {
    requestedCount: 1,
    appliedCount: 0,
    unresolved: [{ index: 0, selector: '#late', identityKey: 'id:late', reason: 'target-missing' }],
    complete: false,
    draftKey: 'project-1|screen-a',
    revision: 7,
    attempt: 1,
    maxAttempts: 8,
    retrying: true,
    exhausted: false,
  }
  restoreState = consumePwaDraftAppliedAck(restoreState, pendingAck)
  assert.equal(restoreState.phase, 'pending')
  assert.match(pwaDraftRestoreLabel(restoreState), /草稿恢复待处理/)
  assert.equal(
    consumePwaDraftAppliedAck(restoreState, pendingAck),
    restoreState,
    '重复 pending ack 必须幂等，不得重复改变宿主状态',
  )
  assert.equal(
    consumePwaDraftAppliedAck(restoreState, { ...pendingAck, draftKey: 'stale-screen', complete: true }),
    restoreState,
    '过期 draftKey 的 ack 必须被忽略',
  )
  restoreState = consumePwaDraftAppliedAck(restoreState, {
    ...pendingAck,
    appliedCount: 1,
    unresolved: [],
    complete: true,
    attempt: 2,
    retrying: false,
  })
  assert.equal(restoreState.phase, 'complete')
  assert.equal(pwaDraftRestoreLabel(restoreState), '已恢复本页草稿 · r7')
  assert.equal(
    consumePwaDraftAppliedAck(restoreState, { ...pendingAck, appliedCount: 1, unresolved: [], complete: true, attempt: 2 }),
    restoreState,
    '重复 complete ack 必须保持已完成状态且无重复副作用',
  )
  const restoreMismatchState = consumePwaDraftAppliedAck(beginPwaDraftRestore('project-1|screen-a', 8, 1), {
    ...pendingAck,
    draftKey: 'project-1|screen-a',
    revision: 8,
    unresolved: [{ index: 0, selector: '#wrong', identityKey: 'id:right', reason: 'identity-mismatch' }],
    exhausted: true,
  })
  assert.equal(restoreMismatchState.phase, 'failed')
  assert.match(pwaDraftRestoreLabel(restoreMismatchState), /身份不匹配，已拒绝修改/)
  const invalidSuccessState = consumePwaDraftAppliedAck(beginPwaDraftRestore('project-1|screen-a', 9, 1), {
    ...pendingAck,
    draftKey: 'project-1|screen-a',
    revision: 9,
    requestedCount: 1,
    appliedCount: 0,
    unresolved: [],
    complete: true,
  })
  assert.equal(invalidSuccessState.phase, 'failed', '计数不闭合的 success ack 绝不能显示恢复成功')
  assert.match(pwaDraftRestoreLabel(invalidSuccessState), /回执计数不一致/)

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
  assert.equal(cliPackage.compactHandoff.purpose, 'low-token-ui-style-writeback')
  assert.equal(cliPackage.compactHandoff.tokenPolicy.fullRepositoryIncluded, false)
  assert.equal(cliPackage.compactHandoff.tokenPolicy.fullDomIncluded, false)
  assert.equal(cliPackage.compactHandoff.tokenPolicy.screenshotsEmbeddedAsBase64, false)
  assert.equal(cliPackage.compactHandoff.elements.length, 1)
  assert.deepEqual(
    cliPackage.compactHandoff.elements[0].changedProperties.map((change) => `${change.property}:${change.after}`),
    ['height:48px', 'borderRadius:12px'],
    'CLI 紧凑包必须直接列出待写回样式，不要求 Codex 重新理解整棵 DOM',
  )
  assert.match(cliPackage.instructions.join('\n'), /优先读取 compactHandoff/)
  assert.equal(JSON.stringify(cliPackage).includes('base64,'), false, 'CLI 包不得内嵌截图 Base64')

  compile(
    'src/features/ui-tuner/source-preview/sourcePreviewTree.ts',
    'source-preview/sourcePreviewTree.js',
  )
  fs.writeFileSync(
    path.join(temporaryDirectory, 'source-preview/sourcePreviewApi.js'),
    'exports.commitSourcePreview = async () => ({ ok: true, sourceRevision: "next" }); exports.commitPwaStylePreview = async () => ({ ok: true, sourceRevision: "0".repeat(64), changedFiles: [] })',
  )
  const writebackOutput = compile(
    'src/features/ui-tuner/source-preview/pwaDesignWriteback.ts',
    'source-preview/pwaDesignWriteback.js',
  )
  const {
    applyDeterministicAndroidWriteback,
    applyDeterministicPwaWriteback,
    planPwaDesignWriteback,
    recordDeterministicWriteback,
  } = require(writebackOutput)
  const fixtureRoot = path.join(temporaryDirectory, 'fixture-project')
  const pwaSourceFile = 'src/styles/pay.css'
  const pwaSourcePath = path.join(fixtureRoot, pwaSourceFile)
  fs.mkdirSync(path.dirname(pwaSourcePath), { recursive: true })
  const initialPwaSource = '.pay { height: 40px; border-radius: 6px; }\n.title { font-size: 18px; }\n'
  fs.writeFileSync(pwaSourcePath, initialPwaSource)
  const pwaRevision = crypto.createHash('sha256').update(initialPwaSource).digest('hex')
  const explicitBinding = {
    version: 1,
    sourceFile: pwaSourceFile,
    sourceRevision: pwaRevision,
    kind: 'css-rule',
    target: '.pay',
    range: { start: 0, end: 47 },
    propertyMap: { height: 'height', borderRadius: 'border-radius' },
  }
  const boundDraft = {
    ...migrated,
    elements: {
      'id:payButton': {
        ...payElement,
        binding: {
          status: 'BOUND', bindingConfidence: 'high', needsBinding: false,
          pwaStyle: explicitBinding,
          pwaCandidates: [{ platform: 'pwa', stableKey: 'id:payButton', file: pwaSourceFile, confidence: 1, reason: '显式绑定' }],
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
  assert.equal(plan.targets.pwa, 'DETERMINISTIC')
  assert.equal(plan.requiresCodex, false)
  assert.deepEqual(plan.deterministic.android[0].changes, { height: '48dp', borderRadius: '12dp' })
  assert.deepEqual(plan.deterministic.pwa[0].changes, { height: '48px', 'border-radius': '12px' })

  const androidCalls = []
  const androidResult = await applyDeterministicAndroidWriteback({
    draft: boundDraft,
    root: androidRoot,
    projectRoot: fixtureRoot,
    sourceRevision: 'android-r0',
    commit: async (request) => {
      androidCalls.push(request)
      return { ok: true, sourceRevision: `android-r${androidCalls.length}` }
    },
  })
  assert.equal(androidResult.applied, 1)
  assert.equal(androidCalls[0].sourceRevision, 'android-r0')
  assert.deepEqual(androidResult.changedFiles, ['app/src/main/res/layout/pay.xml'])

  const multiAndroidDraft = {
    ...boundDraft,
    elements: {
      ...boundDraft.elements,
      'id:title': {
        ...payElement,
        identity: { ...payElement.identity, key: 'id:title', id: 'title' },
        styleDiff: { height: '44px' },
        binding: {
          ...boundDraft.elements['id:payButton'].binding,
          pwaStyle: { ...explicitBinding, target: '.title', range: { start: 49, end: 74 }, propertyMap: { height: 'height' } },
          androidCandidates: [{
            platform: 'android', stableKey: 'android/title', file: 'app/src/main/res/layout/pay.xml',
            resourceId: '@+id/title', confidence: 1, reason: 'resourceId 精确匹配',
          }],
        },
      },
    },
  }
  androidRoot.children.push({
    key: 'android/title', name: 'title', resourceId: '@+id/title',
    source: { layoutFile: 'app/src/main/res/layout/pay.xml', attributes: { 'android:id': '@+id/title' }, startTagStart: 140, startTagEnd: 190 },
    children: [],
  })
  const orderedAndroidCalls = []
  const orderedAndroidResult = await applyDeterministicAndroidWriteback({
    draft: multiAndroidDraft, root: androidRoot, projectRoot: fixtureRoot, sourceRevision: 'r0',
    commit: async (request) => {
      orderedAndroidCalls.push(request)
      return { ok: true, sourceRevision: `r${orderedAndroidCalls.length}` }
    },
  })
  assert.deepEqual(orderedAndroidCalls.map((call) => call.nodeKey), ['android/title', 'android/pay'], 'Android 应按源码位置倒序调用')
  assert.deepEqual(orderedAndroidCalls.map((call) => call.sourceRevision), ['r0', 'r1'], 'Android revision 必须逐步传播')
  assert.equal(orderedAndroidResult.sourceRevision, 'r2')
  assert.deepEqual(orderedAndroidResult.changedFiles, ['app/src/main/res/layout/pay.xml'])

  const failedAndroidCalls = []
  const failedAndroidResult = await applyDeterministicAndroidWriteback({
    draft: multiAndroidDraft, root: androidRoot, projectRoot: fixtureRoot, sourceRevision: 'r0',
    commit: async (request) => {
      failedAndroidCalls.push(request)
      if (failedAndroidCalls.length === 2) throw new Error('revision conflict')
      return { ok: true, sourceRevision: 'r1' }
    },
  })
  assert.equal(failedAndroidResult.stopped, true)
  assert.equal(failedAndroidResult.applied, 1)
  assert.match(failedAndroidResult.error, /revision conflict/)
  assert.equal(failedAndroidCalls.length, 2, 'Android 失败后不得继续调用')

  const pwaCalls = []
  const fixtureCommit = async (request) => {
    pwaCalls.push(request)
    const target = path.resolve(request.projectRoot, request.binding.sourceFile)
    assert.ok(target.startsWith(`${path.resolve(request.projectRoot)}${path.sep}`), 'mock 也只允许 fixture 根内文件')
    const before = fs.readFileSync(target, 'utf8')
    const currentRevision = crypto.createHash('sha256').update(before).digest('hex')
    if (currentRevision !== request.sourceRevision) throw new Error('sourceRevision conflict')
    let after = before
    for (const [property, value] of Object.entries(request.changes)) {
      const expression = new RegExp(`(${property.replace('-', '\\-')}\\s*:\\s*)[^;]+`)
      assert.match(after, expression, `fixture should contain ${property}`)
      after = after.replace(expression, `$1${value}`)
    }
    fs.writeFileSync(target, after)
    const sourceRevision = crypto.createHash('sha256').update(after).digest('hex')
    return { ok: true, sourceRevision, changedFiles: [request.binding.sourceFile] }
  }
  const pwaResult = await applyDeterministicPwaWriteback({
    draft: boundDraft, root: androidRoot, projectRoot: fixtureRoot, commit: fixtureCommit,
  })
  assert.equal(pwaResult.applied, 1)
  assert.deepEqual(pwaResult.changedFiles, [pwaSourceFile])
  assert.match(fs.readFileSync(pwaSourcePath, 'utf8'), /height: 48px; border-radius: 12px/)

  fs.writeFileSync(pwaSourcePath, initialPwaSource)
  const multiPwaDraft = JSON.parse(JSON.stringify(boundDraft))
  multiPwaDraft.elements['id:title'] = {
    ...JSON.parse(JSON.stringify(payElement)),
    identity: { ...payElement.identity, key: 'id:title', id: 'title' },
    styleDiff: { fontSize: '20px' },
    binding: {
      ...JSON.parse(JSON.stringify(boundDraft.elements['id:payButton'].binding)),
      pwaStyle: {
        ...explicitBinding,
        target: '.title',
        range: { start: 49, end: 74 },
        propertyMap: { fontSize: 'font-size' },
      },
      androidCandidates: [{
        platform: 'android', stableKey: 'android/title', file: 'app/src/main/res/layout/pay.xml',
        resourceId: '@+id/title', confidence: 1, reason: 'resourceId 精确匹配',
      }],
    },
  }
  const pwaCallStart = pwaCalls.length
  const orderedPwaResult = await applyDeterministicPwaWriteback({
    draft: multiPwaDraft, root: androidRoot, projectRoot: fixtureRoot, commit: fixtureCommit,
  })
  const orderedPwaCalls = pwaCalls.slice(pwaCallStart)
  assert.deepEqual(orderedPwaCalls.map((call) => call.binding.target), ['.title', '.pay'], 'PWA 同文件写回应按 range 倒序')
  assert.equal(orderedPwaCalls[0].sourceRevision, pwaRevision)
  assert.equal(orderedPwaCalls[1].sourceRevision, orderedPwaResult.completed[0].sourceRevision, 'PWA revision 必须逐步传播')
  assert.equal(orderedPwaResult.applied, 2)

  fs.writeFileSync(pwaSourcePath, initialPwaSource)
  const staleDraft = JSON.parse(JSON.stringify(boundDraft))
  staleDraft.elements['id:payButton'].binding.pwaStyle.sourceRevision = 'f'.repeat(64)
  const beforeConflict = fs.readFileSync(pwaSourcePath, 'utf8')
  const conflictResult = await applyDeterministicPwaWriteback({
    draft: staleDraft, root: androidRoot, projectRoot: fixtureRoot, commit: fixtureCommit,
  })
  assert.equal(conflictResult.stopped, true)
  assert.match(conflictResult.error, /sourceRevision conflict/)
  assert.equal(fs.readFileSync(pwaSourcePath, 'utf8'), beforeConflict, 'revision 冲突不得覆盖文件')

  const traversalDraft = JSON.parse(JSON.stringify(boundDraft))
  traversalDraft.elements['id:payButton'].binding.pwaStyle.sourceFile = '../outside.css'
  const traversalPlan = planPwaDesignWriteback(traversalDraft, androidRoot)
  assert.equal(traversalPlan.deterministic.pwa.length, 0, '路径越界绑定不得进入确定性计划')
  assert.equal(traversalPlan.targets.pwa, 'CODEX_REQUIRED')

  const partialDraft = JSON.parse(JSON.stringify(boundDraft))
  partialDraft.elements['id:payButton'].binding.pwaStyle.propertyMap = { height: 'height' }
  const deterministicCompletion = {
    android: androidResult,
    pwa: {
      applied: 1,
      changedFiles: [pwaSourceFile],
      sourceRevisions: { [pwaSourceFile]: 'a'.repeat(64) },
      completed: [{
        elementKey: 'id:payButton', sourceFile: pwaSourceFile, sourceRevision: 'a'.repeat(64),
        properties: { height: '48px' },
      }],
    },
  }
  const completedDraft = recordDeterministicWriteback(partialDraft, deterministicCompletion)
  assert.equal(completedDraft.elements['id:payButton'].binding.pwaStyle.sourceRevision, 'a'.repeat(64), '后续编辑必须使用 PWA 最新 revision')
  const fallbackPlan = planPwaDesignWriteback(completedDraft, androidRoot)
  assert.equal(fallbackPlan.deterministic.android.length, 0, '已完成 Android 属性不得再次规划')
  assert.equal(fallbackPlan.deterministic.pwa.length, 0, '已完成 PWA 属性不得再次规划')
  assert.deepEqual(
    fallbackPlan.codexChanges.map((change) => `${change.platform}.${change.property}`),
    ['pwa.borderRadius'],
    'Codex 只接收未映射的小范围属性',
  )

  const verificationOutput = compile(
    'src/features/ui-tuner/source-preview/pwaVerificationModel.ts',
    'source-preview/pwaVerificationModel.js',
  )
  const {
    completePwaVerification,
    livePwaVerificationState,
    pwaBuildVerifyingState,
    pwaSourceSavedState,
    sourceSavedEvidenceFromAiReceipt,
    sourceSavedEvidenceFromDraft,
  } = require(verificationOutput)
  const evidence = sourceSavedEvidenceFromDraft(completedDraft, 'verify-r7')
  assert.ok(evidence, '确定性回执应生成小范围 changed-files 验证证据')
  assert.deepEqual(evidence.changedFiles, [pwaSourceFile])
  assert.deepEqual(evidence.expectedValues, ['48px'])
  const aiEvidence = sourceSavedEvidenceFromAiReceipt(completedDraft, {
    schemaVersion: 1, changedFiles: [pwaSourceFile], sourceHash: `sha256:${'c'.repeat(64)}`,
    sourceRevisionBefore: 'workspace-before', sourceRevision: 'workspace-after', targetPlatforms: ['pwa'],
    platformResults: { pwa: { status: 'SAVED', changedFiles: [pwaSourceFile], sourceRevision: 'b'.repeat(64) } },
  }, 'verify-ai-r7')
  assert.ok(aiEvidence, 'AI 机器回执应能和草稿合成真实 PWA 验证证据')
  assert.deepEqual(aiEvidence.changedFiles, [pwaSourceFile])
  assert.deepEqual(aiEvidence.sourceRevisions, { [pwaSourceFile]: 'b'.repeat(64) })
  assert.deepEqual(
    aiEvidence.checks.flatMap((check) => Object.entries(check.styles).map(([property, value]) => `${property}:${value}`)).sort(),
    ['borderRadius:12px', 'height:48px'],
    'AI 回执验证必须覆盖草稿中的目标样式，而不是只相信 Codex 文案',
  )
  const liveState = livePwaVerificationState()
  assert.equal(liveState.phase, 'LIVE_PREVIEW')
  const savedState = pwaSourceSavedState(liveState, evidence)
  assert.equal(savedState.phase, 'SOURCE_SAVED', '源码写回成功只能进入 SOURCE_SAVED')
  assert.notEqual(savedState.phase, 'BUILD_VERIFIED', '写回回执不得冒充真实验证')
  const verifyingState = pwaBuildVerifyingState(savedState)
  assert.equal(verifyingState.phase, 'BUILD_VERIFYING')
  const buildVerified = {
    ok: true,
    status: 'BUILD_VERIFIED',
    message: '构建资源通过',
    sourceRevisions: { [pwaSourceFile]: 'a'.repeat(64) },
    changedFiles: [pwaSourceFile],
    buildDurationMs: 20,
    resourceFiles: ['dist/app.css'],
    resourceValuesVerified: 1,
  }
  const matchingSnapshot = {
    requestId: evidence.requestId,
    route: completedDraft.route,
    changedFiles: [pwaSourceFile],
    sourceRevisions: { [pwaSourceFile]: 'a'.repeat(64) },
    nodes: [{
      elementKey: 'id:payButton', selector: '#payButton', found: true,
      computed: { height: '48px' }, authored: { height: '48px' },
    }],
  }
  const verifiedState = completePwaVerification(verifyingState, buildVerified, matchingSnapshot)
  assert.equal(verifiedState.phase, 'BUILD_VERIFIED', '构建、资源、真实源码画面逐项匹配后才可验证通过')

  const mismatchState = completePwaVerification(verifyingState, buildVerified, {
    ...matchingSnapshot,
    nodes: [{ ...matchingSnapshot.nodes[0], computed: { height: '40px' }, authored: { height: '40px' } }],
  })
  assert.equal(mismatchState.phase, 'VERIFY_FAILED')
  assert.equal(mismatchState.evidence, evidence, '画面不匹配必须保留草稿写回证据以便恢复')
  assert.match(mismatchState.mismatches.join('\n'), /期望 48px/)

  const buildFailed = completePwaVerification(verifyingState, {
    ...buildVerified,
    ok: false,
    status: 'VERIFY_FAILED',
    message: '前端构建失败',
    resourceValuesVerified: 0,
  }, matchingSnapshot)
  assert.equal(buildFailed.phase, 'VERIFY_FAILED')
  assert.equal(buildFailed.evidence, evidence, '构建失败也必须保留草稿证据')

  const contextOutput = compile(
    'src/features/ui-tuner/source-preview/pwaDesignContext.ts',
    'source-preview/pwaDesignContext.js',
  )
  const { buildPwaDesignContextPack } = require(contextOutput)
  const contextPack = buildPwaDesignContextPack({
    draft: completedDraft,
    root: androidRoot,
    selection: null,
    plan: fallbackPlan,
    deterministicResult: deterministicCompletion,
  })
  assert.equal(contextPack.kind, 'elon_ui_tuner_codex_context')
  assert.equal(contextPack.pwaDesign.capabilities.PWA_CODE_GENERATION, true)
  assert.equal(contextPack.pwaDesign.contextPolicy.fullRepositoryIncluded, false)
  assert.equal(contextPack.pwaDesign.contextPolicy.fullDomIncluded, false)
  assert.ok(contextPack.pwaDesign.compactSourceBundle.length <= 16)
  assert.equal(contextPack.pwaDesign.compactHandoff.purpose, 'low-token-ui-style-writeback')
  assert.equal(contextPack.pwaDesign.compactHandoff.tokenPolicy.preferThisSummaryBeforeArtifact, true)
  assert.deepEqual(
    contextPack.pwaDesign.compactHandoff.elements
      .flatMap((element) => element.changedProperties.map((change) => `${change.property}:${change.after}`)),
    ['height:48px', 'borderRadius:12px'],
    'AI 自动接力也必须拿到低 token 样式摘要',
  )
  assert.deepEqual(contextPack.requestedAdjustments.map((change) => change.property), ['pwa.borderRadius'])
  assert.equal(contextPack.pwaDesign.changes.length, 1)
  assert.equal(contextPack.pwaDesign.bindingSummary.length, 1)
  assert.equal(contextPack.pwaDesign.deterministicSummary.android.applied, 1)
  assert.equal(JSON.stringify(contextPack).includes('pwa.height'), false, '确定性完成的 PWA 属性不得再次请求 Codex')
  assert.equal(JSON.stringify(contextPack).includes('android.height'), false, '确定性完成的 Android 属性不得再次请求 Codex')
  assert.equal(JSON.stringify(contextPack).includes('base64,'), false, '低 Token Context Pack 只能引用截图路径')

  const inspectorSource = fs.readFileSync(
    path.join(projectRoot, 'src/features/ui-tuner/source-preview/PwaStyleInspector.tsx'),
    'utf8',
  )
  assert.match(inspectorSource, /写回源码并验证 APK 与 PWA/)
  assert.match(inspectorSource, /data-testid="pwa-cross-platform-sync"/)
  assert.match(inspectorSource, /session\.syncState\.phase === 'BUILD_VERIFYING'/)
  assert.match(inspectorSource, /保留草稿并重试真实验证/)
  assert.match(inspectorSource, /确定性优先，AI 只补缺口/)
  assert.match(inspectorSource, /需要 AI 建立绑定\/结构修改/)
  assert.match(inspectorSource, /PWA：/)
  assert.match(inspectorSource, /APK：/)
  assert.match(inspectorSource, /CrossPlatformWritebackReceiptPanel/)
  const receiptPanelSource = fs.readFileSync(
    path.join(projectRoot, 'src/features/ui-tuner/source-preview/CrossPlatformWritebackReceiptPanel.tsx'),
    'utf8',
  )
  for (const label of ['sourceRevision', 'sourceHash', 'changedFiles', 'targetPlatforms', 'build-verified']) {
    assert.match(receiptPanelSource, new RegExp(label), `机器回执界面必须显示 ${label}`)
  }
  assert.match(receiptPanelSource, /data-platform-status=/)

  const previewSurfaceSource = fs.readFileSync(
    path.join(projectRoot, 'src/features/ui-tuner/source-preview/PwaInteractivePreviewSurface.tsx'),
    'utf8',
  )
  const bridgeSource = fs.readFileSync(
    path.join(projectRoot, '../server/src/assets/ui_tuner_pwa_bridge.js'),
    'utf8',
  )
  const sessionSource = fs.readFileSync(
    path.join(projectRoot, 'src/features/ui-tuner/source-preview/usePwaDesignSession.ts'),
    'utf8',
  )
  const projectSessionSource = fs.readFileSync(
    path.join(projectRoot, 'src/features/ui-tuner/UiTunerProjectSessionPanel.tsx'),
    'utf8',
  )
  const orchestratorSource = fs.readFileSync(
    path.join(projectRoot, 'src/features/ui-tuner/source-preview/crossPlatformWritebackOrchestrator.ts'),
    'utf8',
  )
  const androidWriteIndex = orchestratorSource.indexOf('applyDeterministicAndroidWriteback')
  const pwaWriteIndex = orchestratorSource.indexOf('applyDeterministicPwaWriteback')
  assert.ok(androidWriteIndex >= 0 && pwaWriteIndex > androidWriteIndex)
  assert.doesNotMatch(
    orchestratorSource.slice(androidWriteIndex, pwaWriteIndex),
    /if\s*\([^)]*android\.error[^)]*\)\s*(?:\{[^}]*\})?\s*return/,
    'APK 单端失败不得提前阻断 PWA 写回',
  )
  assert.match(previewSurfaceSource, /当前画面：\{route\.screenTitle/)
  assert.match(bridgeSource, /document\.querySelectorAll\('\.page\.active\[id\]'\)/)
  assert.match(bridgeSource, /document\.querySelector\('#topTitle'\)/)
  assert.match(bridgeSource, /getAttribute\('data-ui-screen'\)/)
  assert.match(bridgeSource, /getAttribute\('data-ui-style-binding'\)/)
  assert.match(bridgeSource, /if \(signature === lastRouteSignature\) return;/, '相同 screen route 必须去重')
  assert.match(sessionSource, /verification\.markSourceSaved\(evidence/, '确定性源码写回后必须先标记 SOURCE_SAVED')
  assert.match(sessionSource, /await verification\.start\(evidence\)/, '确定性绑定才自动运行真实验证')
  assert.match(sessionSource, /verification\.markAiWriting\(taskId,[\s\S]*AI 正在补未绑定属性或结构修改/, 'AI fallback 不得伪造可验证的确定性回执')
  assert.match(sessionSource, /sourceSavedEvidenceFromAiReceipt/, 'AI 回执必须转换成真实 PWA 验证证据')
  assert.match(sessionSource, /verification\.start\(aiEvidence\)/, 'AI PWA 写回后必须自动进入真实构建与画面验证')
  assert.match(sessionSource, /AI 已写回 PWA 源码；正在用真实源码重载验证/, 'AI 成功回执不得让界面停留在 AI_WRITING')
  assert.match(projectSessionSource, /pwaDesign\.compactHandoff/, 'PWA_DRAFT 自动任务必须优先让 Codex 读取紧凑交接包')
  assert.match(projectSessionSource, /elements\[\]\.changedProperties/, 'PWA_DRAFT 自动任务必须按低 token 摘要里的属性清单写回')
  assert.match(projectSessionSource, /sourceFilesToInspect/, 'PWA_DRAFT 自动任务必须优先打开候选源码文件')
  assert.match(projectSessionSource, /不要默认读取整仓库或整棵 DOM/, 'PWA_DRAFT 自动任务必须明确禁止默认全量读取')
  assert.match(sessionSource, /message\.type === 'draft-applied'/, '宿主必须消费 iframe 的真实草稿应用回执')
  assert.match(sessionSource, /identity: \{ \.\.\.element\.identity, key: stablePwaIdentityKey/, '草稿桥接条目必须携带稳定身份指纹')
  assert.doesNotMatch(sessionSource, /setSaveLabel\(didRestore[^\n]*已恢复本页草稿/, '读取本地草稿后不得在真实 ack 前显示已恢复')
  assert.doesNotMatch(sessionSource, /phase:\s*'completed'/, '旧 completed 状态不得继续冒充验证')
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
})().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
