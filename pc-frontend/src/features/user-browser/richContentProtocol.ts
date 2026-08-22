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

export type YilongRichContent = FinanceRichContent
