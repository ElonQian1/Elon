const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')

const source = fs.readFileSync(path.resolve(
  __dirname,
  '../desktop-shell/src-tauri/src/local_ai_browser/chatgpt_win_private_conversation_refresh.js',
), 'utf8')

const calls = []
const delegate = Object.freeze({
  version: 11,
  conversationPrefetchEnabled: true,
  prefetchConversation(path, emit, navigate) {
    calls.push(['prefetch', path])
    emit(snapshot())
    if (typeof navigate === 'function') navigate()
    return true
  },
  refreshCurrentConversation(path, emit) {
    calls.push(['refresh', path])
    emit(snapshot())
    return true
  },
})
const window = {
  __elonChatGptPrivateTransport: delegate,
  __elonWinChatGptConversationRichCache: {
    enrichMessage(message) {
      return {
        ...message,
        content: message.content.concat([{
          type: 'rich_card',
          text: 'Bitcoin (BTC)',
          kind: 'finance',
        }]),
      }
    },
  },
}
const location = {
  origin: 'https://chatgpt.com',
  pathname: '/g/g-p-roadmap/c/conversation-one',
}
vm.runInNewContext(source, { window, location, Object, Array, String, Number }, {
  filename: 'chatgpt_win_private_conversation_refresh.js',
})

const transport = window.__elonChatGptPrivateTransport
assert.notEqual(transport, delegate)
assert.equal(transport.__elonWinConversationRefreshWrapped, true)

let emitted
assert.equal(transport.refreshCurrentConversation(location.pathname, (event) => { emitted = event }), true)
assert.deepEqual(calls.pop(), ['prefetch', location.pathname])
assert.equal(emitted.url, 'https://chatgpt.com/g/g-p-roadmap/c/conversation-one')
assert.equal(emitted.accessSource, 'private_response')
assert.equal(JSON.stringify(emitted.messages[0].content), JSON.stringify([
  { type: 'markdown', text: '完整历史正文' },
  { type: 'rich_card', text: 'Bitcoin (BTC)', kind: 'finance' },
]))
assert.equal(emitted.observedMessageCount, 1)

assert.equal(transport.refreshCurrentConversation('/c/stale-conversation', () => {}), false)
assert.equal(calls.length, 0, 'a stale conversation must never issue a private refresh')

location.pathname = '/c/conversation-two'
emitted = null
assert.equal(transport.refreshCurrentConversation(location.pathname, (event) => { emitted = event }), true)
assert.deepEqual(calls.pop(), ['refresh', location.pathname])
assert.equal(emitted.url, 'https://chatgpt.com/c/conversation-two')

let navigated = false
assert.equal(transport.prefetchConversation('/c/conversation-three', () => {}, () => {
  navigated = true
}), true)
assert.equal(navigated, true)
assert.deepEqual(calls.pop(), ['prefetch', '/c/conversation-three'])

function snapshot() {
  return {
    type: 'message_snapshot',
    url: 'https://chatgpt.com/c/canonicalized',
    messages: [{
      id: 'assistant-one',
      role: 'assistant',
      state: 'completed',
      content: '完整历史正文',
      parts: [],
    }],
  }
}

console.log('ChatGPT Win private conversation refresh tests passed')
