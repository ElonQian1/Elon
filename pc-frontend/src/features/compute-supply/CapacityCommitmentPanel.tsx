import { useCallback, useEffect, useMemo, useState } from 'react'
import { CircleCheck, LoaderCircle, LockKeyhole, RefreshCw, ShieldAlert, Unlock } from 'lucide-react'
import CancelCapacityCommitmentDialog from './CancelCapacityCommitmentDialog'
import CreateCapacityCommitmentDialog from './CreateCapacityCommitmentDialog'
import {
  computeCapacityCommitmentApi,
  type CapacityCommitmentDetail,
  type CapacityCommitmentSourceView,
  type CreateCapacityCommitmentBody,
} from './computeCapacityCommitmentApi'
import { type MyComputeOfferView } from './computeOfferApi'
import { computePriceSnapshotApi, type MyComputePriceSnapshotView } from './computePriceSnapshotApi'
import styles from './CapacityCommitmentPanel.module.css'

export default function CapacityCommitmentPanel({ view }: { view: MyComputeOfferView }) {
  const [commitments, setCommitments] = useState<CapacityCommitmentDetail[]>([])
  const [snapshots, setSnapshots] = useState<MyComputePriceSnapshotView[]>([])
  const [snapshotId, setSnapshotId] = useState('')
  const [source, setSource] = useState<CapacityCommitmentSourceView | null>(null)
  const [loading, setLoading] = useState(false)
  const [sourceLoading, setSourceLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [createOpen, setCreateOpen] = useState(false)
  const [canceling, setCanceling] = useState<CapacityCommitmentDetail | null>(null)
  const [error, setError] = useState('')
  const [sourceError, setSourceError] = useState('')
  const [notice, setNotice] = useState('')
  const eligibleSnapshots = useMemo(() => snapshots.filter(({ snapshot }) => snapshot.pricing_mode === 'capacity_future'
    && Boolean(view.offer.price_terms.instrument_id)
    && new Date(snapshot.expires_at).getTime() > Date.now()
    && new Date(snapshot.delivery_window.starts_at_utc).getTime() > Date.now()), [snapshots, view.offer.price_terms.instrument_id])
  const selectedSnapshot = eligibleSnapshots.find((item) => item.snapshot.snapshot_id === snapshotId) ?? eligibleSnapshots[0] ?? null
  const offerCommitments = useMemo(() => commitments.filter((item) => item.commitment.offer.offer_id === view.offer.offer_id), [commitments, view.offer.offer_id])

  const load = useCallback(async () => {
    setLoading(true); setError('')
    try {
      const [nextCommitments, nextSnapshots] = await Promise.all([
        computeCapacityCommitmentApi.list(view),
        computePriceSnapshotApi.list(view, 50),
      ])
      setCommitments(nextCommitments)
      setSnapshots(nextSnapshots)
      setSnapshotId((current) => nextSnapshots.some((item) => item.snapshot.snapshot_id === current) ? current : nextSnapshots[0]?.snapshot.snapshot_id ?? '')
    } catch (reason) { setError(messageOf(reason, '容量承诺工作区读取失败')) } finally { setLoading(false) }
  }, [view])

  useEffect(() => { setCommitments([]); setSnapshots([]); setSnapshotId(''); setSource(null); setNotice(''); void load() }, [load])
  useEffect(() => {
    const target = selectedSnapshot
    if (!target) { setSource(null); setSourceError(''); return }
    let active = true
    setSourceLoading(true); setSource(null); setSourceError('')
    void computeCapacityCommitmentApi.source(view, target.snapshot.snapshot_id)
      .then((next) => { if (active) setSource(next) })
      .catch((reason) => { if (active) setSourceError(messageOf(reason, '当前快照缺少平台审核绑定')) })
      .finally(() => { if (active) setSourceLoading(false) })
    return () => { active = false }
  }, [selectedSnapshot, view])

  async function create(body: CreateCapacityCommitmentBody) {
    if (busy) return
    setBusy(true); setError(''); setNotice('')
    try {
      const receipt = await computeCapacityCommitmentApi.create(view, body)
      setCreateOpen(false); setNotice(receipt.replayed ? '已返回同一容量承诺。' : '容量已锁定，承诺回执已写入。')
      await load()
    } catch (reason) { setError(messageOf(reason, '容量承诺创建失败')) } finally { setBusy(false) }
  }

  async function cancel(reason: string, idempotencyKey: string) {
    if (!canceling || busy) return
    setBusy(true); setError(''); setNotice('')
    try {
      const receipt = await computeCapacityCommitmentApi.cancel(view, canceling, reason, idempotencyKey)
      setCanceling(null); setNotice(receipt.replayed ? '已返回同一取消回执。' : '容量承诺已取消，held 容量已归还。')
      await load()
    } catch (cause) { setError(messageOf(cause, '容量承诺取消失败')) } finally { setBusy(false) }
  }

  const canCreate = view.offer.status === 'active' && Boolean(source) && !sourceLoading
  return <section className={styles.panel}>
    <header><div><LockKeyhole size={16} /><div><h4>未来容量承诺</h4><span>平台审核价格 · held 容量</span></div></div><div><button type="button" onClick={() => void load()} disabled={loading} aria-label="刷新容量承诺" title="刷新容量承诺"><RefreshCw size={13} className={loading ? styles.spinning : ''} /></button><button type="button" onClick={() => { setError(''); setCreateOpen(true) }} disabled={!canCreate}><LockKeyhole size={13} />锁定容量</button></div></header>
    {error && !createOpen && !canceling && <div className={styles.error}>{error}</div>}
    {notice && <div className={styles.notice}><CircleCheck size={14} />{notice}</div>}
    {eligibleSnapshots.length > 0 && <div className={styles.sourceBar}><label><span>受治理报价</span><select value={selectedSnapshot?.snapshot.snapshot_id ?? ''} onChange={(event) => setSnapshotId(event.target.value)}>{eligibleSnapshots.map((item) => <option key={item.snapshot.snapshot_id} value={item.snapshot.snapshot_id}>{formatAmount(item.snapshot.consumer_max_amount_micros, item.snapshot.currency)} · {formatTime(item.snapshot.expires_at)}</option>)}</select></label><div data-ready={Boolean(source)}>{sourceLoading ? <LoaderCircle size={13} className={styles.spinning} /> : source ? <CircleCheck size={13} /> : <ShieldAlert size={13} />}<span>{sourceLoading ? '核验价格绑定' : source ? '平台审核绑定有效' : sourceError || '等待平台审核绑定'}</span></div></div>}
    {loading && !offerCommitments.length ? <div className={styles.empty}><LoaderCircle size={16} className={styles.spinning} />读取容量承诺</div> : offerCommitments.length ? <div className={styles.list}>{offerCommitments.map((detail) => {
      const cancelAllowed = detail.current_status === 'committed' && new Date(detail.commitment.delivery_window.starts_at_utc).getTime() > Date.now()
      return <article key={detail.commitment.commitment_id}><div className={styles.identity}><div><strong>{statusLabel(detail.current_status)}</strong><span>{detail.commitment.instrument_id}</span></div><code>{shortId(detail.commitment.commitment_id)}</code></div><div className={styles.quantities}>{detail.quantities.map((item) => <span key={item.meter}><small>{item.meter}</small><strong>{item.quantity_units}</strong></span>)}</div><div className={styles.window}><span>交付 {formatTime(detail.commitment.delivery_window.starts_at_utc)}</span><span>到期 {formatTime(detail.commitment.expires_at)}</span></div><footer><code>{shortDigest(detail.commitment.commitment_digest)}</code>{cancelAllowed && <button type="button" onClick={() => { setError(''); setCanceling(detail) }}><Unlock size={12} />取消</button>}</footer></article>
    })}</div> : <div className={styles.empty}>{eligibleSnapshots.length === 0 ? '等待 capacity_future Offer 的未过期报价快照' : source ? '当前 Offer 尚未锁定未来容量' : '报价需先完成平台四眼审核和应用'}</div>}
    {createOpen && source && <CreateCapacityCommitmentDialog view={view} source={source} busy={busy} error={error} onClose={() => setCreateOpen(false)} onSubmit={create} />}
    {canceling && <CancelCapacityCommitmentDialog detail={canceling} busy={busy} error={error} onClose={() => setCanceling(null)} onSubmit={cancel} />}
  </section>
}

function statusLabel(value: string) { return ({ committed: '已承诺', canceled: '已取消', expired: '已到期' } as Record<string, string>)[value] ?? value }
function formatAmount(value: number, currency: string) { return `${currency} ${(value / 1_000_000).toLocaleString('zh-CN', { maximumFractionDigits: 6 })}` }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-8)}` }
function shortDigest(value: string) { return value.length <= 22 ? value : `${value.slice(0, 10)}…${value.slice(-8)}` }
function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
