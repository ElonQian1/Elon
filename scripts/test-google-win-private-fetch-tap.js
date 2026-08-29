'use strict'

const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')

const repoRoot = path.resolve(__dirname, '..')
const tapSource = fs.readFileSync(path.join(
  repoRoot,
  'desktop-shell/src-tauri/src/local_ai_browser/google_win_private_fetch_tap.js',
), 'utf8')
const captureSource = fs.readFileSync(path.join(
  repoRoot,
  'desktop-shell/src-tauri/src/local_ai_browser/win_web_response_research_capture.js',
), 'utf8').replace('__PROVIDER_ID__', 'google-ai-mode')

const responseBody = `)]}'\n\n92\n[["wrb.fr","rpc-id","{\\"answer\\":[\\"hello\\"]}",null]]`
const response = {
  ok: true,
  status: 200,
  headers: { get: () => 'application/json; charset=utf-8' },
  body: null,
  text: async () => responseBody,
  clone() {
    return this
  },
}

async function main() {
  const captures = []
  const originalFetch = async () => response
  class FakeXmlHttpRequest {}
  const windowObject = {
    fetch: originalFetch,
    XMLHttpRequest: FakeXmlHttpRequest,
    __TAURI_INTERNALS__: {
      invoke: async (command, args) => {
        assert.equal(command, 'publish_local_ai_web_research_capture')
        captures.push(args.capture)
      },
    },
  }
  const sandbox = {
    window: windowObject,
    location: { href: 'https://www.google.com/aimode', origin: 'https://www.google.com' },
    URL,
    TextEncoder,
    TextDecoder,
    Set,
    Promise,
    XMLHttpRequest: FakeXmlHttpRequest,
    console,
  }

  vm.runInNewContext(tapSource, sandbox, { filename: 'google_win_private_fetch_tap.js' })
  const installedFetch = windowObject.fetch
  const installedTap = windowObject.__elonWinGooglePrivateFetchTap
  assert.equal(installedTap.version, 1)
  vm.runInNewContext(captureSource, sandbox, {
    filename: 'win_web_response_research_capture.google-tap.js',
  })
  assert.equal(
    windowObject.fetch,
    installedFetch,
    'research capture must subscribe to the Google tap instead of stacking a fetch wrapper',
  )

  await windowObject.fetch('https://www.google.com/async/folif', { method: 'POST' })
  await new Promise((resolve) => setImmediate(resolve))
  assert.equal(captures.length, 1)
  assert.equal(captures[0].providerId, 'google-ai-mode')
  assert.equal(captures[0].captureRuntimeVersion, 12)
  assert.equal(captures[0].endpointFamily, 'ai_rpc')

  await windowObject.fetch('https://www.google.com/async/folif', { method: 'GET' })
  await windowObject.fetch('https://example.com/async/folif', { method: 'POST' })
  await new Promise((resolve) => setImmediate(resolve))
  assert.equal(captures.length, 1, 'non-private and cross-origin requests must remain ignored')

  vm.runInNewContext(tapSource, sandbox, { filename: 'google_win_private_fetch_tap.rebind.js' })
  assert.equal(windowObject.__elonWinGooglePrivateFetchTap, installedTap)
  assert.equal(windowObject.fetch, installedFetch, 'reinjection must remain idempotent')

  installedTap.dispose()
  assert.equal(windowObject.fetch, originalFetch)

  console.log('GOOGLE_WIN_PRIVATE_FETCH_TAP_TESTS=passed')
}

main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
