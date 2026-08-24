const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')
const ts = require('typescript')

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
const structuredPolicy = read('pc-frontend/src/features/user-browser/localAiStructuredPartPolicy.ts')
const fixture = JSON.parse(read('scripts/fixtures/chatgpt-rich-content-finance.json'))
const mediaMapFixture = JSON.parse(read('scripts/fixtures/rich-content-media-map.json'))
const authorizedFixture = JSON.parse(read('scripts/fixtures/rich-content-authorized-envelope.json'))

assert.match(adapter, /yilong\.rich-content\.v1/)
assert.match(adapter, /function financeRoots\(content\)/)
assert.match(adapter, /svg path\[d\]/)
assert.match(adapter, /role="radiogroup"/)
assert.match(adapter, /role="application"/)
assert.match(adapter, /function normalizeFinancePayload\(value\)/)
assert.match(adapter, /function sampleChartGeometry\(geometry\)/)
assert.match(adapter, /function visibleCandlestickChart\(root\)/)
assert.match(adapter, /function financePayloadFromPairs\(pairs, title\)/)
assert.match(adapter, /function ohlcTableParts\(content\)/)
assert.match(adapter, /function visibleChart\(root\)/)
assert.match(adapter, /MAX_VISIBLE_CHART_POINTS = 96/)
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
assert.equal(authorizationRegistry.authorizations.length, 1, 'only reviewed production authorizations are registered')
assert.equal(authorizationRegistry.authorizations[0].providerId, 'chatgpt')
assert.deepEqual(authorizationRegistry.authorizations[0].dataClasses, ['rich_content.finance'])
assert.equal(authorizationRegistry.authorizations[0].persistence, 'sanitized_ast_only')
assert.equal(authorizationRegistry.authorizations[0].upload, 'none')
assert.equal(authorizationRegistry.authorizations[0].rawRetentionSeconds, 0)
assert.match(localProtocol, /'rich_card'/)
assert.match(protocol, /YILONG_RICH_CONTENT_SCHEMA/)
assert.match(protocol, /function isYilongRichContent\(value: unknown\)/)
assert.match(backend, /isYilongRichContent\(part\.richContent\)/)
assert.match(structuredContent, /isYilongRichContent\(part\.richContent\)/)
assert.match(structuredPolicy, /Boolean\(part\.richContent\)/)
assert.match(backend, /part\.type !== 'rich_card' \|\| Boolean\(part\.richContent\)/)
assert.match(backend, /richContent[,\s]/)
assert.match(structuredContent, /<AiRichContentCard/)
assert.match(structuredContent, /placement === 'primary'/)
assert.match(structuredContent, /isPrimaryRichCard/)
assert.match(renderer, /aria-label="官方行情卡片"/)
assert.match(renderer, /aria-label="官方天气卡片"/)
assert.match(renderer, /aria-label="官方回答图片"/)
assert.match(renderer, /aria-label="官方地图摘要"/)
assert.match(renderer, /referrerPolicy="no-referrer"/)
assert.match(renderer, /periods\.map/)
assert.match(renderer, /metrics\.map/)
assert.match(renderer, /payload\.chart\.points/)
assert.match(renderer, /payload\.chart\?\.kind === 'candlestick'/)
assert.match(renderer, /aria-label="缓存行情 K 线图"/)
assert.match(renderer, /useId\(\)/)
assert.match(renderer, /url\(#\$\{gradientId\}\)/)
assert.match(renderer, /payload\.rows\.map/)

const context = {
  window: { getComputedStyle: () => ({ display: 'block', visibility: 'visible' }) },
  location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/' },
  Element: Object,
  HTMLElement: Object,
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

const candlestickPayload = context.window.__elonChatGptRichContent.normalizeFinancePayload({
  title: 'Apple Inc. (AAPL)',
  primaryValue: 'US$231.59',
  trend: 'positive',
  chart: {
    kind: 'candlestick',
    candles: [
      { x: '2026-08-20', open: 228.1, high: 232.4, low: 227.7, close: 231.2 },
      { x: '2026-08-21', open: 231.2, high: 233.1, low: 229.8, close: 230.4 },
    ],
  },
})
assert.equal(candlestickPayload.chart.kind, 'candlestick')
assert.equal(candlestickPayload.chart.candles.length, 2)
assert.equal(candlestickPayload.chart.candles[0].close, 231.2)
const tableFinancePayload = context.window.__elonChatGptRichContent.financePayloadFromPairs([
  { label: '股票代码', value: 'AAPL（NASDAQ）' },
  { label: '最新价', value: '309.35 美元' },
  { label: '涨跌', value: '-1.88 美元（-0.60%）' },
  { label: '开盘（Open）', value: '312.15 美元' },
  { label: '最高（High）', value: '312.60 美元' },
  { label: '最低（Low）', value: '307.03 美元' },
], 'Apple Inc.（AAPL）最新行情')
assert.equal(tableFinancePayload.chart.kind, 'candlestick')
assert.equal(tableFinancePayload.chart.candles.length, 1)
assert.equal(
  JSON.stringify(tableFinancePayload.chart.candles[0]),
  JSON.stringify({ x: '最新交易日', open: 312.15, high: 312.6, low: 307.03, close: 309.35 }),
)
const pairRows = [
  ['股票代码', 'AAPL（NASDAQ）'],
  ['最新价', '309.35 美元'],
  ['涨跌', '-1.88 美元（-0.60%）'],
  ['开盘（Open）', '312.15 美元'],
  ['最高（High）', '312.60 美元'],
  ['最低（Low）', '307.03 美元'],
].map((values) => ({
  querySelectorAll: () => values.map((innerText) => ({ innerText, textContent: innerText })),
}))
let tableMarker = ''
const visibleTable = {
  isConnected: true,
  getBoundingClientRect: () => ({ width: 480, height: 240 }),
  closest: () => tableMarker ? visibleTable : null,
  setAttribute: (_name, value) => { tableMarker = value },
  querySelectorAll: (selector) => selector === 'tr' ? pairRows : [],
}
const visibleHeading = {
  isConnected: true,
  innerText: 'Apple Inc.（AAPL）最新行情',
  getBoundingClientRect: () => ({ width: 360, height: 32 }),
}
const visibleTableContent = {
  querySelectorAll(selector) {
    if (selector === 'table') return [visibleTable]
    if (selector.startsWith('h1')) return [visibleHeading]
    return []
  },
}
assert.equal(context.window.__elonChatGptRichContent.ohlcTableParts(visibleTableContent).length, 1)
assert.equal(
  context.window.__elonChatGptRichContent.ohlcTableParts(visibleTableContent).length,
  1,
  'a marked OHLC table must remain present in consecutive semantic snapshots',
)
const visibleCandlesticks = context.window.__elonChatGptRichContent.visibleCandlestickChart({
  querySelectorAll: () => [
    { getAttribute: () => '2026-08-20 Open 228.1 High 232.4 Low 227.7 Close 231.2', textContent: '' },
    { getAttribute: () => '2026-08-21 Open 231.2 High 233.1 Low 229.8 Close 230.4', textContent: '' },
  ],
})
assert.equal(visibleCandlesticks.kind, 'candlestick')
assert.equal(visibleCandlesticks.candles[1].low, 229.8)

const sampledChart = context.window.__elonChatGptRichContent.sampleChartGeometry({
  getBBox: () => ({ width: 100, height: 20 }),
  getBoundingClientRect: () => ({ width: 640, height: 128 }),
  getTotalLength: () => 100,
  getPointAtLength: (position) => ({ x: position, y: 10 + Math.sin(position / 8) * 9 }),
})
assert.equal(sampledChart.length, 96, 'visible SVG chart sampling must stay bounded')
assert.equal(sampledChart[0].x, '0')
assert.ok(Math.max(...sampledChart.map((point) => point.y)) > 15)
assert.equal(
  context.window.__elonChatGptRichContent.sampleChartGeometry({
    getBBox: () => ({ width: 640, height: 0 }),
    getTotalLength: () => 640,
    getPointAtLength: (position) => ({ x: position, y: 10 }),
  }).length,
  0,
  'horizontal grid lines must not become finance charts',
)
assert.equal(
  context.window.__elonChatGptRichContent.sampleChartGeometry({
    getBBox: () => ({ width: 640, height: 80 }),
    getTotalLength: () => 640,
    getPointAtLength: (position) => ({
      x: position <= 320 ? position * 2 : 640 - (position - 320) * 2,
      y: 40 + Math.sin(position / 40) * 35,
    }),
  }).length,
  0,
  'closed or strongly backtracking area paths must fail closed',
)
assert.equal(
  context.window.__elonChatGptRichContent.sampleChartGeometry({
    getBBox: () => { throw new Error('detached') },
  }).length,
  0,
  'detached SVG geometry must not break the message snapshot',
)

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

const protocolModule = { exports: {} }
const compiledProtocol = ts.transpileModule(protocol, {
  compilerOptions: {
    module: ts.ModuleKind.CommonJS,
    target: ts.ScriptTarget.ES2022,
  },
  fileName: 'richContentProtocol.ts',
}).outputText
new Function('exports', 'module', 'require', compiledProtocol)(
  protocolModule.exports,
  protocolModule,
  require,
)
const { isYilongRichContent } = protocolModule.exports

const validFinance = {
  schema: 'yilong.rich-content.v1',
  kind: 'finance',
  source: 'official_dom',
  payload: authorizedFixture.chatgptEnvelope.parts[0].payload,
}
const validCandlestickFinance = structuredClone(validFinance)
validCandlestickFinance.payload.chart = candlestickPayload.chart
const validSingleCandlestickFinance = structuredClone(validFinance)
validSingleCandlestickFinance.payload.chart = tableFinancePayload.chart
const validWeather = {
  schema: 'yilong.rich-content.v1',
  kind: 'weather',
  source: 'cache',
  payload: authorizedFixture.googleEnvelope.parts[0].payload,
}
const validMedia = {
  schema: 'yilong.rich-content.v1',
  kind: 'media_gallery',
  source: 'official_dom',
  payload: media,
}
const validMap = {
  schema: 'yilong.rich-content.v1',
  kind: 'map',
  source: 'cache',
  payload: map,
}
assert.equal(isYilongRichContent(validFinance), true)
assert.equal(isYilongRichContent(validCandlestickFinance), true)
assert.equal(isYilongRichContent(validSingleCandlestickFinance), true)
assert.equal(isYilongRichContent(validWeather), true)
assert.equal(isYilongRichContent(validMedia), true)
assert.equal(isYilongRichContent(validMap), true)
assert.equal(isYilongRichContent({ ...validFinance, schema: 'yilong.rich-content.v2' }), false)
assert.equal(isYilongRichContent({ ...validFinance, kind: 'unknown' }), false)
assert.equal(isYilongRichContent({ ...validFinance, source: 'raw_response' }), false)
assert.equal(isYilongRichContent({ ...validFinance, payload: { title: 'BTC' } }), false)

const invalidChart = structuredClone(validFinance)
invalidChart.payload.chart.points[0].y = Number.NaN
assert.equal(isYilongRichContent(invalidChart), false)
const invalidCandle = structuredClone(validCandlestickFinance)
invalidCandle.payload.chart.candles[0].high = 227
assert.equal(isYilongRichContent(invalidCandle), false, 'high must contain the candle body')
const oversizedMetrics = structuredClone(validFinance)
oversizedMetrics.payload.metrics = Array.from({ length: 17 }, (_, index) => ({ label: `L${index}`, value: '1' }))
assert.equal(isYilongRichContent(oversizedMetrics), false)
const httpMedia = structuredClone(validMedia)
httpMedia.payload.items[0].url = 'http://example.com/chart.png'
assert.equal(isYilongRichContent(httpMedia), false)
const credentialMedia = structuredClone(validMedia)
credentialMedia.payload.items[0].url = 'https://user:password@example.com/chart.png'
assert.equal(isYilongRichContent(credentialMedia), false)
assert.equal(isYilongRichContent({ ...validWeather, payload: { title: '天气', rows: [] } }), false)
assert.equal(isYilongRichContent({ ...validMap, payload: { title: '地图', places: [] } }), false)

console.log('PASS: Win rich-content AST preserves citations, finance, weather, media, and map with fail-closed private-response authorization')
