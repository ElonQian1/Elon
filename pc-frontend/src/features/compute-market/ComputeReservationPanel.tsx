import { useCallback, useEffect, useMemo, useState } from 'react'
import { CircleDollarSign, Clock3, LoaderCircle, RefreshCw, RotateCcw, ShieldCheck, TriangleAlert } from 'lucide-react'
import FinishReservationDialog from './FinishReservationDialog'
import ReserveComputeJobDialog from './ReserveComputeJobDialog'
import {
  computeMarketApi,
  type ComputeJobReceipt,
  type ComputeQuoteCandidate,
  type ComputeReservationReceipt,
  type ReserveComputeJobBody,
} from './computeMarketApi'
import styles from './ComputeReservationPanel.module.css'

interface Props {
  job: ComputeJobReceipt
  candidate: ComputeQuoteCandidate | null
  onJobChanged: (jobId: string, notice: string) => Promise<void>
}

export default function ComputeReservationPanel({ job, candidate, onJobChanged }: Props) {
  const [reservations, setReservations] = useState<ComputeReservationReceipt[]>([])
  const [reserveOpen, setReserveOpen] = useState(false)
  const [finishAction, setFinishAction] = useState<'release' | 'expire' | null>(null)
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const current = useMemo(() => reservations.find((item) => item.reservation.status === 'active') ?? reservations[0] ?? null, [reservations])
  const expired = current?.reservation.status === 'active' && new Date(current.reservation.expires_at).getTime() <= Date.now()

  const load = useCallback(async () => {
    setLoading(true); setError('')
    try {
      const all = await computeMarketApi.listReservations()
      setReservations(all.filter((item) => item.reservation.job.job_id === job.job.job_id).sort((left, right) => right.reservation.updated_at.localeCompare(left.reservation.updated_at)))
    } catch (reason) { setError(messageOf(reason, 'Reservation 读取失败')) } finally { setLoading(false) }
  }, [job.job.job_id])

  useEffect(() => { void load() }, [load])

  async function reserve(body: ReserveComputeJobBody) {
    if (busy) return
    setBusy(true); setError('')
    try {
      const receipt = await computeMarketApi.reserve(body)
      setReserveOpen(false)
      await load()
      await onJobChanged(receipt.reserved_job.job_id, `已冻结 CNY ${(receipt.budget_reserved_fen / 100).toFixed(2)} 并持有容量。`)
    } catch (reason) { setError(messageOf(reason, '预算与容量预留失败')) } finally { setBusy(false) }
  }

  async function finish(idempotencyKey: string) {
    if (!current || !finishAction || busy) return
    setBusy(true); setError('')
    try {
      const receipt = await computeMarketApi.finishReservation(current, finishAction, idempotencyKey)
      setFinishAction(null)
      await load()
      await onJobChanged(receipt.terminal_job.job_id, `预留已${receipt.action === 'release' ? '释放' : '到期'}，退回 CNY ${(receipt.budget_refunded_fen / 100).toFixed(2)}。`)
    } catch (reason) { setError(messageOf(reason, finishAction === 'release' ? '释放预留失败' : '确认到期失败')) } finally { setBusy(false) }
  }

  const canReserve = job.job.status === 'quoted' && candidate !== null && !current?.reservation.status.match(/^(active|pending)$/)

  return <section className={styles.panel}><header><div><h3>预算与容量预留</h3><span>{current ? `${statusLabel(current.reservation.status)} · revision ${current.revision}` : '尚无 Reservation'}</span></div><div><button type="button" onClick={() => void load()} disabled={loading} aria-label="刷新预留" title="刷新预留"><RefreshCw size={14} className={loading ? styles.spinning : ''} /></button><button type="button" className={styles.reserve} onClick={() => { setError(''); setReserveOpen(true) }} disabled={!canReserve} title={candidate ? '预留预算与容量' : '先发现已锁定报价的当前候选'}><CircleDollarSign size={14} />预留</button></div></header>{error && !reserveOpen && !finishAction && <div className={styles.error}><TriangleAlert size={14} />{error}</div>}{loading && !current && <div className={styles.empty}><LoaderCircle size={15} className={styles.spinning} />读取预留</div>}{!loading && !current && <div className={styles.empty}>{job.job.status === 'quoted' && !candidate ? '重新发现当前报价后即可预留' : '当前 Job 尚未持有预算与容量'}</div>}{current && <div className={styles.receipt}><div className={styles.receiptHeader}><div><ShieldCheck size={16} /><span><strong>{shortId(current.reservation.reservation_id)}</strong><small>{formatTime(current.reservation.expires_at)} 到期</small></span></div><b>{statusLabel(current.reservation.status)}</b></div><div className={styles.facts}><div><span>冻结上限</span><strong>{formatAmount(current.reservation.price_snapshot.consumer_max_amount_micros, current.reservation.price_snapshot.currency)}</strong></div><div><span>预算凭证</span><strong title={current.reservation.consumer_authorization_ref}>{shortId(current.reservation.consumer_authorization_ref)}</strong></div><div><span>容量凭证</span><strong title={current.reservation.capacity_claim.claim_id}>{shortId(current.reservation.capacity_claim.claim_id)}</strong></div></div><div className={styles.capacity}>{current.reservation.reserved_capacity.map((item) => <span key={item.meter}>{item.meter}: {item.quantity}</span>)}</div><code>{current.reservation_digest}</code>{current.reservation.status === 'active' && <footer>{expired ? <button type="button" data-tone="expire" onClick={() => { setError(''); setFinishAction('expire') }}><Clock3 size={14} />确认到期</button> : <button type="button" data-tone="release" onClick={() => { setError(''); setFinishAction('release') }}><RotateCcw size={14} />释放预留</button>}</footer>}</div>}{reserveOpen && candidate && <ReserveComputeJobDialog job={job} candidate={candidate} busy={busy} error={error} onClose={() => setReserveOpen(false)} onSubmit={reserve} />}{finishAction && current && <FinishReservationDialog receipt={current} action={finishAction} busy={busy} error={error} onClose={() => setFinishAction(null)} onSubmit={finish} />}</section>
}

function statusLabel(value: string) { return ({ pending: '待生效', active: '已持有', consumed: '已消费', released: '已释放', expired: '已到期' } as Record<string, string>)[value] ?? value }
function formatAmount(value: number, currency: string) { return `${currency} ${(value / 1_000_000).toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 6 })}` }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-7)}` }
function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
