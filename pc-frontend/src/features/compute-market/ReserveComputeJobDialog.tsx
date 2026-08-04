import { useMemo, useState, type FormEvent } from 'react'
import { CircleDollarSign, LoaderCircle, X } from 'lucide-react'
import {
  type ComputeJobReceipt,
  type ComputeQuoteCandidate,
  type ComputeReservedCapacity,
  type ReserveComputeJobBody,
} from './computeMarketApi'
import styles from './ReserveComputeJobDialog.module.css'

interface Props {
  job: ComputeJobReceipt
  candidate: ComputeQuoteCandidate
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: ReserveComputeJobBody) => Promise<void>
}

export default function ReserveComputeJobDialog({ job, candidate, busy, error, onClose, onSubmit }: Props) {
  const [identity] = useState(createIdentity)
  const [capacity, setCapacity] = useState<ComputeReservedCapacity[]>(() => initialCapacity(job, candidate))
  const expiryBoundary = useMemo(() => minimumExpiry(job, candidate), [candidate, job])
  const [expiresAt, setExpiresAt] = useState(() => localValue(new Date(Math.min(Date.now() + 30 * 60_000, expiryBoundary))))
  const [confirmed, setConfirmed] = useState(false)
  const validation = useMemo(() => validateCapacity(job, candidate, capacity), [candidate, capacity, job])
  const expiryMs = new Date(expiresAt).getTime()
  const validExpiry = Number.isFinite(expiryMs) && expiryMs > Date.now() && expiryMs <= expiryBoundary
  const valid = validation === '' && validExpiry && confirmed && !busy

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!valid) return
    await onSubmit({
      reservation_id: identity.reservationId,
      idempotency_key: identity.idempotencyKey,
      job_id: job.job.job_id,
      expected_job_revision: job.revision,
      expected_job_digest: job.job_digest,
      reserved_capacity: capacity,
      expires_at: new Date(expiresAt).toISOString(),
    })
  }

  function updateQuantity(meter: string, quantity: number) {
    setCapacity((current) => current.map((item) => item.meter === meter ? { ...item, quantity } : item))
    setConfirmed(false)
  }

  return <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}><section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="reserve-job-title"><header><div><CircleDollarSign size={18} /><h2 id="reserve-job-title">预留预算与容量</h2></div><button type="button" onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header><form onSubmit={(event) => void submit(event)}>{error && <div className={styles.error}>{error}</div>}<div className={styles.facts}><div><span>冻结上限</span><strong>{formatAmount(candidate.price_snapshot.consumer_max_amount_micros, candidate.price_snapshot.currency)}</strong></div><div><span>实际冻结</span><strong>{formatFen(candidate.price_snapshot.consumer_max_amount_micros)}</strong></div><div><span>Provider</span><strong>{candidate.provider.display_name}</strong></div><div><span>Job 版本</span><strong>{job.revision}</strong></div></div><fieldset><legend>逐 meter 预留</legend><div className={styles.meters}>{capacity.map((item) => { const component = candidate.price_snapshot.components.find((row) => row.meter === item.meter); const limit = job.job.workload.usage_limits.find((row) => row.meter === item.meter); return <label key={item.meter}><span>{item.meter}<small>粒度 {component?.unit_size ?? '-'} · 上限 {Math.min(limit?.max_quantity ?? 0, component?.max_units ?? 0)}</small></span><input type="number" min={component?.unit_size ?? 1} max={Math.min(limit?.max_quantity ?? 0, component?.max_units ?? 0)} step={component?.unit_size ?? 1} value={item.quantity} onChange={(event) => updateQuantity(item.meter, Number(event.target.value))} /></label> })}</div>{validation && <div className={styles.inlineError}>{validation}</div>}</fieldset><label className={styles.expiry}><span>预留到期时间</span><input type="datetime-local" value={expiresAt} max={localValue(new Date(expiryBoundary))} onChange={(event) => { setExpiresAt(event.target.value); setConfirmed(false) }} /><small>不得晚于 Job 截止、报价失效或交付窗口结束时间。</small></label>{!validExpiry && <div className={styles.inlineError}>到期时间必须在未来，并处于当前合同的最早截止边界内。</div>}<div className={styles.boundary}>确认后，系统将原子冻结人民币余额并持有全部 meter 容量，同时把 Job 变为“已预留”；本操作不会创建执行 Attempt、派发节点或完成结算。</div><label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我已核对冻结金额、容量数量、到期时间和当前 Job 版本，确认执行真实预留。</span></label><code>{candidate.price_snapshot.snapshot_digest}</code><footer><button type="button" onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.primary} disabled={!valid}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在预留' : '确认预留'}</button></footer></form></section></div>
}

function initialCapacity(job: ComputeJobReceipt, candidate: ComputeQuoteCandidate) {
  return candidate.price_snapshot.components.map((component) => {
    const jobLimit = job.job.workload.usage_limits.find((item) => item.meter === component.meter)?.max_quantity ?? 0
    const maximum = Math.min(jobLimit, component.max_units)
    return { meter: component.meter, quantity: Math.floor(maximum / component.unit_size) * component.unit_size }
  })
}

function validateCapacity(job: ComputeJobReceipt, candidate: ComputeQuoteCandidate, capacity: ComputeReservedCapacity[]) {
  const components = candidate.price_snapshot.components
  if (!capacity.length || capacity.length !== components.length || capacity.length !== job.job.workload.usage_limits.length) return 'Job、报价与预留的 meter 集合不一致。'
  for (const item of capacity) {
    const component = components.find((row) => row.meter === item.meter)
    const limit = job.job.workload.usage_limits.find((row) => row.meter === item.meter)
    if (!component || !limit) return `${item.meter} 不在当前合同中。`
    if (!Number.isSafeInteger(item.quantity) || item.quantity <= 0 || item.quantity > limit.max_quantity || item.quantity > component.max_units || item.quantity % component.unit_size !== 0) return `${item.meter} 必须为正整数、符合计价粒度且不超过合同上限。`
  }
  return ''
}

function minimumExpiry(job: ComputeJobReceipt, candidate: ComputeQuoteCandidate) {
  const boundaries = [job.job.workload.deadline_at, candidate.price_snapshot.expires_at, candidate.price_snapshot.delivery_window.ends_at_utc]
    .map((value) => new Date(value).getTime())
    .filter(Number.isFinite)
  return boundaries.length ? Math.min(...boundaries) : Date.now()
}

function createIdentity() { const nonce = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`; return { reservationId: `reservation_${nonce}`, idempotencyKey: `pc-compute-reservation:${nonce}` } }
function localValue(date: Date) { return new Date(date.getTime() - date.getTimezoneOffset() * 60_000).toISOString().slice(0, 16) }
function formatAmount(value: number, currency: string) { return `${currency} ${(value / 1_000_000).toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 6 })}` }
function formatFen(micros: number) { return `CNY ${(Math.ceil(micros / 10_000) / 100).toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}` }
