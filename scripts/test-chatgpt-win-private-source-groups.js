const assert = require('node:assert/strict')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const basePolicy = require(path.join(
  root,
  'android/app/src/main/assets/chatgpt_web_private_stream_policy.js',
))
const sourceGroups = require(path.join(
  root,
  'desktop-shell/src-tauri/src/local_ai_browser/chatgpt_win_private_source_groups.js',
))

const policy = sourceGroups.enhancePolicy(basePolicy)
assert.equal(policy.__elonWinPrivateSourceGroupsWrapped, true)
assert.equal(sourceGroups.enhancePolicy(policy), policy)
let clock = 1_000
const session = policy.createSession({ now: () => ++clock })

session.begin()
assert.equal(session.accept({
  c: 1,
  v: {
    conversation_id: 'conversation-source-1',
    message: {
      author: { role: 'tool' },
      content: { content_type: 'text', parts: [] },
      metadata: {
        turn_exchange_id: 'turn-source-1',
        tool_icons: [
          'https://cdn.reuters.com/icon.png?signature=drop-me',
          'https://cdn.marketwatch.com/icon.png',
        ],
        search_result_groups: [
          {
            domain: 'reuters.com',
            entries: [{
              title: 'Markets recover after a volatile open',
              attribution: 'Reuters',
              snippet: 'A concise public result summary for the native source card.',
              thumbnail_url: 'https://cdn.reuters.com/market.jpg?tracking=drop-me',
              url: 'https://www.reuters.com/markets/recovery/?utm_source=chatgpt',
            }],
          },
          {
            domain: 'marketwatch.com',
            entries: [{
              title: 'What investors are watching',
              snippet: 'A second result from the same private response structure.',
              url: 'https://www.marketwatch.com/story/investors-watch',
            }],
          },
        ],
      },
    },
  },
}), true, 'tool-only source groups must be retained until the assistant frame arrives')

assert.equal(session.accept({
  conversation_id: 'conversation-source-1',
  message: {
    id: 'assistant-source-1',
    author: { role: 'assistant' },
    status: 'finished_successfully',
    content: { content_type: 'text', parts: ['Markets recovered today.'] },
    metadata: { turn_exchange_id: 'turn-source-1', content_references: [] },
  },
}), true)

session.acceptRichParts([{
  type: 'rich_card',
  text: 'Bitcoin (BTC)',
  kind: 'finance',
  richContent: {
    schema: 'yilong.rich-content.v1',
    kind: 'finance',
    source: 'private_response',
    payload: { title: 'Bitcoin (BTC)' },
  },
}], {
  conversationId: 'conversation-source-1',
  turnId: 'turn-source-1',
  messageId: 'assistant-source-1',
})

const current = session.current('/c/conversation-source-1')
assert.equal(current.citations.length, 2)
assert.equal(current.citations[0].text, 'Markets recover after a volatile open')
assert.equal(current.citations[0].url, 'https://www.reuters.com/markets/recovery/')
assert.equal(current.citations[0].iconUrl, 'https://cdn.reuters.com/icon.png')
assert.equal(current.citations[0].thumbnailUrl, 'https://cdn.reuters.com/market.jpg')
assert.match(current.citations[0].snippet, /native source card/)
assert.equal(current.citations[1].targetHost, 'marketwatch.com')

const merged = session.merge([], '/c/conversation-source-1')
assert.equal(merged.length, 1)
assert.equal(merged[0].content.filter((part) => part.type === 'citation').length, 2)
assert.equal(
  merged[0].content.filter((part) => part.type === 'rich_card').length,
  1,
  'bounded source enrichment must not evict an existing finance card',
)

session.begin()
session.accept({
  conversation_id: 'conversation-source-2',
  message: {
    id: 'assistant-source-2',
    author: { role: 'assistant' },
    status: 'finished_successfully',
    content: { content_type: 'text', parts: ['A new conversation answer.'] },
    metadata: { turn_exchange_id: 'turn-source-2' },
  },
})
assert.equal(
  session.current('/c/conversation-source-2').citations.length,
  0,
  'new turns and conversations must never inherit an earlier source group',
)

console.log('PASS: Win private source groups become bounded native citations with public logos and no cross-turn leakage')
