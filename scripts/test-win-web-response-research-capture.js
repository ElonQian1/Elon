'use strict'

const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')
const zlib = require('node:zlib')

const repoRoot = path.resolve(__dirname, '..')
const source = fs.readFileSync(path.join(
  repoRoot,
  'desktop-shell/src-tauri/src/local_ai_browser/win_web_response_research_capture.js',
), 'utf8').replace('__PROVIDER_ID__', 'google-ai-mode')
const chatGptSource = fs.readFileSync(path.join(
  repoRoot,
  'desktop-shell/src-tauri/src/local_ai_browser/win_web_response_research_capture.js',
), 'utf8').replace('__PROVIDER_ID__', 'chatgpt')
const capturedFinanceRecovery = require(path.join(
  repoRoot,
  'desktop-shell/src-tauri/src/local_ai_browser/chatgpt_win_captured_finance_recovery.js',
))
const chatGptPolicy = require(path.join(
  repoRoot,
  'android/app/src/main/assets/chatgpt_web_private_stream_policy.js',
))

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
  assert.equal(capture.captureRuntimeVersion, 11)
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
  assert.equal(captures[1].captureRuntimeVersion, 11)
  const upgradedSource = source.replace('var VERSION = 11;', 'var VERSION = 12;')
  vm.runInNewContext(upgradedSource, sandbox, {
    filename: 'win_web_response_research_capture.js',
  })
  assert.equal(windowObject.fetch, installedFetch, 'runtime upgrade must not stack fetch wrappers')
  assert.equal(FakeXmlHttpRequest.prototype.open, installedOpen)
  assert.equal(FakeXmlHttpRequest.prototype.send, installedSend)
  assert.equal(windowObject.__elonWinWebResponseResearchCaptureVersion, 12)
  assert.equal(windowObject.__elonWinWebResponseResearchCaptureRuntime.version, 12)
  await windowObject.fetch('https://www.google.com/async/folif', { method: 'POST' })
  await new Promise((resolve) => setImmediate(resolve))
  assert.equal(captures.length, 3, 'each response must be captured exactly once after upgrade')
  assert.equal(captures[2].captureRuntimeVersion, 12)
  const secondXhr = new FakeXmlHttpRequest()
  secondXhr.open('POST', 'https://www.google.com/async/folif')
  secondXhr.send()
  secondXhr.complete()
  assert.equal(captures.length, 4)
  assert.equal(captures[3].transport, 'xhr')
  assert.equal(captures[3].captureRuntimeVersion, 12)

  const tapCaptures = []
  const tapListeners = new Set()
  let tapSubscriptions = 0
  const tapFetch = async () => response
  const tapWindow = {
    fetch: tapFetch,
    XMLHttpRequest: class {},
    __elonChatGptPrivateFetchTap: {
      subscribe(listener) {
        tapSubscriptions += 1
        tapListeners.add(listener)
        return () => tapListeners.delete(listener)
      },
    },
    __TAURI_INTERNALS__: {
      invoke: async (command, args) => {
        assert.equal(command, 'publish_local_ai_web_research_capture')
        tapCaptures.push(args.capture)
      },
    },
  }
  const tapSandbox = {
    window: tapWindow,
    XMLHttpRequest: tapWindow.XMLHttpRequest,
    location: { href: 'https://www.google.com/aimode', origin: 'https://www.google.com' },
    URL,
    TextEncoder,
    TextDecoder,
    Set,
    Promise,
    console,
  }
  vm.runInNewContext(source, tapSandbox, {
    filename: 'win_web_response_research_capture.tap.js',
  })
  assert.equal(tapWindow.fetch, tapFetch, 'the canonical fetch tap avoids another fetch wrapper')
  assert.equal(tapSubscriptions, 1)
  tapListeners.forEach((listener) => listener({
    method: 'POST',
    url: 'https://www.google.com/async/folif',
    response,
  }))
  await new Promise((resolve) => setImmediate(resolve))
  assert.equal(tapCaptures.length, 1)
  assert.equal(tapCaptures[0].captureRuntimeVersion, 11)
  const replacementFetch = async () => response
  tapWindow.fetch = replacementFetch
  vm.runInNewContext(upgradedSource, tapSandbox, {
    filename: 'win_web_response_research_capture.tap-upgrade.js',
  })
  assert.equal(tapWindow.fetch, replacementFetch, 'capture survives a later page fetch replacement')
  assert.equal(tapSubscriptions, 1, 'runtime upgrades reuse one tap subscription')
  tapListeners.forEach((listener) => listener({
    method: 'POST',
    url: 'https://www.google.com/async/folif',
    response,
  }))
  await new Promise((resolve) => setImmediate(resolve))
  assert.equal(tapCaptures.length, 2)
  assert.equal(tapCaptures[1].captureRuntimeVersion, 12)

  const financeWidget = {
    asset_display_name: 'Bitcoin (BTC)',
    current_price_text: 'US$78,805.00',
    default_range: '1D',
    timeframe_order: ['1D'],
    timeframe_configs: {
      '1D': {
        summary: { price_text: 'US$78,805.00', price_change_text: '+1.81%' },
        chart: { data: [
          { formatted: '10:00', close: 77_000 },
          { formatted: '11:00', close: 78_805 },
        ] },
      },
    },
  }
  const compressed = zlib.gzipSync(Buffer.from(JSON.stringify(financeWidget)))
    .toString('base64url')
  const richFrame = {
    conversation_id: 'conversation-captured-finance',
    message: {
      id: 'assistant-captured-finance',
      author: { role: 'assistant' },
      status: 'finished_successfully',
      content: { content_type: 'text', parts: ['完整正文。'] },
      metadata: {
        turn_exchange_id: 'turn-captured-finance',
        view_state: {
          widgets: {
            'widget-captured-finance': {
              __encoding: 'gzip-json-base64url-v1',
              __compressed: compressed,
            },
          },
        },
      },
    },
  }
  const richSse = `data: ${JSON.stringify(richFrame)}\n\ndata: [DONE]\n\n`
  const recovered = []
  const recoveryRoot = {
    atob,
    Blob,
    Response,
    DecompressionStream,
    location: { pathname: '/c/conversation-captured-finance' },
    __elonChatGptPrivateStreamPolicy: chatGptPolicy,
    __elonChatGptPrivateStreamTransport: { current: () => ({
      id: 'assistant-captured-finance',
      turnId: 'turn-captured-finance',
      conversationId: 'conversation-captured-finance',
      richParts: [],
    }) },
    __elonWinChatGptPrivateStreamRecovery: {
      accept: (snapshot) => { recovered.push(snapshot); return true },
    },
  }
  assert.equal(
    await capturedFinanceRecovery.recover(recoveryRoot, richSse, 'sse', 7),
    true,
    'a completed cloned response should recover its packed finance chart without replaying a request',
  )
  assert.equal(recovered.length, 1)
  assert.equal(recovered[0].generation, 7)
  assert.equal(recovered[0].messageId, 'assistant-captured-finance')
  assert.equal(recovered[0].richParts[0].kind, 'finance')
  assert.equal(recovered[0].richParts[0].richContent.payload.chart.points.length, 2)
  recoveryRoot.__elonChatGptPrivateStreamTransport.current = () => ({
    id: 'assistant-captured-finance',
    turnId: 'turn-captured-finance',
    conversationId: 'conversation-captured-finance',
    richParts: recovered[0].richParts,
  })
  assert.equal(await capturedFinanceRecovery.recover(recoveryRoot, richSse, 'sse', 7), false)
  assert.equal(recovered.length, 1, 'an already-renderable live rich part must not be duplicated')
  assert.equal(await capturedFinanceRecovery.recover(recoveryRoot, richSse, 'json', 7), false)

  const recoveryCalls = []
  const chatGptResponse = {
    status: 200,
    headers: { get: () => 'text/event-stream' },
    clone() {
      return { status: 200, headers: this.headers, body: null, text: async () => richSse }
    },
  }
  const chatGptWindow = {
    fetch: async () => chatGptResponse,
    XMLHttpRequest: class {},
    __elonWinChatGptCapturedFinanceRecovery: {
      recover: async (...args) => { recoveryCalls.push(args); return true },
    },
    __TAURI_INTERNALS__: { invoke: async () => {} },
  }
  vm.runInNewContext(chatGptSource, {
    window: chatGptWindow,
    XMLHttpRequest: chatGptWindow.XMLHttpRequest,
    location: {
      href: 'https://chatgpt.com/backend-anon/conversation',
      origin: 'https://chatgpt.com',
      pathname: '/',
    },
    URL,
    TextEncoder,
    TextDecoder,
    Set,
    WeakMap,
    Promise,
    Uint8Array,
    console,
  }, { filename: 'win_web_response_research_capture.chatgpt.js' })
  await chatGptWindow.fetch('https://chatgpt.com/backend-anon/conversation', { method: 'POST' })
  await new Promise((resolve) => setImmediate(resolve))
  assert.equal(recoveryCalls.length, 1)
  assert.equal(recoveryCalls[0][0], richSse)
  assert.equal(recoveryCalls[0][1], 'sse')
  console.log('PASS: Win Web response research capture and finance recovery tests')
}

main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
