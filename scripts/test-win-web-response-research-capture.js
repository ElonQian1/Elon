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
class FakeXmlHttpRequest {
  constructor() {
    this.status = 200
    this.responseType = ''
    this.responseText = responseBody
    this.listeners = new Map()
  }

  open() {}

  send() {}

  addEventListener(name, listener) {
    this.listeners.set(name, listener)
  }

  getResponseHeader() {
    return 'application/json; charset=utf-8'
  }

  complete() {
    const listener = this.listeners.get('loadend')
    if (listener) listener()
  }
}
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

vm.runInNewContext(source, sandbox, { filename: 'win_web_response_research_capture.js' })

async function main() {
  await windowObject.fetch('https://www.google.com/async/folif', { method: 'POST' })
  await new Promise((resolve) => setImmediate(resolve))
  assert.equal(captures.length, 1)
  const capture = captures[0]
  assert.equal(capture.providerId, 'google-ai-mode')
  assert.equal(capture.captureRuntimeVersion, 9)
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
  const installedFetch = windowObject.fetch
  const installedOpen = FakeXmlHttpRequest.prototype.open
  const installedSend = FakeXmlHttpRequest.prototype.send
  const firstXhr = new FakeXmlHttpRequest()
  firstXhr.open('POST', 'https://www.google.com/async/folif')
  firstXhr.send()
  firstXhr.complete()
  assert.equal(captures.length, 2)
  assert.equal(captures[1].transport, 'xhr')
  assert.equal(captures[1].captureRuntimeVersion, 9)
  const upgradedSource = source.replace('var VERSION = 9;', 'var VERSION = 10;')
  vm.runInNewContext(upgradedSource, sandbox, {
    filename: 'win_web_response_research_capture.js',
  })
  assert.equal(windowObject.fetch, installedFetch, 'runtime upgrade must not stack fetch wrappers')
  assert.equal(FakeXmlHttpRequest.prototype.open, installedOpen)
  assert.equal(FakeXmlHttpRequest.prototype.send, installedSend)
  assert.equal(windowObject.__elonWinWebResponseResearchCaptureVersion, 10)
  assert.equal(windowObject.__elonWinWebResponseResearchCaptureRuntime.version, 10)
  await windowObject.fetch('https://www.google.com/async/folif', { method: 'POST' })
  await new Promise((resolve) => setImmediate(resolve))
  assert.equal(captures.length, 3, 'each response must be captured exactly once after upgrade')
  assert.equal(captures[2].captureRuntimeVersion, 10)
  const secondXhr = new FakeXmlHttpRequest()
  secondXhr.open('POST', 'https://www.google.com/async/folif')
  secondXhr.send()
  secondXhr.complete()
  assert.equal(captures.length, 4)
  assert.equal(captures[3].transport, 'xhr')
  assert.equal(captures[3].captureRuntimeVersion, 10)
  console.log('PASS: Win Web response research capture tests (26 assertions)')
}

main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
