const assert = require('node:assert/strict')
const path = require('node:path')

const bridge = require(path.resolve(
  __dirname,
  '../desktop-shell/src-tauri/src/local_ai_browser/chatgpt_win_private_transport_health.js',
))

const root = {
  __elonChatGptPrivateTransport: {
    conversationPrefetchEnabled: true,
    conversationPrefetchReady: () => true,
    health: () => ({
      officialFresh: true,
      privateLatencyMs: 280,
      successes: 4,
      failures: 1,
      lastOutcome: 'success',
      attemptBudgetMs: 700,
    }),
  },
  __elonWinChatGptPrivateStreamRecovery: {
    snapshot: () => ({
      version: 1,
      generation: 9,
      active: true,
      detached: false,
      conversationBound: true,
      turnBound: false,
      messageBound: true,
      richKinds: ['finance'],
      acceptedCount: 2,
      rejectedCount: 1,
      lastOutcome: 'accepted',
      placeholderReconciled: true,
      sampledAtMs: 1234,
    }),
  },
}

const original = JSON.stringify({
  schema: 'yilong.ai.ui.v1',
  event: { type: 'message_snapshot', messages: [] },
})
const enriched = JSON.parse(bridge.enrich(root, original))
assert.equal(enriched.event.privateTransportHealth.privateLatencyMs, 280)
assert.equal(enriched.event.privateRichRecovery.generation, 9)
assert.deepEqual(enriched.event.privateRichRecovery.richKinds, ['finance'])
assert.equal(enriched.event.privateRichRecovery.placeholderReconciled, true)

const navigation = JSON.stringify({ event: { type: 'navigation_snapshot' } })
assert.equal(
  bridge.enrich(root, navigation),
  navigation,
  'diagnostics must only enrich message snapshots',
)

process.stdout.write('PASS ChatGPT Win private transport and rich-recovery diagnostics\n')
