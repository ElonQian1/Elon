const assert = require('node:assert/strict')
const guestTransport = require('../desktop-shell/src-tauri/src/local_ai_browser/chatgpt_win_private_guest_conversation_transport.js')

async function main() {
  const first = harness(
    (url) => response(url.includes('/backend-anon/conversation/') ? 200 : 204),
    { Authorization: 'signed-in-context-must-not-cross' },
  )
  await first.root.fetch('/backend-anon/conversation', {
    method: 'POST',
    headers: { 'x-guest-context': 'guest-one' },
  })
  assert.equal(first.runtime.diagnostics().mode, 'guest')
  assert.equal(first.runtime.diagnostics().contextReady, true)
  assert.deepEqual(
    first.root.__elonChatGptPrivateResearchProbe.copyRequestContext('conversation_content'),
    { Accept: 'application/json', 'x-guest-context': 'guest-one' },
  )
  const guestResponse = await first.root.fetch('/backend-api/conversations/conversation-one', {
    method: 'GET',
    headers: { Accept: 'application/json' },
    credentials: 'include',
    __elonPrivateTransport: 'conversation_prefetch',
  })
  assert.equal(guestResponse.status, 200)
  assert.equal(first.calls.at(-1).path, '/backend-anon/conversation/conversation-one')
  assert.equal(first.calls.at(-1).credentials, 'include')
  assert.equal(first.calls.at(-1).headers['x-guest-context'], 'guest-one')
  assert.equal(first.calls.at(-1).headers.Authorization, undefined)
  assert.equal(first.runtime.diagnostics().rewrittenRequests, 1)

  const fallback = harness((url) => response(
    url.includes('/backend-anon/conversations/') ? 200 :
      url.includes('/backend-anon/conversation/') ? 404 : 204,
  ))
  await fallback.root.fetch('/backend-anon/conversation', {
    method: 'POST',
    headers: { 'x-guest-context': 'guest-two' },
  })
  const fallbackResponse = await fallback.root.fetch('/backend-api/conversations/conversation-two', {
    method: 'GET',
    __elonPrivateTransport: 'conversation_prefetch',
  })
  assert.equal(fallbackResponse.status, 200)
  assert.deepEqual(
    fallback.calls.slice(-2).map((entry) => entry.path),
    [
      '/backend-anon/conversation/conversation-two',
      '/backend-anon/conversations/conversation-two',
    ],
  )
  assert.equal(fallback.runtime.diagnostics().fallbackRequests, 1)

  const observed = harness(() => response(200))
  await observed.root.fetch('/backend-anon/f/conversations/conversation-official', {
    method: 'GET',
    headers: { 'x-guest-context': 'guest-three' },
  })
  await observed.root.fetch('/backend-api/conversations/conversation-three', {
    method: 'GET',
    __elonPrivateTransport: 'conversation_prefetch',
  })
  assert.equal(observed.calls.at(-1).path, '/backend-anon/f/conversations/conversation-three')
  assert.equal(observed.runtime.diagnostics().exactTemplateObserved, true)

  const account = harness(() => response(200))
  await account.root.fetch('/backend-anon/conversation', {
    method: 'POST', headers: { 'x-guest-context': 'guest-four' },
  })
  await account.root.fetch('/backend-api/conversation', { method: 'POST' })
  await account.root.fetch('/backend-api/conversations/conversation-four', {
    method: 'GET',
    __elonPrivateTransport: 'conversation_prefetch',
  })
  assert.equal(account.calls.at(-1).path, '/backend-api/conversations/conversation-four')
  assert.equal(account.runtime.diagnostics().mode, 'api')
  assert.equal(account.runtime.diagnostics().contextReady, false)

  process.stdout.write('ChatGPT Win private guest conversation transport tests passed\n')
}

function harness(responder, baseContext = {}) {
  const calls = []
  const root = {
    URL,
    location: {
      origin: 'https://chatgpt.com',
      href: 'https://chatgpt.com/',
    },
    __elonChatGptPrivateResearchProbe: Object.freeze({
      version: 10,
      copyRequestContext: () => Object.assign({}, baseContext),
    }),
    fetch: async (input, init = {}) => {
      const url = new URL(typeof input === 'string' ? input : input.url, 'https://chatgpt.com/')
      calls.push({
        path: url.pathname,
        method: String(init.method || 'GET').toUpperCase(),
        credentials: init.credentials,
        headers: Object.assign({}, init.headers || {}),
      })
      return responder(url.toString(), init)
    },
  }
  const runtime = guestTransport.install(root)
  assert.ok(runtime)
  assert.equal(runtime.version, guestTransport.version)
  return { calls, root, runtime }
}

function response(status) {
  return { status, ok: status >= 200 && status < 300 }
}

main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
