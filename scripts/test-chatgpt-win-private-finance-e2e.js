'use strict'

const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')

const root = path.resolve(__dirname, '..')
const fixturePath = path.join(
  __dirname,
  'fixtures',
  'chatgpt-private-finance-recovery-sanitized.json',
)
const fixtureSource = fs.readFileSync(fixturePath, 'utf8')
const fixture = JSON.parse(fixtureSource)
assert.equal(fixture.schema, 'yilong.chatgpt-private-finance-recovery-fixture.v1')
assert.doesNotMatch(fixtureSource, /(?:authorization|bearer|cookie|set-cookie|signed_url|account_id)/i)

const policy = require(path.join(
  root,
  'android', 'app', 'src', 'main', 'assets',
  'chatgpt_web_private_stream_policy.js',
))
const parsedSession = policy.createSession({ now: () => 10_000 })
parsedSession.begin()
assert.equal(parsedSession.accept(fixture.payload), true)
const parsed = parsedSession.current('/c/synthetic-finance-conversation')
assert.equal(parsed.state, 'completed')
assert.equal(parsed.richParts.length, 1)
assert.equal(parsed.richParts[0].kind, 'finance')
assert.deepEqual(
  parsed.richParts[0].richContent.payload.chart.points.map((point) => Object.keys(point).sort()),
  [['x', 'y'], ['x', 'y'], ['x', 'y']],
  'the private response must become the stable x/y chart contract',
)

const active = { ...parsed, richParts: [] }
const base = {
  version: 9,
  enabled: true,
  current: () => active,
  access: () => ({ blocked: false }),
  mergeMessages: (messages) => messages,
  prepareSend: () => {},
  reset: () => {},
  subscribe: () => () => {},
  dispose: () => {},
}
const window = {
  __elonChatGptPrivateStreamTransport: base,
  getComputedStyle: () => ({ display: 'block', visibility: 'visible' }),
}
const document = {
  querySelector: () => null,
  querySelectorAll: () => [],
}
const context = {
  window,
  document,
  location: {
    origin: 'https://chatgpt.com',
    pathname: '/c/synthetic-finance-conversation',
  },
  Set,
  Date,
  JSON,
}
const recoverySource = fs.readFileSync(path.join(
  root,
  'desktop-shell', 'src-tauri', 'src', 'local_ai_browser',
  'chatgpt_win_private_stream_recovery.js',
), 'utf8')
vm.runInNewContext(recoverySource, context, {
  filename: 'chatgpt_win_private_stream_recovery.js',
})
const recovery = window.__elonWinChatGptPrivateStreamRecovery
assert.equal(recovery.accept({
  messageId: parsed.id,
  turnId: parsed.turnId,
  conversationId: parsed.conversationId,
  text: parsed.text,
  richParts: parsed.richParts,
}), true)

const officialMessages = [{
  id: 'synthetic-finance-user',
  role: 'user',
  state: 'completed',
  content: [{ type: 'text', text: '合成行情问题' }],
}, {
  id: 'synthetic-finance-assistant-stage-one',
  role: 'assistant',
  state: 'completed',
  content: [{ type: 'markdown', text: parsed.text }],
}, {
  id: 'synthetic-finance-assistant-stage-two',
  role: 'assistant',
  state: 'completed',
  content: [
    { type: 'markdown', text: parsed.text },
    { type: 'interactive', text: '交互内容', kind: 'interactive' },
    { type: 'interactive', text: '另一个独立工具', kind: 'interactive' },
  ],
}]
const merged = window.__elonChatGptPrivateStreamTransport.mergeMessages(
  officialMessages,
  context.location.pathname,
)
assert.equal(merged.length, 2, 'one user turn must expose one assistant answer')
assert.equal(merged[1].id, 'synthetic-finance-assistant-stage-two')
assert.equal(merged[1].content.some((part) => part.text === '交互内容'), false)
assert.equal(merged[1].content.some((part) => part.text === '另一个独立工具'), true)
const richCards = merged[1].content.filter((part) => part.type === 'rich_card')
assert.equal(richCards.length, 1)

const typescript = require(path.join(root, 'pc-frontend', 'node_modules', 'typescript'))
const protocolSource = fs.readFileSync(path.join(
  root,
  'pc-frontend', 'src', 'features', 'user-browser', 'richContentProtocol.ts',
), 'utf8')
const compiled = typescript.transpileModule(protocolSource, {
  compilerOptions: {
    module: typescript.ModuleKind.CommonJS,
    target: typescript.ScriptTarget.ES2022,
  },
  fileName: 'richContentProtocol.ts',
}).outputText
const protocolModule = { exports: {} }
new Function('exports', 'module', 'require', compiled)(
  protocolModule.exports,
  protocolModule,
  require,
)
assert.equal(
  protocolModule.exports.isYilongRichContent(richCards[0].richContent),
  true,
  'the recovered private chart must reach the React renderer as a valid rich-content AST',
)

console.log('PASS: sanitized private finance response becomes one Win answer with a native chart and no generic placeholder')
