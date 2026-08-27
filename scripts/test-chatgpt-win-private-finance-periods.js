const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')

const source = fs.readFileSync(path.resolve(
  __dirname,
  '../desktop-shell/src-tauri/src/local_ai_browser/chatgpt_win_private_finance_periods.js',
), 'utf8')

const points = (count, start) => Array.from({ length: count }, (_, index) => ({
  timestamp: index,
  formatted: `T${index}`,
  close: start + index,
}))
const widget = {
  asset_display_name: 'Bitcoin (BTC)',
  current_price_text: 'US$77,274.00',
  default_range: '1D',
  timeframe_order: ['1D', '5D', '1M'],
  timeframe_configs: {
    '1D': {
      chart: { data: points(300, 77000) },
      summary: {
        price_text: 'US$77,274.00',
        price_change_text: '+US$123.00 (0.16%)',
        price_change_color: 'success',
      },
    },
    '5D': {
      chart: { data: points(90, 74000) },
      summary: {
        price_text: 'US$77,274.00',
        price_change_text: '-US$350.00 (0.45%)',
        price_change_color: 'danger',
      },
    },
    '1M': { chart: { data: [{ close: 1 }] }, summary: {} },
  },
}
const original = {
  type: 'rich_card',
  text: 'Bitcoin (BTC)',
  kind: 'finance',
  richContent: {
    schema: 'yilong.rich-content.v1',
    kind: 'finance',
    source: 'private_response',
    payload: {
      title: 'Bitcoin (BTC)',
      primaryValue: 'US$77,274.00',
      trend: 'positive',
      periods: widget.timeframe_order.map((label) => ({
        id: label.toLowerCase(),
        label,
        selected: label === '1D',
      })),
      chart: { kind: 'line', points: points(2, 77000).map((point) => ({ x: point.formatted, y: point.close })) },
    },
  },
}
let acceptedRichParts = null
const session = {
  accept: () => true,
  acceptRichParts(parts, identity) {
    acceptedRichParts = { parts, identity }
    return true
  },
}
const base = Object.freeze({
  assistantFrame: () => null,
  createSession: () => session,
  financePartFromWidget: () => original,
  financePartsFromMetadata: () => [original],
})
const window = { __elonChatGptPrivateStreamPolicy: base }
const sandbox = {
  window,
  location: { origin: 'https://chatgpt.com' },
  Array,
  Object,
  String,
  Number,
  JSON,
  Set,
}
vm.runInNewContext(source, sandbox, { filename: 'chatgpt_win_private_finance_periods.js' })

const bridge = window.__elonWinChatGptPrivateFinancePeriods
assert.ok(bridge)
assert.notEqual(window.__elonChatGptPrivateStreamPolicy, base)
const views = bridge.periodViews(widget)
assert.equal(views.length, 2, 'periods without enough chart data stay non-interactive')
assert.equal(views[0].selected, true)
assert.equal(views[0].chart.points.length, 192, 'each cached period remains bounded')
assert.equal(views[0].chart.points[0].y, 77000)
assert.equal(views[0].chart.points.at(-1).y, 77299)
assert.equal(views[1].trend, 'negative')

const enriched = window.__elonChatGptPrivateStreamPolicy.financePartFromWidget(widget)
assert.equal(enriched.richContent.payload.periodViews.length, 2)
assert.equal(enriched.richContent.payload.periodViews[1].label, '5D')
assert.equal(original.richContent.payload.periodViews, undefined, 'the shared APK policy result stays immutable')
const metadata = window.__elonChatGptPrivateStreamPolicy.financePartsFromMetadata({
  content_references: [{ type: 'dil', dil: { initialState: widget } }],
})
assert.equal(metadata[0].richContent.payload.periodViews[0].id, '1d')

const wrappedSession = window.__elonChatGptPrivateStreamPolicy.createSession()
assert.equal(wrappedSession.accept({
  conversation_id: 'conversation-finance',
  message: {
    id: 'assistant-finance',
    metadata: {
      turn_exchange_id: 'turn-finance',
      content_references: [{ type: 'dil', dil: { initialState: widget } }],
    },
  },
}), true)
assert.ok(acceptedRichParts, 'a live official DIL frame must immediately upgrade the stream card')
assert.equal(acceptedRichParts.parts[0].richContent.payload.periodViews.length, 2)
assert.equal(acceptedRichParts.identity.messageId, 'assistant-finance')
assert.equal(acceptedRichParts.identity.turnId, 'turn-finance')
assert.equal(acceptedRichParts.identity.conversationId, 'conversation-finance')

const replacementBase = Object.freeze(Object.assign({}, base, { generation: 2 }))
window.__elonChatGptPrivateStreamPolicy = replacementBase
vm.runInNewContext(source, sandbox, { filename: 'chatgpt_win_private_finance_periods.js' })
assert.notEqual(window.__elonChatGptPrivateStreamPolicy, replacementBase)
assert.equal(window.__elonWinChatGptPrivateFinancePeriods.basePolicy, replacementBase)
assert.equal(window.__elonChatGptPrivateStreamPolicy.__elonWinPrivateFinancePeriodsWrapped, true)

console.log('ChatGPT Win private finance-period tests passed')
