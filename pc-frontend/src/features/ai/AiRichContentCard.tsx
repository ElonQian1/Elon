import { Activity, CloudSun, ExternalLink, Images, MapPinned, Navigation } from 'lucide-react'
import { useId } from 'react'
import type {
  FinanceRichContent,
  MapRichContent,
  MediaGalleryRichContent,
  RichContentCandlestick,
  RichContentChartPoint,
  WeatherRichContent,
  YilongRichContent,
} from '../user-browser/richContentProtocol'
import styles from './AiRichContentCard.module.css'

export default function AiRichContentCard({ content }: { content: YilongRichContent }) {
  if (content.kind === 'weather') return <WeatherCard content={content} />
  if (content.kind === 'media_gallery') return <MediaGalleryCard content={content} />
  if (content.kind === 'map') return <MapCard content={content} />
  return <FinanceCard content={content} />
}

function FinanceCard({ content }: { content: FinanceRichContent }) {
  const { payload } = content
  const periods = payload.periods ?? []
  const metrics = payload.metrics ?? []
  const candles = payload.chart?.kind === 'candlestick' ? payload.chart.candles : undefined
  const path = chartPath(payload.chart?.kind === 'line' ? payload.chart.points : undefined)
  const gradientId = useId()
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
      {candles ? (
        <CandlestickChart candles={candles} />
      ) : path ? (
        <svg className={styles.chart} data-trend={payload.trend} viewBox="0 0 640 180" role="img" aria-label="缓存行情走势">
          <defs>
            <linearGradient id={gradientId} x1="0" x2="0" y1="0" y2="1">
              <stop offset="0" stopColor="currentColor" stopOpacity=".28" />
              <stop offset="1" stopColor="currentColor" stopOpacity="0" />
            </linearGradient>
          </defs>
          <path className={styles.area} d={`${path} L 640 180 L 0 180 Z`} style={{ fill: `url(#${gradientId})` }} />
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

function CandlestickChart({ candles }: { candles: RichContentCandlestick[] }) {
  const visibleCandles = candles.slice(-96)
  const values = visibleCandles.flatMap((candle) => [candle.high, candle.low])
  const minimum = Math.min(...values)
  const maximum = Math.max(...values)
  const range = maximum - minimum || 1
  const step = 640 / visibleCandles.length
  const bodyWidth = Math.max(2, Math.min(8, step * .58))
  const yFor = (value: number) => 166 - ((value - minimum) / range) * 146
  return (
    <svg className={styles.chart} viewBox="0 0 640 180" role="img" aria-label="缓存行情 K 线图">
      <title>{`K 线图，共 ${visibleCandles.length} 根，区间 ${formatChartValue(minimum)} 至 ${formatChartValue(maximum)}`}</title>
      <text className={styles.chartAxisLabel} x="4" y="13">{formatChartValue(maximum)}</text>
      <text className={styles.chartAxisLabel} x="4" y="177">{formatChartValue(minimum)}</text>
      {visibleCandles.map((candle, index) => {
        const x = (index + .5) * step
        const openY = yFor(candle.open)
        const closeY = yFor(candle.close)
        const highY = yFor(candle.high)
        const lowY = yFor(candle.low)
        const trendClass = candle.close > candle.open
          ? styles.candleUp
          : candle.close < candle.open ? styles.candleDown : styles.candleFlat
        return (
          <g className={trendClass} key={`${candle.x}:${index}`}>
            <title>{`${candle.x} 开 ${candle.open} 高 ${candle.high} 低 ${candle.low} 收 ${candle.close}`}</title>
            <line className={styles.candleWick} x1={x} x2={x} y1={highY} y2={lowY} />
            <rect
              className={styles.candleBody}
              x={x - bodyWidth / 2}
              y={Math.min(openY, closeY)}
              width={bodyWidth}
              height={Math.max(1.5, Math.abs(closeY - openY))}
              rx={Math.min(1.2, bodyWidth / 4)}
            />
          </g>
        )
      })}
    </svg>
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

function MediaGalleryCard({ content }: { content: MediaGalleryRichContent }) {
  const { payload } = content
  return (
    <article className={styles.card} aria-label="官方回答图片">
      <header>
        <span className={[styles.providerIcon, styles.mediaIcon].join(' ')}>
          <Images size={19} aria-hidden="true" />
        </span>
        <div>
          <span className={styles.eyebrow}>回答媒体</span>
          <h3>{payload.title}</h3>
        </div>
      </header>
      <div className={styles.gallery} data-count={Math.min(payload.items.length, 4)}>
        {payload.items.map((item, index) => {
          const image = (
            <img
              src={item.url}
              alt={item.alt}
              loading="lazy"
              decoding="async"
              referrerPolicy="no-referrer"
              width={item.width}
              height={item.height}
            />
          )
          return item.sourceUrl ? (
            <a
              href={item.sourceUrl}
              target="_blank"
              rel="noopener noreferrer"
              title={`${item.alt} · 打开来源`}
              key={`${item.url}:${index}`}
            >
              {image}
              <span><ExternalLink size={13} aria-hidden="true" />{item.alt}</span>
            </a>
          ) : (
            <figure key={`${item.url}:${index}`}>
              {image}
              <figcaption>{item.alt}</figcaption>
            </figure>
          )
        })}
      </div>
    </article>
  )
}

function MapCard({ content }: { content: MapRichContent }) {
  const { payload } = content
  return (
    <article className={styles.card} aria-label="官方地图摘要">
      <header>
        <span className={[styles.providerIcon, styles.mapIcon].join(' ')}>
          <MapPinned size={19} aria-hidden="true" />
        </span>
        <div>
          <span className={styles.eyebrow}>地图结果</span>
          <h3>{payload.title}</h3>
        </div>
      </header>
      {payload.summary && <p className={styles.summary}>{payload.summary}</p>}
      {payload.places.length > 0 && (
        <ul className={styles.places}>
          {payload.places.map((place, index) => (
            <li key={`${place}:${index}`}><MapPinned size={14} aria-hidden="true" />{place}</li>
          ))}
        </ul>
      )}
      <div className={styles.mapFallback}>
        <Navigation size={15} aria-hidden="true" />
        <span>交互地图在官网回答中保留；缓存仅保存可见地点摘要</span>
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

function formatChartValue(value: number) {
  return new Intl.NumberFormat('zh-CN', { maximumFractionDigits: 2 }).format(value)
}
