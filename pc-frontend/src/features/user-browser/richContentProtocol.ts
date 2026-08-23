export const YILONG_RICH_CONTENT_SCHEMA = 'yilong.rich-content.v1' as const

export interface RichContentPeriod {
  id: string
  label: string
  selected: boolean
}

export interface RichContentMetric {
  label: string
  value: string
}

export interface RichContentChartPoint {
  x: string
  y: number
}

export interface RichContentCandlestick {
  x: string
  open: number
  high: number
  low: number
  close: number
}

export type RichContentFinanceChart = {
  kind: 'line'
  points: RichContentChartPoint[]
} | {
  kind: 'candlestick'
  candles: RichContentCandlestick[]
}

export interface FinanceRichContentPayload {
  title: string
  symbol?: string
  primaryValue: string
  secondaryValue?: string
  trend: 'positive' | 'negative' | 'neutral'
  periods?: RichContentPeriod[]
  metrics?: RichContentMetric[]
  chart?: RichContentFinanceChart
}

export interface FinanceRichContent {
  schema: typeof YILONG_RICH_CONTENT_SCHEMA
  kind: 'finance'
  source: 'official_dom' | 'private_response' | 'cache'
  payload: FinanceRichContentPayload
}

export interface WeatherRichContentRow {
  period: string
  condition: string
  temperature: string
  precipitation?: string
  wind?: string
}

export interface WeatherRichContent {
  schema: typeof YILONG_RICH_CONTENT_SCHEMA
  kind: 'weather'
  source: 'official_dom' | 'private_response' | 'cache'
  payload: {
    title: string
    summary?: string
    rows: WeatherRichContentRow[]
  }
}

export interface MediaGalleryRichContentItem {
  url: string
  alt: string
  mediaType?: string
  width?: number
  height?: number
  sourceUrl?: string
}

export interface MediaGalleryRichContent {
  schema: typeof YILONG_RICH_CONTENT_SCHEMA
  kind: 'media_gallery'
  source: 'official_dom' | 'private_response' | 'cache'
  payload: {
    title: string
    items: MediaGalleryRichContentItem[]
  }
}

export interface MapRichContent {
  schema: typeof YILONG_RICH_CONTENT_SCHEMA
  kind: 'map'
  source: 'official_dom' | 'private_response' | 'cache'
  payload: {
    title: string
    summary?: string
    places: string[]
  }
}

export type YilongRichContent =
  | FinanceRichContent
  | WeatherRichContent
  | MediaGalleryRichContent
  | MapRichContent

const RICH_CONTENT_SOURCES = new Set(['official_dom', 'private_response', 'cache'])
const IMAGE_MEDIA_TYPES = new Set([
  'image/*',
  'image/jpeg',
  'image/png',
  'image/gif',
  'image/webp',
  'image/avif',
  'image/svg+xml',
])

/**
 * Defense-in-depth for old DPAPI snapshots and upstream schema drift. Rust remains
 * the sanitizer and authorization boundary; this guard only prevents malformed
 * cached values from reaching React components as trusted TypeScript objects.
 */
export function isYilongRichContent(value: unknown): value is YilongRichContent {
  if (!isRecord(value)
      || value.schema !== YILONG_RICH_CONTENT_SCHEMA
      || !RICH_CONTENT_SOURCES.has(value.source as string)
      || !isRecord(value.payload)) return false

  if (value.kind === 'finance') return isFinancePayload(value.payload)
  if (value.kind === 'weather') return isWeatherPayload(value.payload)
  if (value.kind === 'media_gallery') return isMediaGalleryPayload(value.payload)
  if (value.kind === 'map') return isMapPayload(value.payload)
  return false
}

