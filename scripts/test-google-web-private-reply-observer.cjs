const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')

const root = path.resolve(__dirname, '..')
const source = fs.readFileSync(
  path.join(root, 'android/app/src/main/assets/google_web_private_reply_observer.js'),
  'utf8',
)
const adapter = fs.readFileSync(
  path.join(root, 'android/app/src/main/assets/google_web_adapter.js'),
  'utf8',
)
const pageAdapter = fs.readFileSync(
  path.join(root, 'android/app/src/main/kotlin/com/elon/app/googleweb/GoogleWebPageAdapter.kt'),
  'utf8',
)

assert.doesNotMatch(source, /document\.cookie|authorization|request\.headers|init\.headers|init\.body/)
assert.match(source, /url\.pathname === '\/async\/folif'/)
assert.match(source, /diagnostics: \(\) =>/)
assert.match(source, /baseline = new WeakSet\(candidateNodes\(\)\)/)
assert.match(adapter, /privateReplyObserver\.observePrompt\(prompt\)/)
assert.match(adapter, /id: 'google-private-answer-' \+ userIndex/)
assert.match(adapter, /privateReplyObserver\.setListener\(scheduleSnapshot\)/)
assert.match(pageAdapter, /"google_web_private_reply_observer\.js"/)

class FakeElement {
  constructor(text) {
    this.children = []
    this.childNodes = [{ nodeType: 3, nodeValue: text }]
    this.parentElement = null
  }
  getBoundingClientRect() { return { width: 120, height: 24 } }
  getClientRects() { return [1] }
  hasAttribute() { return false }
  getAttribute() { return null }
  closest(selector) { return selector.includes('main, [role="main"]') ? this : null }
  querySelector() { return null }
}

const initial = new FakeElement('existing answer')
const answer = new FakeElement('ELON-GOOGLE-PRIVATE-REPLY')
const nodes = [initial]
let calls = 0
const response = { ok: true }
const originalFetch = async () => { calls += 1; return response }
const window = {
  fetch: originalFetch,
  __elonGoogleWebAnswerCandidatePolicy: { accepts: () => true },
  getComputedStyle: () => ({ display: 'block', visibility: 'visible', opacity: '1' }),
  setTimeout: (callback) => callback(),
}
window.window = window
const document = { querySelectorAll: () => nodes }
const sandbox = {
  window,
  document,
  location: {
    origin: 'https://www.google.com',
    href: 'https://www.google.com/search?udm=50',
  },
  Element: FakeElement,
  Node: { TEXT_NODE: 3 },
  URL,
  Promise,
  Object,
  String,
  Number,
  Array,
  Set,
  WeakSet,
}
vm.runInNewContext(source, sandbox, { filename: 'google_web_private_reply_observer.js' })

;(async () => {
  const observer = window.__elonGoogleWebPrivateReplyObserver
  observer.observePrompt('controlled prompt')
  nodes.push(answer)
  let notified = 0
  observer.setListener(() => { notified += 1 })
  const returned = await window.fetch('https://www.google.com/async/folif')
  assert.equal(returned, response)
  assert.equal(calls, 1)
  assert.ok(notified >= 1)
  assert.deepEqual(
    JSON.parse(JSON.stringify(observer.snapshot())),
    {
      prompt: 'controlled prompt',
      text: 'ELON-GOOGLE-PRIVATE-REPLY',
      streaming: false,
    },
  )
  assert.match(observer.diagnostics(), /^v5\|p1\|b1\|r1\|probe7\|c2\|n1\|reply1\|done1$/)
  console.log('GOOGLE_WEB_PRIVATE_REPLY_OBSERVER_TESTS=passed')
})().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
