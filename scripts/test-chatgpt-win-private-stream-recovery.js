const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')

const source = fs.readFileSync(path.resolve(
  __dirname,
  '../desktop-shell/src-tauri/src/local_ai_browser/chatgpt_win_private_stream_recovery.js',
), 'utf8')

function financePart(title = 'Bitcoin (BTC)', sourceName = 'private_response') {
  return {
    type: 'rich_card',
    text: title,
    kind: 'finance',
    richContent: {
      schema: 'yilong.rich-content.v1',
      kind: 'finance',
      source: sourceName,
      payload: {
        title,
        primaryValue: 'US$78,805.00',
        periods: [{ id: '1d', label: '1D' }],
        chart: {
          kind: 'line',
          points: [{ label: '12:00', value: 77000 }, { label: '13:00', value: 78805 }],
        },
      },
    },
  }
}

let active = {
  id: 'assistant-one',
  turnId: 'turn-one',
  conversationId: 'conversation-one',
  text: 'BTC answer',
  state: 'completed',
  richParts: [],
}
let baseResetCount = 0
const baseListeners = new Set()
const base = {
  version: 9,
  enabled: true,
  current: () => active,
  access: () => ({ blocked: false }),
  mergeMessages: (messages) => messages,
  reset: () => { baseResetCount += 1; active = null },
  subscribe: (listener) => {
    baseListeners.add(listener)
    return () => baseListeners.delete(listener)
  },
  dispose: () => {},
}

const window = { __elonChatGptPrivateStreamTransport: base }
const context = {
  window,
  location: { origin: 'https://chatgpt.com', pathname: '/c/conversation-one' },
  Set,
  Date,
  JSON,
}
vm.runInNewContext(source, context, { filename: 'chatgpt_win_private_stream_recovery.js' })

const recovery = window.__elonWinChatGptPrivateStreamRecovery
const transport = window.__elonChatGptPrivateStreamTransport
assert.ok(recovery)
assert.notEqual(transport, base)
assert.equal(transport.__elonWinRichRecoveryWrapped, true)

let notifications = 0
transport.subscribe(() => { notifications += 1 })
assert.equal(recovery.accept({
  messageId: 'assistant-one',
  turnId: 'turn-one',
  conversationId: 'conversation-one',
  richParts: [financePart()],
}), true)
assert.equal(notifications, 1)

const messages = [{
  id: 'dom-assistant',
  role: 'assistant',
  state: 'completed',
  content: [
    { type: 'markdown', text: 'BTC answer' },
    { type: 'interactive', text: 'Bitcoin (BTC)', kind: 'renderer_upgrade_required' },
  ],
}]
const merged = transport.mergeMessages(messages, '/c/conversation-one')
assert.equal(merged.length, 1, 'recovery must enrich the existing assistant instead of duplicating it')
assert.equal(merged[0].content.filter((part) => part.type === 'rich_card').length, 1)
assert.equal(merged[0].content.some((part) => part.type === 'interactive'), false)
assert.equal(merged[0].content.at(-1).richContent.payload.chart.points.length, 2)
assert.equal(transport.current('/c/conversation-one').richParts.length, 1)

const privateBase = financePart('Bitcoin (BTC)')
const privateWins = transport.mergeMessages([{
  role: 'assistant',
  content: [{ type: 'markdown', text: 'answer' }, privateBase],
}], '/c/conversation-one')
assert.equal(privateWins[0].content.filter((part) => part.type === 'rich_card').length, 1)
assert.equal(privateWins[0].content[1], privateBase, 'the live private transport remains authoritative')

context.location.pathname = '/c/conversation-two'
assert.equal(recovery.accept({
  messageId: 'assistant-two',
  conversationId: 'conversation-three',
  richParts: [financePart('Ether (ETH)')],
}), false, 'a late response must never enter a different active conversation')

context.location.pathname = '/'
active = null
assert.equal(recovery.accept({
  messageId: 'stale-assistant',
  conversationId: 'conversation-one',
  richParts: [financePart()],
}), false, 'a reset root route requires matching live-stream identity')

transport.reset()
assert.equal(baseResetCount, 1)
assert.equal(transport.mergeMessages(messages, '/'), messages)
console.log('ChatGPT Win private stream recovery tests passed')
