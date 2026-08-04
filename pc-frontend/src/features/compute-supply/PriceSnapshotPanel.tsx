import { useCallback, useEffect, useMemo, useState } from 'react'
import { CircleCheck, LineChart, LoaderCircle, Plus, RefreshCw } from 'lucide-react'
import { type MyComputeOfferView } from './computeOfferApi'
import PublishPriceSnapshotDialog from './PublishPriceSnapshotDialog'
import {
  computePriceSnapshotApi,
  type MyComputePriceSnapshotView,
  type PublishComputePriceSnapshotBody,
} from './computePriceSnapshotApi'
import styles from './PriceSnapshotPanel.module.css'

export default function PriceSnapshotPanel({ view }: { view: MyComputeOfferView }) {
  const [snapshots, setSnapshots] = useState<MyComputePriceSnapshotView[]>([])
  const [selectedId, setSelectedId] = useState('')
  const [dialogOpen, setDialogOpen] = useState(false)
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')
  const selected = useMemo(() => snapshots.find((item) => item.snapshot.snapshot_id === selectedId) ?? snapshots[0] ?? null, [selectedId, snapshots])

  const load = useCallback(async (preferredId?: string) => {
    setLoading(true); setError('')
    try {
      const response = await computePriceSnapshotApi.list(view)
      setSnapshots(response)
      setSelectedId((current) => preferredId && response.some((item) => item.snapshot.snapshot_id === preferredId) ? preferredId : response.some((item) => item.snapshot.snapshot_id === current) ? current : response[0]?.snapshot.snapshot_id ?? '')
    } catch (reason) { setError(messageOf(reason, '报价快照读取失败')) } finally { setLoading(false) }
  }, [view])

  useEffect(() => { setSnapshots([]); setSelectedId(''); setNotice(''); void load() }, [load])

  async function publish(body: PublishComputePriceSnapshotBody) {
    if (busy) return
    setBusy(true); setError(''); setNotice('')
    try {
      const created = await computePriceSnapshotApi.publish(view, body)
      setDialogOpen(false); setNotice(created.replayed ? '已返回同一幂等报价快照。' : '报价快照已写入，当前 Offer 可进入候选发现。')
      await load(created.snapshot.snapshot_id)
    } catch (reason) { setError(messageOf(reason, '报价快照发布失败')) } finally { setBusy(false) }
  }

  return <section className={styles.panel}><header><div><LineChart size={16} /><div><h4>报价快照</h4><span>fallback_curve · 不可变历史</span></div></div><div><button type="button" onClick={() => void load()} disabled={loading} aria-label="刷新报价快照" title="刷新报价快照"><RefreshCw size={13} className={loading ? styles.spinning : ''} /></button>{view.offer.status === 'active' && <button type="button" onClick={() => { setError(''); setDialogOpen(true) }} disabled={!view.offer.delivery_windows.length}><Plus size={13} />发布快照</button>}</div></header>{error && !dialogOpen && <div className={styles.error}>{error}</div>}{notice && <div className={styles.notice}><CircleCheck size={14} />{notice}</div>}{loading && !snapshots.length ? <div className={styles.empty}><LoaderCircle size={16} className={styles.spinning} />读取报价历史</div> : snapshots.length ? <div className={styles.workspace}><nav>{snapshots.map((item) => <button type="button" key={item.snapshot.snapshot_id} data-active={item.snapshot.snapshot_id === selected?.snapshot.snapshot_id} onClick={() => setSelectedId(item.snapshot.snapshot_id)}><strong>{formatAmount(item.snapshot.consumer_max_amount_micros, item.snapshot.currency)}</strong><span>{formatTime(item.snapshot.quoted_at)}</span></button>)}</nav>{selected && <article><div className={styles.facts}><div><span>消费者上限</span><strong>{formatAmount(selected.snapshot.consumer_max_amount_micros, selected.snapshot.currency)}</strong></div><div><span>Provider 上限</span><strong>{formatAmount(selected.snapshot.provider_max_amount_micros, selected.snapshot.currency)}</strong></div><div><span>来源</span><strong>{selected.snapshot.price_source.source_kind}</strong></div><div><span>失效时间</span><strong>{formatTime(selected.snapshot.expires_at)}</strong></div></div><code>{selected.snapshot.snapshot_digest}</code><footer>进入候选发现；Reservation、容量与资金效果均为 none。</footer></article>}</div> : <div className={styles.empty}>{view.offer.status === 'active' ? '尚未发布报价快照' : '当前 Offer 没有报价快照'}</div>}{dialogOpen && <PublishPriceSnapshotDialog view={view} busy={busy} error={error} onClose={() => setDialogOpen(false)} onSubmit={publish} />}</section>
}

function formatAmount(value: number, currency: string) { return `${currency} ${(value / 1_000_000).toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 6 })}` }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
