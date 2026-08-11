import { useMemo, useState, type FormEvent } from 'react'
import { LoaderCircle, LockKeyhole, X } from 'lucide-react'
import {
  type CapacityCommitmentQuantity,
  type CapacityCommitmentSourceView,
  type CreateCapacityCommitmentBody,
} from './computeCapacityCommitmentApi'
import { type MyComputeOfferView } from './computeOfferApi'
import styles from './OfferDraftActionDialog.module.css'

interface Props {
  view: MyComputeOfferView
  source: CapacityCommitmentSourceView
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: CreateCapacityCommitmentBody) => Promise<void>
}

interface QuantitySpec extends CapacityCommitmentQuantity {
  step: number
  maximum: number
}

export default function CreateCapacityCommitmentDialog({ view, source, busy, error, onClose, onSubmit }: Props) {
  const specs = useMemo(() => quantitySpecs(view, source), [source, view])
  const [quantities, setQuantities] = useState<Record<string, number>>(() => Object.fromEntries(specs.map((item) => [item.meter, item.quantity_units])))
  const [confirmed, setConfirmed] = useState(false)
  const [idempotencyKey] = useState(() => `capacity-commitment:${source.snapshot.snapshot_id}:${Date.now()}`)
  const valid = specs.length > 0 && specs.every((item) => {
    const value = quantities[item.meter] ?? 0
    return Number.isSafeInteger(value) && value > 0 && value <= item.maximum && value % item.step === 0
  })

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!confirmed || !valid || busy) return
    const offer = view.offer
    await onSubmit({
      idempotency_key: idempotencyKey,
      provider_policy_revision: view.provider_policy_revision,
      provider_digest: view.provider_digest,
      offer_id: offer.offer_id,
      offer_version: offer.offer_version,
      offer_digest: offer.offer_digest,
      capacity_epoch: offer.capacity_pool.capacity_epoch,
      pool_revision: offer.capacity_pool.pool_revision,
      pool_digest: offer.capacity_pool.pool_digest,
      delivery_window_id: source.snapshot.delivery_window.binding.window_id,
      delivery_window_digest: source.snapshot.delivery_window.binding.window_digest,
      price_snapshot_id: source.snapshot.snapshot_id,
      price_snapshot_digest: source.snapshot.snapshot_digest,
      reference_binding_id: source.reference_binding.binding_id,
      reference_binding_digest: source.reference_binding.binding_digest,
      instrument_id: offer.price_terms.instrument_id ?? '',
      quantities: specs.map((item) => ({ meter: item.meter, quantity_units: quantities[item.meter] })),
      confirm_commitment: true,
    })
  }

  return <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}><section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="create-capacity-commitment-title">
    <header><div><LockKeyhole size={18} /><h2 id="create-capacity-commitment-title">锁定未来容量</h2></div><button type="button" onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header>
    <form onSubmit={(event) => void submit(event)}>
      {error && <div className={styles.error}>{error}</div>}
      <div className={styles.identity}><span>{view.offer.sku.sku_id} · {formatTime(source.snapshot.delivery_window.starts_at_utc)}</span><strong>{view.offer.price_terms.instrument_id}</strong><code>{source.reference_binding.binding_digest}</code></div>
      <fieldset><legend>承诺数量</legend>{specs.map((item) => <div className={styles.row} key={item.meter}><span><strong>{item.meter}</strong><small>步长 {item.step} · 上限 {item.maximum}</small></span><label>数量<input type="number" min={item.step} max={item.maximum} step={item.step} value={quantities[item.meter]} onChange={(event) => setQuantities((current) => ({ ...current, [item.meter]: Number(event.target.value) }))} /></label></div>)}</fieldset>
      <div className={styles.warning}>容量会立即从 available 移到 held。只能在交付窗口开始前主动取消；窗口结束后由平台到期恢复。</div>
      <label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我确认按当前 Offer、价格快照和平台审核绑定锁定以上全部 meter。</span></label>
      <footer><button type="button" onClick={onClose} disabled={busy}>返回</button><button type="submit" className={styles.primary} disabled={!confirmed || !valid || busy}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在锁定' : '确认锁定'}</button></footer>
    </form>
  </section></div>
}

function quantitySpecs(view: MyComputeOfferView, source: CapacityCommitmentSourceView): QuantitySpec[] {
  const windowId = source.snapshot.delivery_window.binding.window_id
  const components = new Map(source.snapshot.components.map((item) => [item.meter, item]))
  return view.offer.capacity
    .filter((item) => item.bucket.delivery_window.window_id === windowId)
    .map((item) => {
      const component = components.get(item.bucket.meter)
      const step = lcm(item.bucket.quantum_units, component?.unit_size ?? 1)
      const maximum = Math.floor(Math.min(item.reservable_units, component?.max_units ?? item.reservable_units) / step) * step
      return { meter: item.bucket.meter, quantity_units: step, step, maximum }
    })
    .filter((item) => item.maximum >= item.step)
}

function lcm(left: number, right: number) { return Math.abs(left * right) / gcd(left, right) }
function gcd(left: number, right: number) { let a = Math.abs(left); let b = Math.abs(right); while (b) { const next = a % b; a = b; b = next } return a || 1 }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
