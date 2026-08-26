const assert = require('node:assert/strict')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const basePolicy = require(path.join(
  root,
  'android/app/src/main/assets/chatgpt_web_private_stream_policy.js',
))
const compatibility = require(path.join(
  root,
  'desktop-shell/src-tauri/src/local_ai_browser/chatgpt_win_private_rich_compatibility.js',
))

let clock = 10_000

function assistantFrame(id, text) {
  return {
    conversation_id: 'conversation-rich-compatibility',
    message: {
      id,
      author: { role: 'assistant' },
      status: 'finished_successfully',
      content: { content_type: 'text', parts: [text] },
      metadata: { turn_exchange_id: 'turn-rich-compatibility' },
    },
  }
}

function packedWidget() {
  return {
    encoding: 'gzip-json-base64url-v1',
    compressed: 'not-a-real-upstream-widget',
    conversationId: 'conversation-rich-compatibility',
    turnId: 'turn-rich-compatibility',
    messageId: 'assistant-rich-compatibility',
    widgetId: 'widget-rich-compatibility',
  }
}

const widget = packedWidget()
assert.equal(compatibility.packedWidgetKey(null), '')
assert.equal(compatibility.packedWidgetKey({}), '')
const fakePayloadPolicy = compatibility.enhancePolicy(Object.assign({}, basePolicy, {
  packedFinanceWidgets: () => [widget],
}))
assert.equal(fakePayloadPolicy.__elonWinPrivateRichCompatibilityWrapped, true)
assert.equal(compatibility.enhancePolicy(fakePayloadPolicy), fakePayloadPolicy)
const fakeSession = fakePayloadPolicy.createSession({ now: () => ++clock })
fakeSession.begin()
fakeSession.accept(assistantFrame('assistant-rich-compatibility', '正文仍然应该完整显示。'))
assert.equal(
  fakeSession.accept({ packed: true }),
  true,
  'a standalone packed-widget SSE frame must reach the Win decoder even without a message body',
)
fakePayloadPolicy.packedFinanceWidgets({ packed: true })
fakeSession.finish()

const failed = fakeSession.current('/c/conversation-rich-compatibility')
assert.equal(failed.text, '正文仍然应该完整显示。')
assert.deepEqual(failed.richParts, [{
  type: 'interactive',
  text: '官网富内容已升级',
  kind: 'renderer_upgrade_required',
}])
assert.deepEqual(fakePayloadPolicy.richCompatibility(), {
  packedWidgetCount: 1,
  convertedWidgetCount: 0,
  rendererUpgradeRequired: true,
})

const merged = fakeSession.merge([], '/c/conversation-rich-compatibility')
assert.equal(merged.length, 1)
assert.equal(merged[0].content[0].text, '正文仍然应该完整显示。')
assert.equal(merged[0].content[1].kind, 'renderer_upgrade_required')

fakeSession.reset()
assert.equal(fakePayloadPolicy.richCompatibility().packedWidgetCount, 0)
assert.equal(fakeSession.current('/c/conversation-rich-compatibility'), null)

const validPolicy = compatibility.enhancePolicy(Object.assign({}, basePolicy, {
  packedFinanceWidgets: () => [widget],
}))
const validSession = validPolicy.createSession({ now: () => ++clock })
validSession.begin()
validSession.accept(assistantFrame('assistant-rich-valid', '行情正文。'))
validPolicy.packedFinanceWidgets({ packed: true })
const financePart = validPolicy.financePartFromWidget({
  asset_display_name: 'Bitcoin (BTC)',
  current_price_text: 'US$77,000.00',
  default_range: '1D',
  timeframe_order: ['1D'],
  timeframe_configs: {
    '1D': {
      summary: { price_text: 'US$77,000.00', price_change_text: '+1.2%' },
      chart: { data: [
        { formatted: '10:00', close: 76_900 },
        { formatted: '11:00', close: 77_000 },
      ] },
    },
  },
})
assert.equal(financePart.kind, 'finance')
validSession.acceptRichParts([financePart], widget)
validSession.finish()
const valid = validSession.current('/c/conversation-rich-compatibility')
assert.equal(valid.richParts.length, 1)
assert.equal(valid.richParts[0].kind, 'finance')
assert.equal(validPolicy.richCompatibility().rendererUpgradeRequired, false)

const renderedPolicy = compatibility.enhancePolicy(basePolicy)
const renderedStream = {
  id: 'assistant-rendered-equivalent',
  conversationId: 'conversation-rendered-equivalent',
  turnId: 'turn-rendered-equivalent',
  text: '**当前走势**\n\n- BTC 价格约为 **US$77,000**，短线保持震荡。',
  state: 'completed',
  citations: [],
  richParts: [financePart],
}
const renderedMessages = [{
  id: 'official-dom-assistant',
  role: 'assistant',
  state: 'completed',
  content: [{
    type: 'markdown',
    text: '当前走势 BTC 价格约为 US$77,000，短线保持震荡。',
  }],
}]
const renderedMerged = compatibility.mergeRenderedReply(
  basePolicy,
  renderedMessages,
  renderedStream,
)
assert.equal(renderedMerged.length, 1, 'rendered DOM text and Markdown stream text are one answer')
assert.equal(renderedMerged[0].id, 'official-dom-assistant')
assert.equal(renderedMerged[0].content.filter((part) => part.kind === 'finance').length, 1)
assert.equal(
  compatibility.sameRenderedReply(renderedMessages[0].content[0].text, renderedStream.text),
  true,
)

console.log('PASS: Win private rich compatibility accepts standalone widget frames, preserves text, flags failed widgets, and clears on reset or successful finance decoding')