function isFinancePayload(payload: Record<string, unknown>) {
  if (!boundedText(payload.title, 120)
      || !boundedText(payload.primaryValue, 64)
      || !['positive', 'negative', 'neutral'].includes(String(payload.trend))) return false
  if (!optionalText(payload.symbol, 24) || !optionalText(payload.secondaryValue, 96)) return false
  if (!optionalArray(payload.periods, 12, (value) => (
    isRecord(value)
      && boundedText(value.id, 16)
      && boundedText(value.label, 16)
      && typeof value.selected === 'boolean'
  ))) return false
  if (!optionalArray(payload.metrics, 16, (value) => (
    isRecord(value) && boundedText(value.label, 64) && boundedText(value.value, 96)
  ))) return false
  if (payload.chart === undefined) return true
  if (!isRecord(payload.chart)) return false
  if (payload.chart.kind === 'line') {
    if (!Array.isArray(payload.chart.points) || payload.chart.points.length < 2) return false
    return requiredArray(payload.chart.points, 512, (value) => (
      isRecord(value)
        && boundedText(value.x, 64)
        && finiteNumber(value.y)
    ))
  }
  if (payload.chart.kind === 'candlestick') {
    if (!Array.isArray(payload.chart.candles) || payload.chart.candles.length < 2) return false
    return requiredArray(payload.chart.candles, 512, (value) => {
      if (!isRecord(value)
          || !boundedText(value.x, 64)
          || !finiteNumber(value.open)
          || !finiteNumber(value.high)
          || !finiteNumber(value.low)
          || !finiteNumber(value.close)) return false
      return value.high >= Math.max(value.open, value.close)
        && value.low <= Math.min(value.open, value.close)
    })
  }
  return false
}

function isWeatherPayload(payload: Record<string, unknown>) {
  return boundedText(payload.title, 120)
    && optionalText(payload.summary, 240)
    && requiredArray(payload.rows, 24, (value) => (
      isRecord(value)
        && boundedText(value.period, 48)
        && boundedText(value.condition, 64)
        && boundedText(value.temperature, 32)
        && optionalText(value.precipitation, 32)
        && optionalText(value.wind, 48)
    ))
}

function isMediaGalleryPayload(payload: Record<string, unknown>) {
  return boundedText(payload.title, 120)
    && requiredArray(payload.items, 8, (value) => (
      isRecord(value)
        && publicHttpsUrl(value.url)
        && boundedText(value.alt, 180)
        && (value.mediaType === undefined || IMAGE_MEDIA_TYPES.has(value.mediaType as string))
        && optionalDimension(value.width)
        && optionalDimension(value.height)
        && (value.sourceUrl === undefined || publicHttpsUrl(value.sourceUrl))
    ))
}

function isMapPayload(payload: Record<string, unknown>) {
  if (!boundedText(payload.title, 120)
      || !optionalText(payload.summary, 500)
      || !Array.isArray(payload.places)
      || payload.places.length > 12
      || !payload.places.every((value) => boundedText(value, 120))) return false
  return boundedText(payload.summary, 500) || payload.places.length > 0
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === 'object' && !Array.isArray(value))
}

function boundedText(value: unknown, maximum: number) {
  return typeof value === 'string' && value.trim().length > 0 && [...value].length <= maximum
}

function optionalText(value: unknown, maximum: number) {
  return value === undefined || boundedText(value, maximum)
}

function requiredArray(
  value: unknown,
  maximum: number,
  predicate: (item: unknown) => boolean,
) {
  return Array.isArray(value)
    && value.length > 0
    && value.length <= maximum
    && value.every(predicate)
}

function optionalArray(
  value: unknown,
  maximum: number,
  predicate: (item: unknown) => boolean,
) {
  return value === undefined
    || (Array.isArray(value) && value.length <= maximum && value.every(predicate))
}

function optionalDimension(value: unknown) {
  return value === undefined
    || (typeof value === 'number' && Number.isInteger(value) && value > 0 && value <= 8_192)
}

function finiteNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value)
}

function publicHttpsUrl(value: unknown) {
  if (typeof value !== 'string' || value.length > 1_200) return false
  try {
    const url = new URL(value)
    return url.protocol === 'https:'
      && !url.username
      && !url.password
      && (!url.port || url.port === '443')
  } catch {
    return false
  }
}
