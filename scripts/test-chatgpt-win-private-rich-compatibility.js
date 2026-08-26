const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')

const root = path.resolve(__dirname, '..')
const basePolicy = require(path.join(
  root,
  'android/app/src/main/assets/chatgpt_web_private_stream_policy.js',
))
const compatibility = require(path.join(
  root,
  'desktop-shell/src-tauri/src/local_ai_browser/chatgpt_win_private_rich_compatibility.js',
))
const compatibilitySource = fs.readFileSync(path.join(
  root,
  'desktop-shell/src-tauri/src/local_ai_browser/chatgpt_win_private_rich_compatibility.js',
), 'utf8')

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

function assistantFrameFor(id, text, conversationId, turnId) {
  const frame = assistantFrame(id, text)
  frame.conversation_id = conversationId
  frame.message.metadata.turn_exchange_id = turnId
  return frame
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
  packedFinanceWidgets: (payload) => payload && payload.packed ? [widget] : [],
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
  packedFinanceWidgets: (payload) => payload && payload.packed ? [widget] : [],
}))
const validSession = validPolicy.createSession({ now: () => ++clock })
validSession.begin()
validSession.accept(assistantFrame('assistant-rich-valid', '行情正文。'))
validSession.accept({ packed: true })
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
assert.equal(validSession.acceptRichParts([financePart], widget), true)
validSession.finish()
const valid = validSession.current('/c/conversation-rich-compatibility')
assert.equal(valid.richParts.length, 1)
assert.equal(valid.richParts[0].kind, 'finance')
assert.equal(validPolicy.richCompatibility().rendererUpgradeRequired, false)

const isolatedPolicy = compatibility.enhancePolicy(Object.assign({}, basePolicy, {
  packedFinanceWidgets: (payload) => payload && payload.packed ? [widget] : [],
}))
const failedSession = isolatedPolicy.createSession({ now: () => ++clock })
failedSession.begin()
failedSession.accept(assistantFrame('assistant-isolated-failed', '第一会话正文。'))
failedSession.accept({ packed: true })
failedSession.finish()
const successfulSession = isolatedPolicy.createSession({ now: () => ++clock })
successfulSession.begin()
successfulSession.accept(assistantFrame('assistant-isolated-chart', '第二会话正文。'))
assert.equal(successfulSession.acceptRichParts([{
  type: 'rich_card',
  text: 'BTC 趋势',
  kind: 'chart',
  richContent: {
    schema: 'yilong.rich-content.v1',
    kind: 'chart',
    source: 'private_response',
    payload: {
      title: 'BTC 趋势',
      chartType: 'line',
      series: [{ key: 'price', label: '价格' }],
      points: [{ x: '10:00', values: [1] }, { x: '11:00', values: [2] }],
    },
  },
}], widget), true)
successfulSession.finish()
assert.equal(
  failedSession.current('/c/conversation-rich-compatibility').richParts[0].kind,
  'renderer_upgrade_required',
  'another parser session must not clear the first conversation compatibility state',
)
assert.equal(
  successfulSession.current('/c/conversation-rich-compatibility').richParts[0].kind,
  'chart',
  'a supported chart must not be accompanied by an upgrade placeholder',
)

const stalePolicy = Object.assign({}, basePolicy, {
  __elonWinPrivateRichCompatibilityWrapped: true,
})
const upgradeWindow = {
  __elonChatGptPrivateStreamPolicy: stalePolicy,
  __elonWinChatGptPrivateRichCompatibility: Object.freeze({
    version: 4,
    basePolicy,
    policy: stalePolicy,
  }),
}
vm.runInNewContext(compatibilitySource, { window: upgradeWindow }, {
  filename: 'chatgpt_win_private_rich_compatibility.js',
})
assert.equal(upgradeWindow.__elonWinChatGptPrivateRichCompatibility.version, 5)
assert.notEqual(upgradeWindow.__elonChatGptPrivateStreamPolicy, stalePolicy)
assert.equal(upgradeWindow.__elonWinChatGptPrivateRichCompatibility.basePolicy, basePolicy)

const delayedPolicy = compatibility.enhancePolicy(basePolicy)
const delayedSession = delayedPolicy.createSession({ now: () => ++clock })
const staleWidget = Object.assign({}, widget, {
  messageId: 'assistant-stale-widget',
  turnId: 'turn-stale-widget',
  conversationId: 'conversation-stale-widget',
})
delayedSession.begin()
assert.equal(
  delayedSession.acceptRichParts([financePart], staleWidget),
  false,
  'a decoded widget must wait until the response identity is known',
)
delayedSession.accept(assistantFrameFor(
  'assistant-next-response',
  '下一会话正文。',
  'conversation-next-response',
  'turn-next-response',
))
const delayed = delayedSession.current('/c/conversation-next-response')
assert.equal(delayed.text, '下一会话正文。')
assert.equal(delayed.richParts.length, 0, 'a late widget from the previous conversation must be discarded')

const earlyPolicy = compatibility.enhancePolicy(basePolicy)
const earlySession = earlyPolicy.createSession({ now: () => ++clock })
const earlyWidget = Object.assign({}, widget, {
  messageId: 'assistant-early-widget',
  turnId: 'turn-early-widget',
  conversationId: 'conversation-early-widget',
})
earlySession.begin()
assert.equal(earlySession.acceptRichParts([financePart], earlyWidget), false)
earlySession.accept(assistantFrameFor(
  'assistant-early-widget',
  '同一会话正文。',
  'conversation-early-widget',
  'turn-early-widget',
))
const early = earlySession.current('/c/conversation-early-widget')
assert.equal(early.text, '同一会话正文。')
assert.equal(early.richParts.length, 1, 'an early widget must bind after its matching response arrives')
assert.equal(early.richParts[0].kind, 'finance')

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

console.log('PASS: Win private rich compatibility isolates sessions, binds delayed widgets by response identity, preserves text, and treats finance or chart decoding as supported')
