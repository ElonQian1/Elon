const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')

const root = path.resolve(__dirname, '..')
const source = fs.readFileSync(
  path.join(root, 'android/app/src/main/assets/google_web_private_response_tap.js'),
  'utf8',
)
const buildGradle = fs.readFileSync(path.join(root, 'android/app/build.gradle'), 'utf8')
const pageAdapter = fs.readFileSync(
  path.join(root, 'android/app/src/main/kotlin/com/elon/app/googleweb/GoogleWebPageAdapter.kt'),
  'utf8',
)

assert.match(buildGradle, /findProperty\("ELON_GOOGLE_WEB_PRIVATE_RESEARCH"\)/)
assert.match(buildGradle, /buildConfigField "boolean", "GOOGLE_WEB_PRIVATE_RESEARCH_ENABLED"/)
assert.match(pageAdapter, /BuildConfig\.GOOGLE_WEB_PRIVATE_RESEARCH_ENABLED/)
assert.match(pageAdapter, /WebViewFeature\.DOCUMENT_START_SCRIPT/)
assert.match(pageAdapter, /google_web_private_response_tap\.js/)
const adapterSource = fs.readFileSync(
  path.join(root, 'android/app/src/main/assets/google_web_adapter.js'),
  'utf8',
)
assert.match(adapterSource, /if \(fingerprint !== lastSnapshot\)/)
assert.ok(
  adapterSource.indexOf("privateResearchTap.drain") >
    adapterSource.indexOf("if (fingerprint !== lastSnapshot)"),
  'private observations must drain even when the DOM snapshot is unchanged',
)
assert.doesNotMatch(source, /document\.cookie|authorization|request\.headers|init\.headers|init\.body/)

const tick = () => new Promise((resolve) => setImmediate(resolve))
let calls = 0
const response = {
  ok: true,
  status: 200,
  headers: { get: () => 'application/json' },
  clone: () => ({ text: async () => JSON.stringify([['ELON-GOOGLE-PRIVATE-OK']]) }),
}
const originalFetch = async () => {
  calls += 1
  return response
}
const window = {
  __elonGoogleWebPrivateResearchEnabled: true,
  fetch: originalFetch,
}
window.window = window
const sandbox = {
  window,
  location: { origin: 'https://www.google.com', href: 'https://www.google.com/aimode' },
  URL,
  Promise,
  JSON,
  Object,
  String,
  Number,
  Array,
  Set,
  WeakMap,
  Element: function Element() {},
  Node: { TEXT_NODE: 3 },
  document: { body: null, querySelectorAll: () => [] },
  setTimeout: (callback) => callback(),
}
vm.runInNewContext(source, sandbox, { filename: 'google_web_private_response_tap.js' })

;(async () => {
  const tap = window.__elonGoogleWebPrivateResponseTap
  assert.equal(tap.version, 1)
  tap.observePrompt('Reply exactly with: ELON-GOOGLE-PRIVATE-OK')
  const init = { method: 'POST' }
  Object.defineProperty(init, 'headers', { get: () => { throw new Error('headers read') } })
  Object.defineProperty(init, 'body', { get: () => { throw new Error('body read') } })
  const returned = await window.fetch('https://www.google.com/_/SearchUi/data/batchexecute', init)
  await tick()
  assert.equal(returned, response)
  assert.equal(calls, 1)
  const observations = tap.drain()
  assert.equal(observations.length, 2)
  assert.match(observations[0], /^v1\|fetch\|POST\|\/_\/SearchUi\/data\/batchexecute\|200\|json\|xs\|/)
  assert.match(observations[0], /\.m1$/)
  assert.equal(observations[1], 'v1|dom|marker|m0')
  assert.equal(tap.drain().length, 0)

  response.clone = () => ({
    text: async () => JSON.stringify([
      [['thread_123456789', 'ELON-GOOGLE-PRIVATE-OK', 'metadata']],
    ]),
  })
  await window.fetch(
    'https://www.google.com/httpservice/web/AimThreadsService/ListThreads',
    { method: 'GET' },
  )
  await tick()
  const threadObservations = tap.drain()
  assert.equal(threadObservations.length, 5)
  assert.match(threadObservations[1], /^v1\|schema\|threads\|json\|m0\.0\.1\|v/)
  assert.doesNotMatch(threadObservations[1], /ELON-GOOGLE-PRIVATE-OK|controlled/)
  assert.equal(threadObservations[2], 'v1|schema|threadids|n1|u0|r0|a0|k0|e0|c0')
  assert.equal(threadObservations[3], 'v1|schema|location|p/aimode|k0')
  assert.equal(threadObservations[4], 'v1|schema|threadlink|m0')
  tap.dispose()
  assert.equal(window.fetch, originalFetch)
  console.log('GOOGLE_WEB_PRIVATE_RESPONSE_TAP_TESTS=passed')
})().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
