'use strict'

const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')

const repoRoot = path.resolve(__dirname, '..')
const source = fs.readFileSync(path.join(
  repoRoot,
  'desktop-shell/src-tauri/src/local_ai_browser/win_web_response_research_capture.js',
), 'utf8').replace('__PROVIDER_ID__', 'google-ai-mode')

const captures = []
const responseBody = `)]}'\n\n92\n[["wrb.fr","rpc-id","{\\"answer\\":[\\"hello\\"]}",null,null,null,"generic"]]`
const response = {
  ok: true,
  status: 200,
  headers: { get: () => 'application/json; charset=utf-8' },
  clone() {
    return {
      status: this.status,
      headers: this.headers,
      body: null,
      text: async () => responseBody,
    }
  },
}
const originalFetch = async () => response
const windowObject = {
  fetch: originalFetch,
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
  console,
}

vm.runInNewContext(source, sandbox, { filename: 'win_web_response_research_capture.js' })

async function main() {
  await windowObject.fetch('https://www.google.com/async/folif', { method: 'POST' })
  await new Promise((resolve) => setImmediate(resolve))
  assert.equal(captures.length, 1)
  const capture = captures[0]
  assert.equal(capture.providerId, 'google-ai-mode')
  assert.equal(capture.endpointFamily, 'ai_rpc')
  assert.equal(capture.analysis.policyAvailable, true)
  assert.equal(capture.analysis.decodedFrameCount, 1)
  assert.equal(capture.analysis.acceptedFrameCount, 1)
  assert.deepEqual(
    Array.from(capture.analysis.contentTypes),
    ['google_rpc', 'batched_json', 'nested_json'],
  )
  assert.equal(capture.analysis.assistantFrameCount, 0)
  assert.equal(capture.analysis.textLength, 0)
  console.log('PASS: Win Web response research capture tests (12 assertions)')
}

main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
