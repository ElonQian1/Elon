import { useCallback, useEffect, useState, type FormEvent } from 'react'
import { CircleCheck, PackageCheck, RefreshCw, Search, ShieldCheck, TriangleAlert } from 'lucide-react'
import { useAuthStore } from '../../store/auth'
import { type MyComputeOfferView } from '../compute-supply/computeOfferApi'
import OfferAdminActionDialog from './OfferAdminActionDialog'
import { computeOfferAdminApi, type ComputeOfferAdminAction, type ComputeOfferAdminReceipt } from './computeOfferAdminApi'
import styles from './ComputeOfferAdminPage.module.css'

export default function ComputeOfferAdminPage() {
  const user = useAuthStore((state) => state.user)
  const isAdmin = user?.role === 'admin' || user?.role === 'owner'
  const [drafts, setDrafts] = useState<MyComputeOfferView[]>([])
  const [selected, setSelected] = useState<MyComputeOfferView | null>(null)
  const [lookupId, setLookupId] = useState('')
  const [action, setAction] = useState<ComputeOfferAdminAction | null>(null)
  const [receipt, setReceipt] = useState<ComputeOfferAdminReceipt | null>(null)
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')

  const loadDrafts = useCallback(async () => {
    if (!isAdmin) return
    setLoading(true); setError('')
    try {
      const response = await computeOfferAdminApi.drafts()
      setDrafts(response)
      setSelected((current) => current ?? response[0] ?? null)
    } catch (reason) { setError(messageOf(reason, 'Offer 待审队列读取失败')) } finally { setLoading(false) }
  }, [isAdmin])

  useEffect(() => { void loadDrafts() }, [loadDrafts])

  async function lookup(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!lookupId.trim() || loading) return
    setLoading(true); setError(''); setNotice(''); setReceipt(null)
    try { setSelected(await computeOfferAdminApi.get(lookupId.trim())) }
    catch (reason) { setError(messageOf(reason, 'Offer 读取失败')) } finally { setLoading(false) }
  }

  async function execute(reason: string) {
    if (!selected || !action || busy) return
    setBusy(true); setError(''); setNotice('')
    try {
      const nextReceipt = action === 'publish'
        ? await computeOfferAdminApi.publish(selected)
        : await computeOfferAdminApi.transition(selected, action, reason)
      setReceipt(nextReceipt); setAction(null)
      const current = await computeOfferAdminApi.get(selected.offer.offer_id)
      setSelected(current); setNotice(actionNotice(action)); await loadDrafts()
    } catch (reasonValue) { setError(messageOf(reasonValue, 'Offer 状态变更失败')) } finally { setBusy(false) }
  }

  if (!isAdmin) return <main className={styles.denied}><ShieldCheck size={24} /><h1>需要平台管理员权限</h1><p>当前账号不能发布或终结算力 Offer。</p></main>

  return <main className={styles.page}>
    <header className={styles.header}><div><span>市场合同控制</span><h1>算力 Offer 管理</h1><p>发布和安全退场使用精确版本、摘要与不可变回执。</p></div><button type="button" onClick={() => void loadDrafts()} disabled={loading}><RefreshCw size={15} className={loading ? styles.spinning : ''} />刷新队列</button></header>
    <form className={styles.lookup} onSubmit={(event) => void lookup(event)}><input value={lookupId} onChange={(event) => setLookupId(event.target.value)} placeholder="输入 Offer ID 打开 active / draining / 终态合同" /><button type="submit" disabled={!lookupId.trim() || loading}><Search size={14} />打开</button></form>
    {error && !action && <div className={styles.alert} data-tone="error"><TriangleAlert size={15} />{error}</div>}
    {notice && <div className={styles.alert} data-tone="success"><CircleCheck size={15} />{notice}</div>}
    <section className={styles.workbench}><aside><header><strong>待发布草稿</strong><span>{drafts.length}</span></header>{drafts.map((view) => <button type="button" key={view.offer.offer_id} data-active={view.offer.offer_id === selected?.offer.offer_id} onClick={() => { setSelected(view); setReceipt(null) }}><PackageCheck size={15} /><span><strong>{view.offer.sku.sku_id}</strong><small>v{view.offer.offer_version} · {shortId(view.offer.provider_id)}</small></span></button>)}{!drafts.length && !loading && <div className={styles.empty}>当前没有待发布草稿</div>}</aside><div className={styles.detail}>{selected ? <OfferDetail view={selected} receipt={receipt} onAction={setAction} /> : <div className={styles.empty}>从待审队列选择，或按 Offer ID 打开</div>}</div></section>
    {action && selected && <OfferAdminActionDialog view={selected} action={action} busy={busy} error={error} onClose={() => setAction(null)} onSubmit={execute} />}
  </main>
}

