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
let basePrepareSendCount = 0
let domStreaming = false
let progressNodes = []
const visibleNode = (text = '') => ({
  innerText: text,
  textContent: text,
  getBoundingClientRect: () => ({ width: 120, height: 24 }),
})
const latestTurn = {
  querySelectorAll: () => progressNodes,
}
const main = {
  querySelectorAll: () => progressNodes.length ? [latestTurn] : [],
}
const document = {
  querySelector: (selector) => {
    if (selector === 'main') return main
    if (selector.includes('stop-button') && domStreaming) return visibleNode('停止生成')
    return null
  },
  querySelectorAll: () => progressNodes,
}
const baseListeners = new Set()
const base = {
  version: 9,
  enabled: true,
  current: () => active,
  access: () => ({ blocked: false }),
  mergeMessages: (messages) => messages,
  prepareSend: () => { basePrepareSendCount += 1; active = null },
  reset: () => { baseResetCount += 1; active = null },
  subscribe: (listener) => {
    baseListeners.add(listener)
    return () => baseListeners.delete(listener)
  },
  dispose: () => {},
}

const window = {
  __elonChatGptPrivateStreamTransport: base,
  getComputedStyle: () => ({ display: 'block', visibility: 'visible' }),
}
const context = {
  window,
  document,
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
assert.equal(recovery.baseTransport, base)
assert.equal(transport.__elonWinRichRecoveryWrapped, true)
assert.equal(typeof transport.prepareSend, 'function')

let notifications = 0
transport.subscribe(() => { notifications += 1 })
assert.equal(recovery.accept({
  messageId: 'assistant-one',
  turnId: 'turn-one',
  conversationId: 'conversation-one',
  richParts: [{
    type: 'interactive',
    text: '官网富内容已升级',
    kind: 'renderer_upgrade_required',
  }],
}), true)
const upgradeMerged = transport.mergeMessages([{
  role: 'assistant',
  state: 'completed',
  content: [{ type: 'markdown', text: 'BTC answer' }],
}], '/c/conversation-one')
assert.equal(upgradeMerged[0].content.at(-1).kind, 'renderer_upgrade_required')
assert.equal(recovery.accept({
  messageId: 'assistant-one',
  turnId: 'turn-one',
  conversationId: 'conversation-one',
  richParts: [financePart()],
}), true)
assert.equal(notifications, 2)

const messages = [{
  id: 'dom-assistant',
  role: 'assistant',
  state: 'completed',
  content: [
    { type: 'markdown', text: 'BTC answer' },
    { type: 'interactive', text: 'Bitcoin (BTC)', kind: 'interactive' },
    { type: 'interactive', text: '另一个独立工具', kind: 'interactive' },
  ],
}]
const merged = transport.mergeMessages(messages, '/c/conversation-one')
assert.equal(merged.length, 1, 'recovery must enrich the existing assistant instead of duplicating it')
assert.equal(merged[0].content.filter((part) => part.type === 'rich_card').length, 1)
assert.equal(
  merged[0].content.some((part) => part.type === 'interactive' && part.text === 'Bitcoin (BTC)'),
  false,
  'a generic official placeholder with the recovered finance title must be removed',
)
assert.equal(
  merged[0].content.some((part) => part.type === 'interactive' && part.text === '另一个独立工具'),
  true,
  'an unrelated official interactive component must be preserved',
)
assert.equal(merged[0].content.at(-1).richContent.payload.chart.points.length, 2)
assert.equal(transport.current('/c/conversation-one').richParts.length, 1)
const acceptedDiagnostics = recovery.snapshot()
assert.equal(acceptedDiagnostics.active, true)
assert.equal(acceptedDiagnostics.conversationBound, true)
assert.equal(acceptedDiagnostics.turnBound, true)
assert.equal(acceptedDiagnostics.richKinds.includes('finance'), true)
assert.equal(acceptedDiagnostics.placeholderReconciled, true)
assert.equal(acceptedDiagnostics.acceptedCount, 2)

active = {
  id: 'assistant-stale-stream',
  turnId: 'turn-stale-stream',
  conversationId: 'conversation-one',
  text: 'BTC completed answer',
  state: 'streaming',
  richParts: [],
  updatedAt: Date.now() - 4_000,
}
const completedDom = [{
  id: 'dom-completed-answer',
  role: 'assistant',
  state: 'completed',
  content: [{ type: 'markdown', text: 'BTC completed answer with sources' }],
}]
const completedMerge = transport.mergeMessages(completedDom, '/c/conversation-one')
assert.equal(completedMerge[0].state, 'completed')
assert.equal(
  transport.current('/c/conversation-one').state,
  'completed',
  'a matching completed DOM answer must close a stale private streaming state',
)

active = {
  ...active,
  text: 'BTC completed answer with a new live continuation',
  state: 'streaming',
  updatedAt: Date.now(),
}
assert.equal(
  transport.current('/c/conversation-one').state,
  'streaming',
  'a newer private frame must clear the prior completion override',
)

active = {
  id: 'assistant-progress',
  turnId: 'turn-progress',
  conversationId: 'conversation-one',
  text: '',
  progressLabel: '',
  state: 'streaming',
  richParts: [],
}
progressNodes = [visibleNode('正在搜索 KOSPI today')]
assert.equal(
  transport.current('/c/conversation-one').progressLabel,
  '正在搜索 KOSPI today',
  'Win should recover the visible official search status when the private frame omits it',
)

active = null
domStreaming = true
const progressOnly = transport.current('/c/conversation-one')
assert.equal(
  progressOnly.progressLabel,
  '正在搜索 KOSPI today',
  'visible official progress should bridge a private-stream startup gap',
)
assert.equal(progressOnly.richParts.length, 0, 'a new progress-only turn must not inherit an old rich card')
domStreaming = false
assert.equal(transport.current('/c/conversation-one'), null)

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
const detachedGeneration = recovery.generation()
assert.equal(recovery.accept({
  messageId: 'stale-assistant',
  conversationId: 'conversation-one',
  text: 'BTC answer',
  generation: detachedGeneration,
  richParts: [financePart()],
}), true, 'a generation-bound research response can recover when the live observer missed the stream')
const detachedMerged = transport.mergeMessages(messages, '/')
assert.equal(detachedMerged[0].content.filter((part) => part.type === 'rich_card').length, 1)
const unrelated = transport.mergeMessages([{
  id: 'other-assistant',
  role: 'assistant',
  state: 'completed',
  content: [{ type: 'markdown', text: 'unrelated answer' }],
}], '/')
assert.equal(
  unrelated[0].content.some((part) => part.type === 'rich_card'),
  false,
  'detached recovery must match the official answer text before enrichment',
)

transport.reset()
assert.equal(baseResetCount, 1)
assert.equal(recovery.generation(), detachedGeneration + 1)
transport.reset()
assert.equal(
  baseResetCount,
  1,
  'a recovery retry inside the same new-conversation boundary must preserve the blocked old conversation',
)
assert.equal(recovery.generation(), detachedGeneration + 2)
assert.equal(recovery.accept({
  messageId: 'late-assistant',
  conversationId: 'conversation-one',
  text: 'BTC answer',
  generation: detachedGeneration,
  richParts: [financePart()],
}), false, 'an old response must not cross a new-conversation generation boundary')
assert.equal(recovery.snapshot().lastOutcome, 'stale_generation')
assert.equal(recovery.snapshot().rejectedCount, 2)
assert.equal(transport.mergeMessages(messages, '/'), messages)

transport.prepareSend()
assert.equal(
  basePrepareSendCount,
  0,
  'the first prompt after new conversation must preserve the shared transport old-conversation block',
)
assert.equal(recovery.generation(), detachedGeneration + 3)
transport.prepareSend()
assert.equal(
  basePrepareSendCount,
  1,
  'later prompts in the new conversation must reset the previous completed private turn',
)
console.log('ChatGPT Win private stream recovery tests passed')
