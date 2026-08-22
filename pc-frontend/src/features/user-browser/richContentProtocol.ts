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

export interface FinanceRichContentPayload {
  title: string
  symbol?: string
  primaryValue: string
  secondaryValue?: string
  trend: 'positive' | 'negative' | 'neutral'
  periods?: RichContentPeriod[]
  metrics?: RichContentMetric[]
  chart?: {
    kind: 'line'
    points: RichContentChartPoint[]
  }
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
