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
    viewport: { width: 390.4, height: 844.2 },
  }
  const draft = createPwaDesignDraft(project, route)
  assert.equal(draft.schemaVersion, 2)
  assert.equal(draft.artifactVersion, 'elon.pwa.cross-platform-draft.v2')
  assert.equal(draft.route.search, '?tab=design', '运行时 preview 参数不能污染正式 route')
  assert.deepEqual(draft.viewport, { width: 390, height: 844 })
  assert.equal(draft.pageSource.kind, 'authenticated-pwa')
  assert.equal(draft.pageSource.origin, 'https://elon.example')
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

  const cliPackage = buildPwaDraftCliPackage(migrated)
  assert.equal(cliPackage.kind, 'elon_ui_tuner_pwa_cli_package')
  assert.equal(cliPackage.capabilities.PWA_CODE_GENERATION, true)
  assert.equal(JSON.stringify(cliPackage).includes('base64,'), false, 'CLI 包不得内嵌截图 Base64')

  console.log('pwa design artifact: all assertions passed')
} finally {
  delete global.window
  fs.rmSync(temporaryDirectory, { recursive: true, force: true })
}