function OfferDetail({ view, receipt, onAction }: { view: MyComputeOfferView; receipt: ComputeOfferAdminReceipt | null; onAction: (action: ComputeOfferAdminAction) => void }) {
  const offer = view.offer
  return <><header className={styles.offerHeader}><div><span>Offer ID</span><h2>{offer.offer_id}</h2></div><b>{statusLabel(offer.status)}</b></header><div className={styles.facts}><div><span>版本</span><strong>{offer.offer_version}</strong></div><div><span>Provider</span><strong>{shortId(offer.provider_id)}</strong></div><div><span>Pool</span><strong>{shortId(offer.capacity_pool.pool_id)}</strong></div><div><span>失效时间</span><strong>{formatTime(offer.valid_until)}</strong></div></div><section className={styles.contract}><h3>{offer.sku.sku_id}</h3><p>{offer.sku.task_kind} · {offer.sku.region_or_data_zone} · {offer.runtime.runtime_family}/{offer.runtime.precision}</p><div>{offer.capacity.map((line) => <span key={line.bucket.bucket_id}>{line.bucket.meter}: {line.reservable_units}/{line.total_units}</span>)}</div><code>{offer.offer_digest}</code></section>{receipt && <section className={styles.receipt}><CircleCheck size={17} /><div><strong>本次状态回执已写入</strong><span>{receiptTarget(receipt)}</span><code>{receiptDigest(receipt)}</code></div></section>}<footer className={styles.actions}><span>状态变化不生成报价、不直接移动容量或资金。</span><div>{offer.status === 'draft' && <button type="button" onClick={() => onAction('publish')}>发布 active</button>}{offer.status === 'active' && <button type="button" data-tone="danger" onClick={() => onAction('drain')}>安全退场</button>}{offer.status === 'draining' && <><button type="button" onClick={() => onAction('expire')}>到期终结</button><button type="button" data-tone="danger" onClick={() => onAction('revoke')}>撤销终结</button></>}</div></footer></>
}

function statusLabel(value: string) { return ({ draft: '待发布', active: '已发布', draining: '退场中', expired: '已到期', revoked: '已撤销' } as Record<string, string>)[value] ?? value }
function receiptDigest(receipt: ComputeOfferAdminReceipt) { return 'publication_digest' in receipt ? receipt.publication_digest : receipt.event_digest }
function receiptTarget(receipt: ComputeOfferAdminReceipt) { return 'offer_effect' in receipt ? 'Offer 已发布为 active；Price Snapshot 仍未生成' : `Offer 已转为 ${receipt.target_status}` }
function actionNotice(action: ComputeOfferAdminAction) { return ({ publish: 'Offer 已原子发布；仍需单独生成 Price Snapshot 才能进入报价。', drain: 'Offer 已转为 draining，不再进入新的报价候选。', expire: 'Offer 已进入 expired 终态。', revoke: 'Offer 已进入 revoked 终态。' } as Record<string, string>)[action] }
function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-8)}` }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
function messageOf(reason: unknown, fallback: string) { if (reason instanceof Error && reason.message) return reason.message; if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') return reason.message; return fallback }
