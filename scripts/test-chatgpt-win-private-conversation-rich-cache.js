const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')

const source = fs.readFileSync(path.resolve(
  __dirname,
  '../desktop-shell/src-tauri/src/local_ai_browser/chatgpt_win_private_conversation_rich_cache.js',
), 'utf8')

const responsePayload = {
  mapping: {
    user: {
      message: {
        id: 'user-one',
        author: { role: 'user' },
        content: { content_type: 'text', parts: ['BTC 怎么样？'] },
      },
    },
    assistant: {
      message: {
        id: 'assistant-one',
        author: { role: 'assistant' },
        status: 'finished_successfully',
        content: { content_type: 'text', parts: ['BTC 正在回升。'] },
        metadata: { content_references: [{ type: 'client_defined_widget' }] },
      },
    },
  },
}

let fetchCount = 0
const response = {
  ok: true,
  json: async () => responsePayload,
}
const chart = {
  type: 'rich_card',
  text: 'Bitcoin (BTC)',
  kind: 'chart',
  richContent: {
    schema: 'yilong.rich-content.v1',
    kind: 'chart',
    source: 'private_response',
    payload: { title: 'Bitcoin (BTC)', chartType: 'line', series: [], points: [] },
  },
}
const policy = {
  assistantFrame: ({ message }) => message.id === 'assistant-one' ? {
    text: 'BTC [Reuters](https://reuters.com/) 正在回升。',
    citations: [{
      type: 'citation',
      text: 'Reuters',
      url: 'https://reuters.com/',
      markerText: 'Reuters',
      groupSize: 1,
    }],
  } : null,
  clientChartPartFromMetadata: () => chart,
  financePartsFromMetadata: () => [],
  packedFinanceWidgets: () => [],
  financePartFromWidget: () => null,
}
const window = {
  __elonChatGptPrivateStreamPolicy: policy,
  fetch: async () => {
    fetchCount += 1
    return response
  },
}
const context = {
  window,
  location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/', pathname: '/' },
  URL,
  Map,
  Set,
  Promise,
  Array,
  Object,
  String,
  Number,
  Date,
  JSON,
}

vm.runInNewContext(source, context, {
  filename: 'chatgpt_win_private_conversation_rich_cache.js',
})

async function main() {
  const intercepted = await window.fetch('/backend-api/conversations/conversation-one', {
    method: 'GET',
    __elonPrivateTransport: 'conversation_prefetch',
  })
  assert.equal(fetchCount, 1)
  assert.equal(await intercepted.json(), responsePayload)

  const cache = window.__elonWinChatGptConversationRichCache
  assert.ok(cache)
  assert.equal(cache.size(), 1)
  const enriched = cache.enrichMessage({
    id: 'assistant-one',
    role: 'assistant',
    state: 'completed',
    content: [{ type: 'markdown', text: '缺少富内容的旧正文' }],
  }, '/c/conversation-one')
  assert.equal(enriched.content[0].type, 'markdown')
  assert.match(enriched.content[0].text, /Reuters/)
  assert.equal(enriched.content.filter((part) => part.type === 'citation').length, 1)
  assert.equal(enriched.content.filter((part) => part.type === 'rich_card').length, 1)
  assert.equal(enriched.content.find((part) => part.type === 'rich_card').kind, 'chart')

  const stale = cache.enrichMessage({
    id: 'assistant-one',
    role: 'assistant',
    content: [{ type: 'markdown', text: '另一个会话' }],
  }, '/c/conversation-two')
  assert.equal(stale.content[0].text, '另一个会话')
  console.log('ChatGPT Win private conversation rich-cache tests passed')
}

main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
