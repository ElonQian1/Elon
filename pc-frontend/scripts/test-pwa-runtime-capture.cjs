const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const ts = require('typescript')

const projectRoot = path.resolve(__dirname, '..')

function loadTypescriptModule(relativePath) {
  const filename = path.join(projectRoot, relativePath)
  const output = ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
    fileName: filename,
  }).outputText
  const loaded = { exports: {} }
  Function('require', 'module', 'exports', '__filename', '__dirname', output)(
    require, loaded, loaded.exports, filename, path.dirname(filename),
  )
  return loaded.exports
}

const verification = loadTypescriptModule(
  'src/features/ui-tuner/source-preview/pwaVerificationModel.ts',
)
const context = loadTypescriptModule(
  'src/features/ui-tuner/source-preview/pwaDesignContext.ts',
)
const runtimeAuth = loadTypescriptModule(
  'src/features/ui-tuner/source-preview/pwaRuntimeAuth.ts',
)

const verified = { phase: 'BUILD_VERIFIED', message: 'verified', mismatches: [] }
const pending = verification.pwaRuntimeCapturePendingState(verified)
assert.equal(pending.phase, 'BUILD_VERIFIED')
assert.equal(pending.runtimeCapturePending, true)

const artifact = {
  path: 'C:\\project\\.elon\\ui-tuner\\pwa-runtime\\captures\\capture-test.png',
  manifestPath: 'C:\\project\\.elon\\ui-tuner\\pwa-runtime\\captures\\capture-test.json',
  sha256: 'a'.repeat(64), width: 720, height: 1280, bytes: 321,
  mediaType: 'image/png', capturedAt: '2026-07-21T00:00:00Z',
}
const completed = verification.completePwaRuntimeCapture(pending, {
  ok: true, status: 'CAPTURED', artifact, base64Embedded: false,
})
assert.equal(completed.runtimeCapturePending, false)
assert.deepEqual(completed.runtimeCapture, artifact)

const failed = verification.completePwaRuntimeCapture(pending, {
  ok: false, status: 'CAPTURE_FAILED', base64Embedded: false,
  diagnostic: { code: 'AUTHENTICATION_REQUIRED', message: 'login', retryable: false, nextStep: 'prepare profile' },
})
assert.equal(failed.phase, 'BUILD_VERIFIED')
assert.equal(failed.runtimeCaptureDiagnostic.code, 'AUTHENTICATION_REQUIRED')

const draft = {
  schemaVersion: 2, artifactVersion: 'elon.pwa.cross-platform-draft.v2',
  kind: 'elon.pwa.manual_style_draft',
  project: { id: 'fixture', workspaceIdentity: 'C:\\project', sourceRevision: 'source-r1' },
  pageSource: { kind: 'authenticated-pwa', origin: 'http://127.0.0.1:3210', entryPath: '/app', title: 'Fixture' },
  route: { path: '/app', search: '?tab=proof', hash: '#screen' },
  viewport: { width: 360, height: 640 }, scope: 'route', visualReferences: {}, elements: {},
  revision: 7, createdAt: '2026-07-21T00:00:00Z', updatedAt: '2026-07-21T00:00:00Z',
}
const contextPack = context.buildPwaDesignContextPack({
  draft, root: null, selection: null,
  plan: { targets: [], deterministicChanges: [], codexChanges: [], requiresCodex: false, codexReasons: [] },
  deterministicResult: {
    android: { applied: 0, changedFiles: [], sourceRevision: 'android-r1' },
    pwa: { applied: 0, changedFiles: [], sourceRevisions: {} },
    pendingCodex: [], conflicts: [],
  },
  runtimeCapture: artifact,
})
assert.equal(contextPack.screen.screenshotPath, artifact.path)
assert.equal(contextPack.pwaDesign.runtimeCapture.sha256, artifact.sha256)
assert.equal(contextPack.pwaDesign.contextPolicy.screenshotsEmbeddedAsBase64, false)
assert.doesNotMatch(JSON.stringify(contextPack), /data:image|base64Embedded|"base64"/i)

