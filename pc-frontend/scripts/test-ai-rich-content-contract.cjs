const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')

const root = path.resolve(__dirname, '..', '..')
const read = (relativePath) => fs.readFileSync(path.join(root, relativePath), 'utf8')

const adapter = read('desktop-shell/src-tauri/src/local_ai_browser/chatgpt_rich_content_adapter.js')
const commonAdapter = read('desktop-shell/src-tauri/src/local_ai_browser/rich_content_dom_adapter.js')
const googleAdapter = read('desktop-shell/src-tauri/src/local_ai_browser/google_rich_content_adapter.js')
const citationAdapter = read('desktop-shell/src-tauri/src/local_ai_browser/chatgpt_citation_adapter.js')
const messages = read('android/app/src/main/assets/chatgpt_web_adapter_messages.js')
const bootstrap = read('desktop-shell/src-tauri/src/local_ai_browser/chatgpt_adapter_bootstrap.rs')
const googleBootstrap = read('desktop-shell/src-tauri/src/local_ai_browser/google_ai_mode.rs')
const sanitizer = read('desktop-shell/src-tauri/src/local_ai_browser/adapter_content/rich_content.rs')
const authorization = read('desktop-shell/src-tauri/src/local_ai_browser/private_response_authorization.rs')
const authorizationRegistry = JSON.parse(read('desktop-shell/src-tauri/src/local_ai_browser/private_response_authorizations.v1.json'))
const protocol = read('pc-frontend/src/features/user-browser/richContentProtocol.ts')
const localProtocol = read('pc-frontend/src/features/user-browser/localAiBrowserProtocol.ts')
const backend = read('pc-frontend/src/features/user-browser/useAiWebChatBackend.ts')
const renderer = read('pc-frontend/src/features/ai/AiRichContentCard.tsx')
const structuredContent = read('pc-frontend/src/features/ai/AiStructuredContent.tsx')
const fixture = JSON.parse(read('scripts/fixtures/chatgpt-rich-content-finance.json'))
const mediaMapFixture = JSON.parse(read('scripts/fixtures/rich-content-media-map.json'))
const authorizedFixture = JSON.parse(read('scripts/fixtures/rich-content-authorized-envelope.json'))

assert.match(adapter, /yilong\.rich-content\.v1/)
assert.match(adapter, /function financeRoots\(content\)/)
assert.match(adapter, /role="radiogroup"/)
assert.match(adapter, /role="application"/)
assert.match(adapter, /function normalizeFinancePayload\(value\)/)
assert.match(adapter, /data-elon-rich-content-root/)
assert.match(adapter, /source: 'official_dom'/)
assert.match(adapter, /function fromAuthorizedEnvelope\(envelope, authorize\)/)
assert.match(adapter, /kind: 'finance'[\s\S]*source: 'private_response'/)
assert.match(commonAdapter, /function normalizeMediaGalleryPayload\(value\)/)
assert.match(commonAdapter, /function normalizeMapPayload\(value\)/)
assert.match(commonAdapter, /yilong\.authorized-provider-response\.v1/)
assert.match(commonAdapter, /richPart\(kind, payload, 'private_response'\)/)
assert.match(googleAdapter, /prose\.concat\(rich\)/)
assert.match(googleAdapter, /function normalizeWeatherPayload\(value\)/)
assert.match(googleAdapter, /kind: 'weather'[\s\S]*source: 'private_response'/)
assert.match(citationAdapter, /aria-controls/)
assert.match(citationAdapter, /aria-describedby/)
assert.match(citationAdapter, /node\.querySelector\('img'\)/)
assert.match(citationAdapter, /literalCount\(markdown, marker\) !== 1/)
assert.doesNotMatch(citationAdapter, /Reuters|Barron|MarketWatch/, 'citation association must not guess publishers from visible text')
assert.match(messages, /__elonChatGptRichContent/)
assert.match(messages, /richContent\.parts\(content\)/)
assert.match(messages, /richContent\.owns\(node\)/)
assert.match(bootstrap, /chatgpt_rich_content_adapter\.js/)
assert.match(bootstrap, /WIN_RICH_CONTENT_ADAPTER/)
assert.match(bootstrap, /WIN_COMMON_RICH_CONTENT_ADAPTER/)
assert.match(bootstrap, /WIN_CITATION_ADAPTER/)
assert.match(googleBootstrap, /COMMON_RICH_CONTENT_SOURCE/)
assert.match(googleBootstrap, /WIN_RICH_CONTENT_SOURCE/)
assert.match(sanitizer, /sanitize_rich_card/)
assert.match(sanitizer, /sanitize_media_gallery_payload/)
assert.match(sanitizer, /sanitize_map_payload/)
assert.match(authorization, /allows_rich_kind/)
assert.match(authorization, /sanitized_ast_only/)
assert.deepEqual(authorizationRegistry.authorizations, [], 'private responses must fail closed until a production authorization entry is registered')
assert.match(localProtocol, /'rich_card'/)
assert.match(protocol, /YILONG_RICH_CONTENT_SCHEMA/)
assert.match(backend, /richContent:/)
assert.match(structuredContent, /<AiRichContentCard/)
assert.match(renderer, /aria-label="官方行情卡片"/)
assert.match(renderer, /aria-label="官方天气卡片"/)
assert.match(renderer, /aria-label="官方回答图片"/)
assert.match(renderer, /aria-label="官方地图摘要"/)
assert.match(renderer, /referrerPolicy="no-referrer"/)
assert.match(renderer, /periods\.map/)
assert.match(renderer, /metrics\.map/)
assert.match(renderer, /chart\?\.points/)
assert.match(renderer, /payload\.rows\.map/)

