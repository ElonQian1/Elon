import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { ChevronRight, Database, RefreshCw, ShieldCheck } from 'lucide-react'
import { formatBytes, formatDateTime } from './nodeHelpers'
import {
  fetchNodeCacheFleet,
  summarizeNodeCacheFleet,
  type NodeCacheFleetItem,
} from './nodeCacheFleet'
import { formatCacheReportAge } from './nodeCacheHealth'
import type { NodeSummary } from './types'
import styles from './NodeCacheFleetOverview.module.css'

type FleetState =
  | { status: 'loading' }
  | { status: 'ready'; items: NodeCacheFleetItem[]; loadedAt: Date }
  | { status: 'error' }

export default function NodeCacheFleetOverview({
  nodes,
  onOpenNode,
}: {
  nodes: NodeSummary[]
  onOpenNode: (nodeId: string) => void
}) {
  const [state, setState] = useState<FleetState>({ status: 'loading' })
  const [refreshing, setRefreshing] = useState(false)
  const requestId = useRef(0)

  const load = useCallback(async (manual = false) => {
    const currentRequest = ++requestId.current
    if (manual) setRefreshing(true)
    else setState({ status: 'loading' })
    try {
      const items = await fetchNodeCacheFleet(nodes)
      if (requestId.current === currentRequest) {
        setState({ status: 'ready', items, loadedAt: new Date() })
      }
    } catch {
      if (requestId.current === currentRequest) setState({ status: 'error' })
    } finally {
      if (requestId.current === currentRequest) setRefreshing(false)
    }
  }, [nodes])

  useEffect(() => {
    load()
    return () => { requestId.current += 1 }
  }, [load])

  const summary = useMemo(
    () => summarizeNodeCacheFleet(state.status === 'ready' ? state.items : []),
    [state],
  )

  return (
    <section className={styles.page} aria-labelledby="cache-fleet-title">
      <header className={styles.header}>
        <div>
          <div className={styles.kicker}>我的节点</div>
          <h1 id="cache-fleet-title">缓存总览</h1>
        </div>
        <div className={styles.actions}>
          <span className={styles.readOnly}><ShieldCheck size={14} aria-hidden="true" />只读监控</span>
          <button
            className={styles.refresh}
            type="button"
            onClick={() => load(true)}
            disabled={refreshing || state.status === 'loading'}
            title="刷新全部节点缓存状态"
            aria-label="刷新全部节点缓存状态"
          >
            <RefreshCw size={16} className={refreshing ? styles.spinning : ''} aria-hidden="true" />
          </button>
        </div>
      </header>

      {state.status === 'ready' && state.items.length > 0 && (
        <div className={styles.summary} aria-label="缓存节点汇总">
          <SummaryMetric label="节点" value={summary.total} />
          <SummaryMetric label="健康" value={summary.healthy} tone="healthy" />
          <SummaryMetric label="需关注" value={summary.needsAttention + summary.failed} tone="attention" />
          <SummaryMetric label="未上报" value={summary.missing} />
        </div>
      )}

      {state.status === 'loading' && <p className={styles.message}>正在读取节点缓存状态...</p>}
      {state.status === 'error' && (
        <div className={styles.error} role="alert">缓存总览读取失败，请稍后刷新。</div>
      )}
      {state.status === 'ready' && state.items.length === 0 && (
        <div className={styles.empty}>
          <Database size={22} aria-hidden="true" />
          <strong>暂无已注册节点</strong>
        </div>
      )}
      {state.status === 'ready' && state.items.length > 0 && (
        <div className={styles.list} aria-live="polite">
          {state.items.map((item) => (
            <FleetNodeRow key={item.nodeId} item={item} onOpen={() => onOpenNode(item.nodeId)} />
          ))}
        </div>
      )}

      {state.status === 'ready' && (
        <footer className={styles.footer}>
          <span>更新于 {formatDateTime(state.loadedAt.toISOString())}</span>
          <span>GC 必须在节点本机重新预演、加锁和确认。</span>
        </footer>
      )}
    </section>
  )
}

function SummaryMetric({
  label,
  value,
  tone = 'neutral',
}: {
  label: string
  value: number
  tone?: 'neutral' | 'healthy' | 'attention'
}) {
  return <div data-tone={tone}><span>{label}</span><strong>{value}</strong></div>
}

function FleetNodeRow({ item, onOpen }: { item: NodeCacheFleetItem; onOpen: () => void }) {
  const presentation = fleetPresentation(item)
  return (
    <button className={styles.row} type="button" onClick={onOpen}>
      <span className={styles.nodeIdentity}>
        <span className={styles.nodeName}>{item.name}</span>
        <span className={styles.nodeMeta}>{item.node.online ? '在线' : '离线'} · {item.nodeId}</span>
      </span>
      <span className={styles.health} data-tone={presentation.tone}>
        <strong>{presentation.label}</strong>
        <small>{presentation.detail}</small>
      </span>
      <span className={styles.metrics}>{fleetMetrics(item)}</span>
      <ChevronRight className={styles.chevron} size={17} aria-hidden="true" />
    </button>
  )
}

function fleetPresentation(item: NodeCacheFleetItem) {
  if (item.status === 'missing') {
    return { tone: 'neutral', label: '未上报', detail: '等待节点生成健康报告' } as const
  }
  if (item.status === 'error') {
    return { tone: 'critical', label: '读取失败', detail: '打开节点详情复核' } as const
  }
  return { tone: item.health.tone, label: item.health.label, detail: item.health.summary }
}

function fleetMetrics(item: NodeCacheFleetItem): string {
  if (item.status !== 'ready') return '暂无缓存指标'
  const freePercent = item.report.report.volume?.free_percent
  const free = Number.isFinite(freePercent) ? `${Number(freePercent).toFixed(1)}% 可用` : '磁盘未上报'
  const managed = item.report.managed_size_bytes == null
    ? '缓存未计算'
    : formatBytes(item.report.managed_size_bytes)
  return `${managed} · ${free} · ${item.report.active_writer_count} 个活动编译 · ${formatCacheReportAge(item.health.ageMs)}`
}
