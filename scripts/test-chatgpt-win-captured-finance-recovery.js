const assert = require('node:assert/strict')
const zlib = require('node:zlib')

const recovery = require('../desktop-shell/src-tauri/src/local_ai_browser/chatgpt_win_captured_finance_recovery.js')

const basicPart = {
  type: 'rich_card',
  text: 'Bitcoin (BTC)',
  kind: 'finance',
  richContent: {
    kind: 'finance',
    payload: {
      title: 'Bitcoin (BTC)',
      chart: { kind: 'line', points: [{ x: '12:00', y: 77000 }, { x: '13:00', y: 77100 }] },
    },
  },
}
const periodViews = ['1D', '5D', '1M'].map((label, index) => ({
  id: label.toLowerCase(),
  label,
  selected: index === 0,
  chart: {
    kind: 'line',
    points: [{ x: 'start', y: 77000 - index * 100 }, { x: 'end', y: 77100 + index * 100 }],
  },
}))
const enhancedPart = {
  ...basicPart,
  richContent: {
    ...basicPart.richContent,
    payload: { ...basicPart.richContent.payload, periodViews },
  },
}
const packedValue = { asset_display_name: 'Bitcoin (BTC)', timeframe_configs: {} }
const compressed = zlib.gzipSync(JSON.stringify(packedValue)).toString('base64url')
const packedWidget = {
  encoding: 'gzip-json-base64url-v1',
  messageId: 'assistant-finance',
  widgetId: 'finance-widget',
  compressed,
}
const snapshot = {
  id: 'assistant-finance',
  turnId: 'turn-finance',
  conversationId: 'conversation-finance',
  text: 'BTC answer',
  richParts: [basicPart],
}
const session = {
  begin() {},
  accept() {},
  finish() {},
  current: () => snapshot,
  packedWidgets: () => [packedWidget],
}
const policy = {
  createSession: () => session,
  createSseDecoder: () => ({ push() {}, finish() {} }),
  financePartFromWidget: () => enhancedPart,
}
let accepted = null
const root = {
  atob: globalThis.atob,
  Blob: globalThis.Blob,
  DecompressionStream: globalThis.DecompressionStream,
  TextDecoder: globalThis.TextDecoder,
  location: { pathname: '/c/conversation-finance' },
  __elonChatGptPrivateStreamPolicy: policy,
  __elonChatGptPrivateStreamTransport: { current: () => null },
  __elonWinChatGptPrivateStreamRecovery: {
    accept(value) {
      accepted = value
      return true
    },
  },
}

async function main() {
  assert.equal(await recovery.recover(root, 'data: {}\n\n', 'sse', 7), true)
  assert.ok(accepted)
  assert.equal(accepted.richParts.length, 1, 'the enhanced widget must replace the duplicate basic card')
  assert.deepEqual(
    accepted.richParts[0].richContent.payload.periodViews.map((view) => view.label),
    ['1D', '5D', '1M'],
    'packed private response periods must survive rich-card deduplication',
  )
  assert.equal(accepted.generation, 7)

  accepted = null
  root.__elonChatGptPrivateStreamTransport = { current: () => ({
    ...snapshot,
    richParts: [basicPart],
  }) }
  assert.equal(
    await recovery.recover(root, 'data: {}\n\n', 'sse', 8),
    true,
    'a basic live card must not block a richer completed-response upgrade',
  )
  assert.equal(accepted.richParts[0].richContent.payload.periodViews.length, 3)

  accepted = null
  root.__elonChatGptPrivateStreamTransport = { current: () => ({
    ...snapshot,
    richParts: [enhancedPart],
  }) }
  assert.equal(
    await recovery.recover(root, 'data: {}\n\n', 'sse', 9),
    false,
    'an equal-quality live card must not be replayed again',
  )
  assert.equal(accepted, null)
  console.log('ChatGPT Win captured finance recovery tests passed')
}

main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
