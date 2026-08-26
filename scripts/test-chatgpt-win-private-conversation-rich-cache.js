const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')
const zlib = require('node:zlib')

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
let activeResponsePayload = responsePayload

let fetchCount = 0
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
const financeWidget = {
  asset_display_name: 'Bitcoin (BTC)',
  current_price_text: 'US$78,805.00',
  default_range: '1D',
  timeframe_order: ['1D'],
  timeframe_configs: {
    '1D': {
      summary: { price_text: 'US$78,805.00', price_change_text: '+1.81%' },
      chart: {
        data: [
          { formatted: '08:00', close: 77_000 },
          { formatted: '09:00', close: 78_805 },
        ],
      },
    },
  },
  metrics_display: [{ cols: [{ label: '日内最高价', value: 'US$79,934.00' }] }],
}
const packedFinance = {
  __encoding: 'gzip-json-base64url-v1',
  __compressed: zlib.gzipSync(JSON.stringify(financeWidget)).toString('base64url'),
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
  packedFinanceWidgets: ({ message, conversation_id: conversationId }) => (
    message.id === 'assistant-one'
      ? [{
        widgetId: 'finance-one',
        messageId: message.id,
        conversationId,
        turnId: 'turn-one',
        encoding: packedFinance.__encoding,
        compressed: packedFinance.__compressed,
      }]
      : []
  ),
  financePartFromWidget: (widget) => ({
    type: 'rich_card',
    text: widget.asset_display_name,
    kind: 'finance',
    richContent: {
      schema: 'yilong.rich-content.v1',
      kind: 'finance',
      source: 'private_response',
      payload: {
        title: widget.asset_display_name,
        primaryValue: widget.current_price_text,
        trend: 'positive',
        periods: [{ id: '1d', label: '1D', selected: true }],
        metrics: widget.metrics_display[0].cols,
        chart: {
          kind: 'line',
          points: widget.timeframe_configs['1D'].chart.data.map((point) => ({
            x: point.formatted,
            y: point.close,
          })),
        },
      },
    },
  }),
}
let recoveredFinance = null
const window = {
  __elonChatGptPrivateStreamPolicy: policy,
  __elonWinChatGptPrivateStreamRecovery: {
    accept: (snapshot) => {
      recoveredFinance = snapshot
      return true
    },
  },
  setTimeout,
  fetch: async () => {
    fetchCount += 1
    return {
      ok: true,
      json: async () => activeResponsePayload,
    }
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
  Blob,
  TextDecoder,
  DecompressionStream,
  Uint8Array,
  atob,
  setTimeout,
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
  assert.equal(enriched.content.filter((part) => part.type === 'rich_card').length, 2)
  assert.equal(enriched.content.find((part) => part.kind === 'chart').kind, 'chart')
  const finance = enriched.content.find((part) => part.kind === 'finance')
  assert.ok(finance)
  assert.equal(finance.richContent.payload.chart.points.length, 2)
  assert.equal(finance.richContent.payload.periods.length, 1)
  assert.equal(finance.richContent.payload.metrics.length, 1)
  assert.ok(recoveredFinance)
  assert.equal(recoveredFinance.messageId, 'assistant-one')
  assert.equal(recoveredFinance.conversationId, 'conversation-one')
  assert.equal(recoveredFinance.richParts[0].kind, 'finance')

  const stale = cache.enrichMessage({
    id: 'assistant-one',
    role: 'assistant',
    content: [{ type: 'markdown', text: '另一个会话' }],
  }, '/c/conversation-two')
  assert.equal(stale.content[0].text, '另一个会话')

  const reboundPayload = {
    mapping: {
      assistant: {
        message: {
          id: 'assistant-two',
          author: { role: 'assistant' },
          status: 'finished_successfully',
          content: { content_type: 'text', parts: ['ETH 正在回升。'] },
          metadata: {},
        },
      },
    },
  }
  const reboundPolicy = {
    assistantFrame: ({ message }) => message.id === 'assistant-two' ? {
      text: 'ETH 使用重连后的新策略解析。',
      citations: [{
        type: 'citation',
        text: 'Example',
        url: 'https://example.com/',
        markerText: 'Example',
        groupSize: 1,
      }],
    } : null,
    clientChartPartFromMetadata: () => null,
    financePartsFromMetadata: () => [],
    packedFinanceWidgets: () => [],
  }
  activeResponsePayload = reboundPayload
  window.__elonChatGptPrivateStreamPolicy = reboundPolicy
  const installedFetch = window.fetch
  vm.runInNewContext(source, context, {
    filename: 'chatgpt_win_private_conversation_rich_cache.js',
  })
  assert.equal(window.fetch, installedFetch, 'reconnect must not stack another fetch wrapper')
  assert.equal(window.__elonWinChatGptConversationRichCache, cache)
  assert.equal(cache.diagnostics(), 'v2|bindings=2|entries=1')

  const reboundResponse = await window.fetch('/backend-api/conversations/conversation-two', {
    method: 'GET',
    __elonPrivateTransport: 'conversation_prefetch',
  })
  assert.equal(fetchCount, 2)
  assert.equal(await reboundResponse.json(), reboundPayload)
  const rebound = cache.enrichMessage({
    id: 'assistant-two',
    role: 'assistant',
    state: 'completed',
    content: [{ type: 'markdown', text: '旧策略正文' }],
  }, '/c/conversation-two')
  assert.match(rebound.content[0].text, /重连后的新策略/)
  assert.equal(rebound.content.filter((part) => part.type === 'citation').length, 1)
  assert.equal(cache.diagnostics(), 'v2|bindings=2|entries=2')
  assert.equal(window.fetch.__elonWinPrivateConversationRichCacheWrapped, true)
  console.log('ChatGPT Win private conversation rich-cache tests passed')
}

main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
