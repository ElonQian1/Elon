import { useCallback, useEffect, useRef, useState } from 'react'
import { ChevronDown, CircleCheck, RefreshCw, ShieldAlert } from 'lucide-react'
import {
  computeSupplyApi,
  type CapacityLedgerHistoryTransaction,
  type CapacityPoolAuditReport,
} from './computeSupplyApi'
import styles from './CapacityLedgerPanel.module.css'

interface Props {
  providerId: string
  poolId: string
  refreshKey: string
}

export default function CapacityLedgerPanel({ providerId, poolId, refreshKey }: Props) {
  const [audit, setAudit] = useState<CapacityPoolAuditReport | null>(null)
  const [transactions, setTransactions] = useState<CapacityLedgerHistoryTransaction[]>([])
  const [cursor, setCursor] = useState<number | null>(null)
  const [loading, setLoading] = useState(false)
  const [loadingMore, setLoadingMore] = useState(false)
  const [error, setError] = useState('')
  const requestVersion = useRef(0)

  const load = useCallback(async () => {
    const version = ++requestVersion.current
    setLoading(true); setLoadingMore(false); setError('')
    try {
      const [nextAudit, page] = await Promise.all([
        computeSupplyApi.auditPool(providerId, poolId),
        computeSupplyApi.ledgerHistory(providerId, poolId),
      ])
      if (version !== requestVersion.current) return
      setAudit(nextAudit); setTransactions(page.transactions); setCursor(page.next_before_sequence)
    } catch (reason) {
      if (version === requestVersion.current) setError(messageOf(reason, '容量账本审计读取失败'))
    } finally {
      if (version === requestVersion.current) setLoading(false)
    }
  }, [poolId, providerId])

  useEffect(() => { void load() }, [load, refreshKey])

  async function loadMore() {
    if (cursor === null || loadingMore) return
    const version = requestVersion.current
    setLoadingMore(true); setError('')
    try {
      const page = await computeSupplyApi.ledgerHistory(providerId, poolId, cursor)
      if (version !== requestVersion.current) return
      setTransactions((current) => [...current, ...page.transactions])
      setCursor(page.next_before_sequence)
    } catch (reason) {
      if (version === requestVersion.current) setError(messageOf(reason, '更多账本记录读取失败'))
    } finally {
      if (version === requestVersion.current) setLoadingMore(false)
    }
  }

  return <section className={styles.panel}>
    <header className={styles.header}>
      <div><h3>容量账本审计</h3><span>不可变事务与双分录</span></div>
      <button type="button" onClick={() => void load()} disabled={loading} aria-label="刷新账本审计" title="刷新账本审计"><RefreshCw size={15} className={loading ? styles.spinning : ''} /></button>
    </header>
    {error && <div className={styles.error}>{error}</div>}
    {audit && <div className={styles.audit}>
      <div className={audit.healthy ? styles.healthy : styles.unhealthy}>{audit.healthy ? <CircleCheck size={16} /> : <ShieldAlert size={16} />}<span><strong>{audit.healthy ? '内部账本一致' : '发现账本差异'}</strong><small>{formatTime(audit.checked_at)}</small></span></div>
      <div className={styles.auditFacts}><span><small>事务</small><strong>{audit.transaction_count}</strong></span><span><small>分录</small><strong>{audit.ledger_leg_count}</strong></span><span><small>Bucket</small><strong>{audit.buckets.length}</strong></span><span><small>Epoch</small><strong>{audit.capacity_epoch}</strong></span></div>
      {(audit.issues.length > 0 || audit.buckets.some((bucket) => bucket.issues.length > 0)) && <div className={styles.issues}>{audit.issues.map((issue) => <span key={issue}>{issue}</span>)}{audit.buckets.flatMap((bucket) => bucket.issues.map((issue) => <span key={`${bucket.bucket_id}-${issue}`}>{shortId(bucket.bucket_id)}：{issue}</span>))}</div>}
    </div>}
    <div className={styles.boundary}>健康状态只说明内部账本、守恒关系和余额投影一致，不证明硬件存在、节点在线、容量可交易或已经产生收益。</div>
    <div className={styles.transactions}>
      {transactions.map((transaction) => <TransactionRow transaction={transaction} key={transaction.transaction_id} />)}
      {!loading && transactions.length === 0 && <div className={styles.empty}>当前 Pool 尚无容量账本事务</div>}
    </div>
    {cursor !== null && <button type="button" className={styles.more} onClick={() => void loadMore()} disabled={loadingMore}>{loadingMore ? '正在读取' : '加载更早记录'}<ChevronDown size={14} /></button>}
  </section>
}

function TransactionRow({ transaction }: { transaction: CapacityLedgerHistoryTransaction }) {
  return <details className={styles.transaction}>
    <summary><span className={styles.sequence}>#{transaction.ledger_sequence}</span><span className={styles.event}>{eventLabel(transaction.event_kind)}</span><span className={styles.window}>{shortId(transaction.delivery_window_id)}</span><time>{formatTime(transaction.occurred_at)}</time><ChevronDown size={14} /></summary>
    <div className={styles.legs}>{transaction.legs.map((leg) => <div className={styles.leg} key={`${leg.line_no}-${leg.leg_role}-${leg.account}`}><span>{leg.leg_role === 'from' ? '转出' : '转入'}</span><strong>{accountLabel(leg.account)}</strong><code>{signed(leg.delta_units)}</code><small>{leg.meter} · {shortId(leg.bucket_id)}</small></div>)}</div>
    <div className={styles.digest}><span>事务摘要</span><code>{transaction.transaction_digest}</code></div>
  </details>
}

function eventLabel(value: string) { return ({ supply_added: '追加供给', supply_withdrawn: '撤出供给', reservation_held: '容量持有', attempt_activated: '任务激活', attempt_returned: '容量归还', usage_consumed: '用量消费', reservation_released: '预留释放', reservation_expired: '预留过期' } as Record<string, string>)[value] ?? value }
function accountLabel(value: string) { return ({ issuance: '发行源', available: '可用', held: '持有', active: '运行中', consumed: '已消费', retired: '已撤出' } as Record<string, string>)[value] ?? value }
function signed(value: number) { return value > 0 ? `+${value}` : String(value) }
function shortId(value: string) { return value.length <= 26 ? value : `${value.slice(0, 13)}…${value.slice(-8)}` }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
