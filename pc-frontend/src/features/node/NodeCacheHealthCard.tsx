import { useCallback, useEffect, useState } from 'react'
import { Database, RefreshCw, ShieldCheck, TriangleAlert } from 'lucide-react'
import { formatBytes, formatDateTime, nodeId } from './nodeHelpers'
import {
  deriveNodeCacheHealth,
  fetchNodeCacheHealth,
  formatCacheReportAge,
  type NodeCacheHealthResponse,
} from './nodeCacheHealth'
import type { NodeSummary } from './types'
import styles from './NodeCacheHealthCard.module.css'

type LoadState =
  | { status: 'loading' }
  | { status: 'empty' }
  | { status: 'ready'; report: NodeCacheHealthResponse }
  | { status: 'error'; message: string }

export default function NodeCacheHealthCard({ node }: { node: NodeSummary }) {
  const id = nodeId(node)
  const [state, setState] = useState<LoadState>({ status: 'loading' })
  const [refreshing, setRefreshing] = useState(false)

  const load = useCallback(async (manual = false) => {
    if (manual) setRefreshing(true)
    else setState({ status: 'loading' })
    try {
      const report = await fetchNodeCacheHealth(id)
      setState(report ? { status: 'ready', report } : { status: 'empty' })
    } catch (reason) {
      const message = typeof reason === 'object' && reason !== null && 'message' in reason
        ? String(reason.message)
        : '缓存健康状态读取失败'
      setState({ status: 'error', message })
    } finally {
      setRefreshing(false)
    }
  }, [id])

  useEffect(() => {
    let active = true
    fetchNodeCacheHealth(id)
      .then((report) => {
        if (active) setState(report ? { status: 'ready', report } : { status: 'empty' })
      })
      .catch((reason) => {
        if (!active) return
        const message = typeof reason === 'object' && reason !== null && 'message' in reason
          ? String(reason.message)
          : '缓存健康状态读取失败'
        setState({ status: 'error', message })
      })
    return () => { active = false }
  }, [id])

  return (
    <section className={styles.card} aria-labelledby={`cache-health-${id}`}>
      <header className={styles.header}>
        <div className={styles.title}>
          <Database size={16} aria-hidden="true" />
          <strong id={`cache-health-${id}`}>共享构建缓存</strong>
        </div>
        <div className={styles.headerActions}>
          <span className={styles.readOnly}><ShieldCheck size={13} aria-hidden="true" />只读监控</span>
          <button
            className={styles.refresh}
            type="button"
            onClick={() => load(true)}
            disabled={refreshing}
            title="刷新缓存健康状态"
            aria-label="刷新缓存健康状态"
          >
            <RefreshCw size={15} className={refreshing ? styles.spinning : ''} aria-hidden="true" />
          </button>
        </div>
      </header>
      {renderState(state)}
    </section>
  )
}

function renderState(state: LoadState) {
  if (state.status === 'loading') {
    return <p className={styles.message}>正在读取节点最近一次脱敏报告...</p>
  }
  if (state.status === 'empty') {
    return (
      <div className={styles.empty}>
        <span>尚未收到报告</span>
        <p>节点完成一次缓存健康排队后，这里会自动显示结果。</p>
      </div>
    )
  }
  if (state.status === 'error') {
    return (
      <div className={styles.error}>
        <TriangleAlert size={16} aria-hidden="true" />
        <span>{state.message}</span>
      </div>
    )
  }

  const { report } = state
  const view = deriveNodeCacheHealth(report)
  const cache = report.report.cache ?? {}
  const volume = report.report.volume ?? {}
  const checks = report.report.platform?.actionable_checks?.length ?? 0
  const metrics = [
    ['受管缓存', report.managed_size_bytes == null ? '未计算' : formatBytes(report.managed_size_bytes)],
    ['磁盘可用', formatPercent(volume.free_percent)],
    ['缓存分区', formatCount(cache.partition_count)],
    ['活动编译', `${report.active_writer_count} 个`],
  ]

  return (
    <>
      <div className={styles.summary} data-tone={view.tone}>
        <div>
          <strong>{view.label}</strong>
          <p>{view.summary}</p>
        </div>
        <span title={formatDateTime(report.generated_at)}>{formatCacheReportAge(view.ageMs)}</span>
      </div>
      <div className={styles.metrics}>
        {metrics.map(([label, value]) => (
          <div key={label}><span>{label}</span><strong>{value}</strong></div>
        ))}
      </div>
      <div className={styles.details}>
        <span>锁定 {cache.locked_partition_count ?? 0}</span>
        <span>隔离 {cache.quarantine_partition_count ?? 0}</span>
        <span>旧缓存 {cache.legacy_cache_count ?? 0}</span>
        <span>待复核项 {checks}</span>
        <span>服务端接收 {formatDateTime(report.received_at) || '未知'}</span>
      </div>
      <p className={styles.boundary}>报告不会授权远程删除；实际 GC 仍需对应电脑重新预演、加锁和确认。</p>
    </>
  )
}

function formatPercent(value: number | undefined): string {
  return Number.isFinite(value) ? `${Number(value).toFixed(1)}%` : '未上报'
}

function formatCount(value: number | undefined): string {
  return Number.isFinite(value) ? `${Math.max(0, Math.trunc(Number(value)))}` : '未上报'
}
