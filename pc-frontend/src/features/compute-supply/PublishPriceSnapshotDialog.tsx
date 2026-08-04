import { useState, type FormEvent } from 'react'
import { LineChart, LoaderCircle, X } from 'lucide-react'
import { type MyComputeOfferView } from './computeOfferApi'
import {
  type ComputeRoundingMode,
  type PublishComputePriceSnapshotBody,
} from './computePriceSnapshotApi'
import styles from './PublishPriceSnapshotDialog.module.css'

interface Props {
  view: MyComputeOfferView
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: PublishComputePriceSnapshotBody) => Promise<void>
}

export default function PublishPriceSnapshotDialog({ view, busy, error, onClose, onSubmit }: Props) {
  const [windowId, setWindowId] = useState(view.offer.delivery_windows[0]?.binding.window_id ?? '')
  const [consumerAmount, setConsumerAmount] = useState('')
  const [providerAmount, setProviderAmount] = useState('')
  const [ttl, setTtl] = useState(300)
  const [roundingMode, setRoundingMode] = useState<ComputeRoundingMode>('half_up')
  const [confirmed, setConfirmed] = useState(false)
  const [idempotencyKey] = useState(createIdempotencyKey)
  const consumerMicros = parseMicros(consumerAmount)
  const providerMicros = parseMicros(providerAmount)
  const valid = Boolean(windowId && consumerMicros !== null && providerMicros !== null && providerMicros <= consumerMicros && ttl >= 30 && ttl <= 3600)

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!valid || !confirmed || busy || consumerMicros === null || providerMicros === null) return
    await onSubmit({
      expected_offer_version: view.offer.offer_version,
      expected_offer_digest: view.offer.offer_digest,
      delivery_window_id: windowId,
      consumer_max_amount_micros: consumerMicros,
      provider_max_amount_micros: providerMicros,
      ttl_seconds: ttl,
      rounding_mode: roundingMode,
      idempotency_key: idempotencyKey,
      confirm_publish: true,
    })
  }

  return <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}><section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="publish-price-title">
    <header><div><LineChart size={18} /><h2 id="publish-price-title">发布报价快照</h2></div><button type="button" onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header>
    <form onSubmit={(event) => void submit(event)}>{error && <div className={styles.error}>{error}</div>}<div className={styles.boundary}>快照会让当前 active Offer 进入候选发现，但不会创建订单、预留容量、冻结余额或自动成交。</div><label><span>交付窗口</span><select value={windowId} onChange={(event) => { setWindowId(event.target.value); setConfirmed(false) }}>{view.offer.delivery_windows.map((window) => <option key={window.binding.window_id} value={window.binding.window_id}>{formatTime(window.starts_at_utc)} 至 {formatTime(window.ends_at_utc)}</option>)}</select></label><div className={styles.amounts}><label><span>消费者金额上限（{view.offer.price_terms.currency}）</span><input value={consumerAmount} onChange={(event) => { setConsumerAmount(event.target.value); setConfirmed(false) }} inputMode="decimal" placeholder="例如 100.000000" /></label><label><span>Provider 金额上限（{view.offer.price_terms.currency}）</span><input value={providerAmount} onChange={(event) => { setProviderAmount(event.target.value); setConfirmed(false) }} inputMode="decimal" placeholder="不得高于消费者上限" /></label></div><div className={styles.options}><label><span>有效期（秒）</span><input type="number" min={30} max={3600} value={ttl} onChange={(event) => { setTtl(Number(event.target.value)); setConfirmed(false) }} /></label><label><span>舍入方式</span><select value={roundingMode} onChange={(event) => { setRoundingMode(event.target.value as ComputeRoundingMode); setConfirmed(false) }}><option value="half_up">四舍五入</option><option value="half_even">银行家舍入</option><option value="floor">向下取整</option><option value="ceil">向上取整</option></select></label></div><label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我已核对当前 Offer 版本、摘要、窗口与金额上限，确认发布不可变 fallback_curve 快照。</span></label><footer><button type="button" onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.primary} disabled={!valid || !confirmed || busy}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在发布' : '确认发布'}</button></footer></form>
  </section></div>
}

function parseMicros(value: string): number | null {
  const match = /^(\d+)(?:\.(\d{0,6}))?$/.exec(value.trim())
  if (!match) return null
  const micros = BigInt(match[1]) * 1_000_000n + BigInt((match[2] ?? '').padEnd(6, '0'))
  return micros <= BigInt(Number.MAX_SAFE_INTEGER) ? Number(micros) : null
}

function createIdempotencyKey() {
  const nonce = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`
  return `pc-price-snapshot:${nonce}`
}

function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
