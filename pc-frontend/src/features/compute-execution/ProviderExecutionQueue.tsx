import { useState } from 'react'
import { Activity, Clock3, LoaderCircle, Play, Server } from 'lucide-react'
import { type ComputeReservationReceipt } from '../compute-market/computeMarketApi'
import { type ComputeAttemptLeaseStateReceipt } from './computeExecutionApi'
import styles from './ComputeExecutionPage.module.css'

interface Props {
  candidates: ComputeReservationReceipt[]
  leases: ComputeAttemptLeaseStateReceipt[]
  loading: boolean
  selectedLeaseId: string
  onSelectLease: (leaseId: string) => void
  onActivate: (candidate: ComputeReservationReceipt) => void
}

export default function ProviderExecutionQueue({ candidates, leases, loading, selectedLeaseId, onSelectLease, onActivate }: Props) {
  const [view, setView] = useState<'leases' | 'candidates'>('leases')
  const items = view === 'leases' ? leases : candidates
  const empty = view === 'leases' ? '当前 Provider 尚无 Attempt Lease' : '当前 Provider 没有待激活任务'

  return <section className={styles.queue}>
    <header><div><h2>履约队列</h2><span>选择已有 Lease，或登记一笔首次激活</span></div></header>
    <div className={styles.queueTabs} aria-label="履约队列视图">
      <button type="button" data-active={view === 'leases'} onClick={() => setView('leases')}><Activity size={13} />执行 Lease <b>{leases.length}</b></button>
      <button type="button" data-active={view === 'candidates'} onClick={() => setView('candidates')}><Clock3 size={13} />待激活 <b>{candidates.length}</b></button>
    </div>
    {loading && !items.length && <div className={styles.empty}><LoaderCircle size={16} className={styles.spinning} />读取队列</div>}
    {!loading && !items.length && <div className={styles.empty}>{empty}</div>}
    {view === 'leases' && leases.map((state) => <button type="button" className={styles.leaseRow} data-active={state.lease.lease_id === selectedLeaseId} key={state.lease.lease_id} onClick={() => onSelectLease(state.lease.lease_id)}><Activity size={16} /><span><strong>{state.lease.executor_id}</strong><small>{shortId(state.lease.lease_id)} · {statusLabel(state.lease.status)}</small></span><b>rev {state.lease_revision}</b></button>)}
    {view === 'candidates' && candidates.map((candidate) => <div className={styles.candidate} key={candidate.reservation.reservation_id}><div className={styles.candidateIcon}><Server size={17} /></div><div><strong>{candidate.reservation.price_snapshot.sku.task_kind}</strong><span>{shortId(candidate.reservation.reservation_id)} · 至 {formatTime(candidate.reservation.expires_at)}</span><small>{candidate.reservation.reserved_capacity.map((item) => `${item.meter} ${item.quantity}`).join(' · ')}</small></div><div><strong>{formatAmount(candidate.reservation.price_snapshot.consumer_max_amount_micros)}</strong><span>rev {candidate.revision}</span></div><button type="button" onClick={() => onActivate(candidate)}><Play size={14} />登记激活</button></div>)}
  </section>
}

function formatAmount(micros: number) { return `CNY ${(micros / 1_000_000).toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 6 })}` }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-7)}` }
function statusLabel(value: string) { return ({ staging: '准备中', running: '运行中', terminal: '已终结' } as Record<string, string>)[value] ?? value }
