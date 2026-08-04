import { useCallback, useEffect, useMemo, useState } from 'react'
import { CircleDollarSign, LoaderCircle, PackagePlus, PencilLine, RefreshCw, Trash2 } from 'lucide-react'
import { type MyComputeProvider } from '../compute-settlement/myComputeSettlementApi'
import CreateOfferDraftDialog from './CreateOfferDraftDialog'
import ReviseOfferDraftDialog from './ReviseOfferDraftDialog'
import RevokeOfferDraftDialog from './RevokeOfferDraftDialog'
import { computeOfferApi, type ComputeOfferDraftBody, type MyComputeOfferView, type ReviseComputeOfferDraftBody } from './computeOfferApi'
import { type MyComputeCapacityBucket, type MyComputeCapacityPool } from './computeSupplyApi'
import styles from './CapacityOfferPanel.module.css'

interface Props { provider: MyComputeProvider | null; pool: MyComputeCapacityPool; buckets: MyComputeCapacityBucket[] }

export default function CapacityOfferPanel({ provider, pool, buckets }: Props) {
  const [offers, setOffers] = useState<MyComputeOfferView[]>([])
  const [selectedId, setSelectedId] = useState('')
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [createOpen, setCreateOpen] = useState(false)
  const [reviseOpen, setReviseOpen] = useState(false)
  const [revokeOpen, setRevokeOpen] = useState(false)
  const [error, setError] = useState('')
  const selected = useMemo(() => offers.find((item) => item.offer.offer_id === selectedId) ?? offers[0] ?? null, [offers, selectedId])
  const canCreate = provider?.status === 'active' && pool.status === 'active' && buckets.some((item) => item.balance.status === 'open' && item.balance.available_units > 0)

  const load = useCallback(async () => {
    setLoading(true); setError('')
    try {
      const response = await computeOfferApi.list(pool.provider_id, pool.pool_id)
      setOffers(response); setSelectedId((current) => response.some((item) => item.offer.offer_id === current) ? current : response[0]?.offer.offer_id ?? '')
    } catch (reason) { setError(messageOf(reason, 'Offer 列表读取失败')) } finally { setLoading(false) }
  }, [pool.pool_id, pool.provider_id])

  useEffect(() => { void load() }, [load])

  async function create(body: ComputeOfferDraftBody) {
    if (busy) return
    setBusy(true); setError('')
    try { const created = await computeOfferApi.create(pool.provider_id, pool.pool_id, body); setCreateOpen(false); await load(); setSelectedId(created.offer.offer_id) }
    catch (reason) { setError(messageOf(reason, 'Offer 草稿创建失败')) } finally { setBusy(false) }
  }

  async function revise(body: ReviseComputeOfferDraftBody) {
    if (!selected || busy) return
    setBusy(true); setError('')
    try { await computeOfferApi.revise(pool.provider_id, pool.pool_id, selected.offer.offer_id, body); setReviseOpen(false); await load(); setSelectedId(selected.offer.offer_id) }
    catch (reason) { setError(messageOf(reason, 'Offer 草稿修订失败')) } finally { setBusy(false) }
  }

  async function revoke() {
    if (!selected || selected.offer.status !== 'draft' || busy) return
    setBusy(true); setError('')
    try { await computeOfferApi.revoke(pool.provider_id, pool.pool_id, selected.offer.offer_id, selected.offer.offer_version, selected.offer.offer_digest); setRevokeOpen(false); await load(); setSelectedId(selected.offer.offer_id) }
    catch (reason) { setError(messageOf(reason, 'Offer 草稿撤销失败')) } finally { setBusy(false) }
  }

  return <section className={styles.panel}>
    <header><div><CircleDollarSign size={17} /><div><h3>市场 Offer</h3><span>草稿不产生市场效果</span></div></div><div><button type="button" onClick={() => void load()} disabled={loading} title="刷新"><RefreshCw size={14} className={loading ? styles.spinning : ''} /></button><button type="button" onClick={() => { setError(''); setCreateOpen(true) }} disabled={!canCreate}><PackagePlus size={14} />创建草稿</button></div></header>
    {error && !createOpen && !reviseOpen && !revokeOpen && <div className={styles.error}>{error}</div>}
    {offers.length > 0 ? <div className={styles.workspace}><nav>{offers.map((item) => <button type="button" key={item.offer.offer_id} data-active={item.offer.offer_id === selected?.offer.offer_id} onClick={() => setSelectedId(item.offer.offer_id)}><strong>{item.offer.sku.sku_id}</strong><span>v{item.offer.offer_version} · {statusLabel(item.offer.status)}</span></button>)}</nav>{selected && <article><header><div><span>Offer ID</span><strong>{selected.offer.offer_id}</strong></div><b>{statusLabel(selected.offer.status)}</b></header><div className={styles.facts}><div><span>任务</span><strong>{selected.offer.sku.task_kind}</strong></div><div><span>区域</span><strong>{selected.offer.sku.region_or_data_zone}</strong></div><div><span>运行时</span><strong>{selected.offer.runtime.runtime_family}</strong></div><div><span>有效期</span><strong>{formatTime(selected.offer.valid_until)}</strong></div></div><div className={styles.capacity}>{selected.offer.capacity.map((line) => <div key={line.bucket.bucket_id}><span>{line.bucket.meter}</span><strong>{line.reservable_units} / {line.total_units}</strong></div>)}</div><code>{selected.offer.offer_digest}</code><footer><span>当前响应 `market_effect=none`；发布、报价和预留均为独立步骤。</span>{selected.offer.status === 'draft' && <div><button type="button" onClick={() => { setError(''); setReviseOpen(true) }}><PencilLine size={13} />修订</button><button type="button" data-tone="danger" onClick={() => { setError(''); setRevokeOpen(true) }}><Trash2 size={13} />撤销</button></div>}</footer></article>}</div> : !loading && <div className={styles.empty}><CircleDollarSign size={21} /><span>{canCreate ? '尚未创建 Offer 草稿' : 'Provider、Pool 和可用 Bucket 激活后才能创建草稿'}</span></div>}
    {loading && offers.length === 0 && <div className={styles.empty}><LoaderCircle size={18} className={styles.spinning} /><span>读取 Offer</span></div>}
    {createOpen && provider && <CreateOfferDraftDialog provider={provider} pool={pool} buckets={buckets} busy={busy} error={error} onClose={() => setCreateOpen(false)} onSubmit={create} />}
    {reviseOpen && selected && <ReviseOfferDraftDialog view={selected} busy={busy} error={error} onClose={() => setReviseOpen(false)} onSubmit={revise} />}
    {revokeOpen && selected && <RevokeOfferDraftDialog view={selected} busy={busy} error={error} onClose={() => setRevokeOpen(false)} onSubmit={revoke} />}
  </section>
}

function statusLabel(value: string) { return ({ draft: '草稿', active: '已发布', draining: '退场中', expired: '已到期', revoked: '已撤销' } as Record<string, string>)[value] ?? value }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