async function testTemporaryAuthLifecycle() {
  const profile = 'pc_ui_tuner_0123456789abcdef0123456789abcdef'
  const secret = 'frontend-secret-must-not-enter-capture'
  const calls = []
  const captured = await runtimeAuth.captureWithTemporaryPwaAuthProfile('C:\\project', secret, '夜云', {
    prepare: async (projectRoot, token, accountLabel) => {
      calls.push(['prepare', projectRoot])
      assert.equal(token, secret)
      assert.equal(accountLabel, '夜云')
      return { profile, expiresAt: '2026-07-23T00:10:00Z', remembered: true, accountLabel }
    },
    capture: async (preparedProfile) => {
      calls.push(['capture', preparedProfile])
      return { ok: true, status: 'CAPTURED', artifact, base64Embedded: false }
    },
    cleanup: async (projectRoot, preparedProfile) => {
      calls.push(['cleanup', projectRoot, preparedProfile])
    },
  })
  assert.equal(captured.ok, true)
  assert.deepEqual(calls, [
    ['prepare', 'C:\\project'],
    ['capture', profile],
    ['cleanup', 'C:\\project', profile],
  ])
  assert.doesNotMatch(JSON.stringify(calls.slice(1)), new RegExp(secret))

  let missingTokenCalls = 0
  const missing = await runtimeAuth.captureWithTemporaryPwaAuthProfile('C:\\project', null, null, {
    prepare: async () => {
      missingTokenCalls += 1
      throw new Error('no remembered credential')
    },
    capture: async () => { missingTokenCalls += 1 },
    cleanup: async () => { missingTokenCalls += 1 },
  })
  assert.equal(missing.diagnostic.code, 'AUTHENTICATION_REQUIRED')
  assert.equal(missingTokenCalls, 1)

  let expiredCleaned = false
  const expired = await runtimeAuth.captureWithTemporaryPwaAuthProfile('C:\\project', secret, '夜云', {
    prepare: async () => ({ profile, expiresAt: '2026-07-23T00:10:00Z', remembered: true }),
    capture: async () => ({
      ok: false, status: 'CAPTURE_FAILED', base64Embedded: false,
      diagnostic: { code: 'AUTHENTICATION_REQUIRED', message: 'expired', retryable: false, nextStep: 'login' },
    }),
    cleanup: async () => { expiredCleaned = true },
  })
  assert.equal(expired.diagnostic.code, 'AUTHENTICATION_REQUIRED')
  assert.equal(expiredCleaned, true)

  const cleanupFailed = await runtimeAuth.captureWithTemporaryPwaAuthProfile('C:\\project', secret, '夜云', {
    prepare: async () => ({ profile, expiresAt: '2026-07-23T00:10:00Z', remembered: true }),
    capture: async () => ({ ok: true, status: 'CAPTURED', artifact, base64Embedded: false }),
    cleanup: async () => { throw new Error('simulated cleanup failure') },
  })
  assert.equal(cleanupFailed.diagnostic.code, 'AUTH_PROFILE_CLEANUP_FAILED')
  assert.doesNotMatch(JSON.stringify(cleanupFailed), new RegExp(secret))
}

const api = fs.readFileSync(path.join(
  projectRoot, 'src/features/ui-tuner/source-preview/sourcePreviewApi.ts',
), 'utf8')
assert.match(api, /PWA_RUNTIME_URL_REQUIRED/)
assert.match(api, /target\.pathname = evidence\.route\.path/)
assert.match(api, /\/api\/source-preview\/capture-pwa-runtime/)
assert.match(api, /getAuthToken\(\)/)
assert.match(api, /\/api\/source-preview\/pwa-auth-profile\/prepare/)
assert.match(api, /authProfile: profile/)
assert.match(api, /\/api\/source-preview\/pwa-auth-profile\/cleanup/)
assert.doesNotMatch(api, /Authorization|Cookie/)

testTemporaryAuthLifecycle().then(() => {
  console.log('PWA runtime capture frontend integration tests passed')
})