const context = {
  window: {},
  location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/' },
  URL,
  console,
}
vm.runInNewContext(commonAdapter, context, { filename: 'rich_content_dom_adapter.js' })
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

const common = context.window.__elonRichContentDomAdapter
const media = common.normalizeMediaGalleryPayload(mediaMapFixture.mediaGallery)
assert.equal(media.title, '回答图片')
assert.equal(media.items.length, 1, 'duplicate and signed media URLs must be excluded from persistent AST')
assert.equal(media.items[0].alt, '市场走势图')
assert.equal(media.items[0].mediaType, 'image/png')
const map = common.normalizeMapPayload(mediaMapFixture.map)
assert.equal(map.places.length, 3)
assert.equal(map.summary, '官网回答中可见的地点摘要')

const envelope = {
  schema: 'yilong.authorized-provider-response.v1',
  providerId: 'chatgpt',
  authorizationId: 'synthetic-test-only',
  parts: [
    { kind: 'media_gallery', payload: mediaMapFixture.mediaGallery },
    { kind: 'map', payload: mediaMapFixture.map },
  ],
}
assert.equal(common.fromAuthorizedEnvelope(envelope, () => false).length, 0)
const authorized = common.fromAuthorizedEnvelope(envelope, (_provider, _authorization, kind) => kind === 'map')
assert.equal(authorized.length, 1)
assert.equal(authorized[0].richContent.kind, 'map')
assert.equal(authorized[0].richContent.source, 'private_response')

const privateFinance = context.window.__elonChatGptRichContent.fromAuthorizedEnvelope(
  authorizedFixture.chatgptEnvelope,
  (_provider, _authorization, kind) => kind === 'finance',
)
assert.equal(privateFinance.length, 1)
assert.equal(privateFinance[0].richContent.kind, 'finance')
assert.equal(privateFinance[0].richContent.payload.chart.points.length, 2)
assert.equal(
  context.window.__elonChatGptRichContent.fromAuthorizedEnvelope(
    authorizedFixture.chatgptEnvelope,
    () => false,
  ).length,
  0,
  'authorized finance mapping must fail closed when the production grant callback denies it',
)

context.window.__elonChatGptMessages = Object.freeze({
  readMessageWindow: () => ({ messages: [] }),
  readMessages: () => [],
})
vm.runInNewContext(citationAdapter, context, { filename: 'chatgpt_citation_adapter.js' })
const citation = context.window.__elonChatGptCitationAdapter.normalizeCitationRecord(
  authorizedFixture.citation,
  0,
)
assert.equal(citation.url, 'https://www.reuters.com/technology/example-article')
assert.equal(citation.markerText, 'Reuters +2')
assert.equal(citation.groupSize, 3)
assert.equal(citation.citationId, 'citation_control_1')
const signedIconCitation = context.window.__elonChatGptCitationAdapter.normalizeCitationRecord({
  markerText: 'Reuters',
  url: 'https://www.reuters.com/technology/example-article?utm_source=chatgpt.com',
  iconUrl: 'https://cdn.example.com/icons/reuters.png?width=32&token=transient#fragment',
}, 1)
assert.equal(
  signedIconCitation.iconUrl,
  'https://cdn.example.com/icons/reuters.png',
  'visible official icons keep only a public HTTPS path before entering persistent AST',
)

const googleContext = {
  window: {
    __elonGoogleWebRichContent: Object.freeze({ version: 2, parts: () => [], owns: () => false }),
  },
  location: { origin: 'https://www.google.com', href: 'https://www.google.com/aimode' },
  URL,
  console,
}
vm.runInNewContext(commonAdapter, googleContext, { filename: 'rich_content_dom_adapter.google.js' })
vm.runInNewContext(googleAdapter, googleContext, { filename: 'google_rich_content_adapter.js' })
const privateWeather = googleContext.window.__elonGoogleWebRichContent.fromAuthorizedEnvelope(
  authorizedFixture.googleEnvelope,
  (_provider, _authorization, kind) => kind === 'weather',
)
assert.equal(privateWeather.length, 1)
assert.equal(privateWeather[0].richContent.kind, 'weather')
assert.equal(privateWeather[0].richContent.payload.rows[0].temperature, '23°C')
assert.equal(
  googleContext.window.__elonGoogleWebRichContent.fromAuthorizedEnvelope(
    authorizedFixture.googleEnvelope,
    () => false,
  ).length,
  0,
  'authorized weather mapping must fail closed when the production grant callback denies it',
)

console.log('PASS: Win rich-content AST preserves citations, finance, weather, media, and map with fail-closed private-response authorization')
