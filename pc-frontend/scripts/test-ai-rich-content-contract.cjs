const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')

const root = path.resolve(__dirname, '..', '..')
const read = (relativePath) => fs.readFileSync(path.join(root, relativePath), 'utf8')

const adapter = read('desktop-shell/src-tauri/src/local_ai_browser/chatgpt_rich_content_adapter.js')
const messages = read('android/app/src/main/assets/chatgpt_web_adapter_messages.js')
const bootstrap = read('desktop-shell/src-tauri/src/local_ai_browser/chatgpt_adapter_bootstrap.rs')
const sanitizer = read('desktop-shell/src-tauri/src/local_ai_browser/adapter_content/rich_content.rs')
const protocol = read('pc-frontend/src/features/user-browser/richContentProtocol.ts')
const localProtocol = read('pc-frontend/src/features/user-browser/localAiBrowserProtocol.ts')
const backend = read('pc-frontend/src/features/user-browser/useAiWebChatBackend.ts')
const renderer = read('pc-frontend/src/features/ai/AiRichContentCard.tsx')
const structuredContent = read('pc-frontend/src/features/ai/AiStructuredContent.tsx')
const fixture = JSON.parse(read('scripts/fixtures/chatgpt-rich-content-finance.json'))

assert.match(adapter, /yilong\.rich-content\.v1/)
assert.match(adapter, /function financeRoots\(content\)/)
assert.match(adapter, /role="radiogroup"/)
assert.match(adapter, /role="application"/)
assert.match(adapter, /function normalizeFinancePayload\(value\)/)
assert.match(adapter, /data-elon-rich-content-root/)
assert.match(adapter, /source: 'official_dom'/)
assert.match(messages, /__elonChatGptRichContent/)
assert.match(messages, /richContent\.parts\(content\)/)
assert.match(messages, /richContent\.owns\(node\)/)
assert.match(bootstrap, /chatgpt_rich_content_adapter\.js/)
assert.match(bootstrap, /WIN_RICH_CONTENT_ADAPTER/)
assert.match(sanitizer, /sanitize_rich_card/)
assert.match(localProtocol, /'rich_card'/)
assert.match(protocol, /YILONG_RICH_CONTENT_SCHEMA/)
assert.match(backend, /richContent:/)
assert.match(structuredContent, /<AiRichContentCard/)
assert.match(renderer, /aria-label="官方行情卡片"/)
assert.match(renderer, /periods\.map/)
assert.match(renderer, /metrics\.map/)
assert.match(renderer, /chart\?\.points/)

const context = {
  window: {},
  location: { origin: 'https://chatgpt.com' },
  URL,
  console,
}
vm.runInNewContext(adapter, context, { filename: 'chatgpt_rich_content_adapter.js' })
const normalized = context.window.__elonChatGptRichContent.normalizeFinancePayload(fixture)
assert.equal(normalized.title, 'Bitcoin (BTC)')
assert.equal(normalized.symbol, 'BTC')
assert.equal(normalized.primaryValue, 'US$77,274.00')
assert.equal(normalized.trend, 'positive')
assert.equal(normalized.periods.length, 8)
assert.equal(normalized.periods.filter((period) => period.selected).length, 1)
assert.equal(normalized.metrics.length, 4)
assert.equal(normalized.chart, undefined, 'DOM extraction must not fabricate unavailable chart points')

console.log('PASS: Win rich-content AST preserves observed finance-card semantics without fake chart data')
