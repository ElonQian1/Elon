import { useState, type FormEvent } from 'react'
import { CirclePlus, Search, Trash2 } from 'lucide-react'
import { computeOfferAdminApi } from '../compute-offers/computeOfferAdminApi'
import {
  type ComputePriceComponent,
  type MyComputeOfferView,
} from '../compute-supply/computeOfferApi'
import { type ReferenceCurveEntryIntent } from './computeReferenceCurveApi'
import {
  buildReferenceEntry,
  componentsFromOffer,
  formatMicros,
} from './referenceCurveDraft'
import styles from './ReferenceCurveDialog.module.css'

interface Props {
  entries: ReferenceCurveEntryIntent[]
  onAdd: (entry: ReferenceCurveEntryIntent) => void
  onRemove: (index: number) => void
}

export default function ReferenceCurveOfferEntryBuilder({ entries, onAdd, onRemove }: Props) {
  const [offerId, setOfferId] = useState('')
  const [view, setView] = useState<MyComputeOfferView | null>(null)
  const [windowId, setWindowId] = useState('')
  const [components, setComponents] = useState<ComputePriceComponent[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')

  async function loadOffer(event: FormEvent) {
    event.preventDefault()
    if (!offerId.trim() || loading) return
    setLoading(true); setError('')
    try {
      const loaded = await computeOfferAdminApi.get(offerId.trim())
      if (loaded.offer.status !== 'active') throw new Error('只有 active Offer 可以进入参考价格批次')
      if (!['spot', 'capacity_future'].includes(loaded.offer.price_terms.pricing_mode)) {
        throw new Error('当前 Offer 的定价模式不支持平台参考价格 V1')
      }
      setView(loaded)
      setWindowId(loaded.offer.delivery_windows[0]?.binding.window_id ?? '')
      setComponents(componentsFromOffer(loaded))
    } catch (reason) {
      setView(null); setComponents([]); setWindowId('')
      setError(messageOf(reason, 'Offer 读取失败'))
    } finally { setLoading(false) }
  }

  function updateComponent(index: number, key: keyof ComputePriceComponent, value: string) {
    setComponents((current) => current.map((component, componentIndex) => componentIndex === index
      ? { ...component, [key]: Number(value) }
      : component))
  }

  function addEntry() {
    if (!view) return
    setError('')
    try {
      if (entries.some((entry) => entry.offer_id === view.offer.offer_id
        && entry.offer_version === view.offer.offer_version
        && entry.delivery_window_id === windowId)) {
        throw new Error('当前 Offer 交付窗口已经加入批次')
      }
      onAdd(buildReferenceEntry(view, windowId, components, entries.length + 1))
      setView(null); setOfferId(''); setWindowId(''); setComponents([])
    } catch (reason) { setError(messageOf(reason, '价格条目无效')) }
  }

  return <section className={styles.entryBuilder}>
    <header><div><strong>批次条目</strong><span>{entries.length}/32</span></div></header>
    {entries.length > 0 && <div className={styles.entryList}>{entries.map((entry, index) => <article key={`${entry.offer_id}:${entry.delivery_window_id}`}>
      <div><strong>{entry.sku_id}</strong><span>{shortId(entry.offer_id)} · v{entry.offer_version}</span></div>
      <div><strong>{formatMicros(entry.consumer_max_amount_micros)}</strong><span>供给者 {formatMicros(entry.provider_max_amount_micros)}</span></div>
      <button type="button" title="移除条目" aria-label="移除条目" onClick={() => onRemove(index)}><Trash2 size={15} /></button>
    </article>)}</div>}
    <form className={styles.offerLookup} onSubmit={(event) => void loadOffer(event)}>
      <input value={offerId} onChange={(event) => setOfferId(event.target.value)} placeholder="输入 active Offer ID" />
      <button type="submit" disabled={!offerId.trim() || loading}><Search size={14} />{loading ? '读取中' : '读取 Offer'}</button>
    </form>
    {error && <p className={styles.inlineError}>{error}</p>}
    {view && <div className={styles.offerEditor}>
      <div className={styles.offerIdentity}><div><span>SKU</span><strong>{view.offer.sku.sku_id}</strong></div><div><span>定价模式</span><strong>{view.offer.price_terms.pricing_mode}</strong></div></div>
      <label><span>交付窗口</span><select value={windowId} onChange={(event) => setWindowId(event.target.value)}>{view.offer.delivery_windows.map((window) => <option key={window.binding.window_id} value={window.binding.window_id}>{window.binding.window_id}</option>)}</select></label>
      <div className={styles.componentTable}><div className={styles.componentHead}><span>计量项</span><span>单位粒度</span><span>最大数量</span><span>消费者单价（微元）</span><span>供给者单价（微元）</span></div>{components.map((component, index) => <div className={styles.componentRow} key={component.meter}><strong>{component.meter}</strong><input type="number" min="1" step="1" value={component.unit_size} onChange={(event) => updateComponent(index, 'unit_size', event.target.value)} /><input type="number" min="1" step="1" value={component.max_units} onChange={(event) => updateComponent(index, 'max_units', event.target.value)} /><input type="number" min="0" step="1" value={component.consumer_unit_price_micros} onChange={(event) => updateComponent(index, 'consumer_unit_price_micros', event.target.value)} /><input type="number" min="0" step="1" value={component.provider_unit_price_micros} onChange={(event) => updateComponent(index, 'provider_unit_price_micros', event.target.value)} /></div>)}</div>
      <button type="button" className={styles.addEntry} onClick={addEntry} disabled={!windowId || !components.length || entries.length >= 32}><CirclePlus size={15} />加入批次</button>
    </div>}
  </section>
}

function shortId(value: string) { return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-8)}` }
function messageOf(reason: unknown, fallback: string) { return reason instanceof Error && reason.message ? reason.message : fallback }
