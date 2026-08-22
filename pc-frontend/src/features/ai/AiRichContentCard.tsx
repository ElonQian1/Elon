import { Activity, CloudSun, ExternalLink } from 'lucide-react'
import type {
  FinanceRichContent,
  RichContentChartPoint,
  WeatherRichContent,
  YilongRichContent,
} from '../user-browser/richContentProtocol'
import styles from './AiRichContentCard.module.css'

export default function AiRichContentCard({ content }: { content: YilongRichContent }) {
  if (content.kind === 'weather') return <WeatherCard content={content} />
  return <FinanceCard content={content} />
}

function FinanceCard({ content }: { content: FinanceRichContent }) {
  const { payload } = content
  const periods = payload.periods ?? []
  const metrics = payload.metrics ?? []
  const path = chartPath(payload.chart?.points)
  return (
    <article className={styles.card} aria-label="官方行情卡片">
      <header>
        <span className={styles.providerIcon}><Activity size={18} aria-hidden="true" /></span>
        <div>
          <span className={styles.eyebrow}>市场行情</span>
          <h3>{payload.title}</h3>
        </div>
      </header>
      <div className={styles.quote}>
        <strong>{payload.primaryValue}</strong>
        {payload.secondaryValue && (
          <span data-trend={payload.trend}>{payload.secondaryValue}</span>
        )}
      </div>
      {periods.length > 0 && (
        <div className={styles.periods} aria-label="行情周期">
          {periods.map((period) => (
            <span key={period.id} aria-current={period.selected ? 'true' : undefined}>
              {period.label}
            </span>
          ))}
        </div>
      )}
      {path ? (
        <svg className={styles.chart} viewBox="0 0 640 180" role="img" aria-label="缓存行情走势">
          <defs>
            <linearGradient id="elon-finance-chart-fill" x1="0" x2="0" y1="0" y2="1">
              <stop offset="0" stopColor="currentColor" stopOpacity=".28" />
              <stop offset="1" stopColor="currentColor" stopOpacity="0" />
            </linearGradient>
          </defs>
          <path className={styles.area} d={`${path} L 640 180 L 0 180 Z`} />
          <path className={styles.line} d={path} />
        </svg>
      ) : (
        <div className={styles.chartFallback}>
          <ExternalLink size={15} aria-hidden="true" />
          <span>实时走势图在官网回答中保留完整交互；缓存不伪造缺失数据</span>
        </div>
      )}
      {metrics.length > 0 && (
        <dl className={styles.metrics}>
          {metrics.map((metric) => (
            <div key={`${metric.label}:${metric.value}`}>
              <dt>{metric.label}</dt>
              <dd>{metric.value}</dd>
            </div>
          ))}
        </dl>
      )}
    </article>
  )
}

function WeatherCard({ content }: { content: WeatherRichContent }) {
  const { payload } = content
  return (
    <article className={styles.card} aria-label="官方天气卡片">
      <header>
        <span className={[styles.providerIcon, styles.weatherIcon].join(' ')}>
          <CloudSun size={19} aria-hidden="true" />
        </span>
        <div>
          <span className={styles.eyebrow}>天气预报</span>
          <h3>{payload.title}</h3>
        </div>
      </header>
      {payload.summary && <p className={styles.summary}>{payload.summary}</p>}
      <div className={styles.weatherRows} role="table" aria-label="逐时天气">
        {payload.rows.map((row) => (
          <div className={styles.weatherRow} role="row" key={`${row.period}:${row.condition}`}>
            <strong role="cell">{row.period}</strong>
            <span role="cell">{weatherGlyph(row.condition)} {row.condition}</span>
            <b role="cell">{row.temperature}</b>
            {(row.precipitation || row.wind) && (
              <small role="cell">{[row.precipitation, row.wind].filter(Boolean).join(' · ')}</small>
            )}
          </div>
        ))}
      </div>
    </article>
  )
}

function weatherGlyph(condition: string) {
  if (/雷/.test(condition)) return '⛈️'
  if (/雨|阵雨|陣雨/.test(condition)) return '🌧️'
  if (/雪/.test(condition)) return '🌨️'
  if (/晴/.test(condition)) return '☀️'
  if (/阴|陰/.test(condition)) return '☁️'
  return '🌤️'
}

function chartPath(points: RichContentChartPoint[] | undefined) {
  if (!Array.isArray(points) || points.length < 2) return ''
  const values = points.map((point) => point.y).filter(Number.isFinite)
  if (values.length < 2) return ''
  const minimum = Math.min(...values)
  const maximum = Math.max(...values)
  const range = maximum - minimum || 1
  return points.map((point, index) => {
    const x = (index / (points.length - 1)) * 640
    const y = 166 - ((point.y - minimum) / range) * 146
    return `${index ? 'L' : 'M'} ${x.toFixed(2)} ${y.toFixed(2)}`
  }).join(' ')
}
