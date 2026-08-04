import { useMemo, useState, type FormEvent } from 'react'
import { LoaderCircle, Minus, Plus, X } from 'lucide-react'
import { type MyComputeCapacityBucket } from './computeSupplyApi'
import styles from './BucketSupplyDialog.module.css'

export type SupplyAction = 'add' | 'withdraw'

interface Props {
  bucket: MyComputeCapacityBucket
  initialAction: SupplyAction
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (action: SupplyAction, quantityUnits: number, idempotencyKey: string) => Promise<void>
}

export default function BucketSupplyDialog({ bucket, initialAction, busy, error, onClose, onSubmit }: Props) {
  const [action, setAction] = useState<SupplyAction>(initialAction)
  const [quantity, setQuantity] = useState(String(bucket.balance.binding.quantum_units))
  const [confirmed, setConfirmed] = useState(false)
  const [requestId] = useState(() => globalThis.crypto.randomUUID())
  const parsedQuantity = Number(quantity)
  const quantum = bucket.balance.binding.quantum_units
  const invalidReason = useMemo(() => validateQuantity(parsedQuantity, quantum, action === 'withdraw' ? bucket.balance.available_units : null), [action, bucket.balance.available_units, parsedQuantity, quantum])
  const addWindowEnded = action === 'add' && new Date(bucket.ends_at_utc).getTime() <= Date.now()
  const canSubmit = !invalidReason && !addWindowEnded && confirmed && !busy

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!canSubmit) return
    await onSubmit(action, parsedQuantity, `pc-${action}-${requestId}`)
  }

  return (
    <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}>
      <section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="supply-dialog-title">
        <header className={styles.header}>
          <div><span>{shortId(bucket.balance.binding.bucket_id)}</span><h2 id="supply-dialog-title">调整 Bucket 供给</h2></div>
          <button type="button" onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button>
        </header>
        <form onSubmit={(event) => void submit(event)}>
          {error && <div className={styles.error} role="alert">{error}</div>}
          <div className={styles.segmented} aria-label="供给操作">
            <button type="button" data-active={action === 'add'} onClick={() => { setAction('add'); setConfirmed(false) }}><Plus size={15} />追加供给</button>
            <button type="button" data-active={action === 'withdraw'} onClick={() => { setAction('withdraw'); setConfirmed(false) }}><Minus size={15} />撤出供给</button>
          </div>
          <div className={styles.facts}><div><span>可用</span><strong>{bucket.balance.available_units}</strong></div><div><span>已发行</span><strong>{bucket.balance.issued_units}</strong></div><div><span>最小量子</span><strong>{quantum}</strong></div></div>
          <label className={styles.field}><span>数量（{bucket.balance.binding.meter}）</span><input value={quantity} onChange={(event) => { setQuantity(event.target.value); setConfirmed(false) }} inputMode="numeric" min={quantum} step={quantum} required autoFocus /></label>
          {(invalidReason || addWindowEnded) && <div className={styles.validation}>{addWindowEnded ? '交付窗口已经结束，不能再追加供给' : invalidReason}</div>}
          <label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我确认此次操作只调整内部容量账本；撤出仅限当前可用容量。</span></label>
          <div className={styles.boundary}>不会启动节点任务、生成市场报价、形成收入，也不会执行链上或外部资金结算。</div>
          <footer className={styles.footer}>
            <button type="button" className={styles.cancelButton} onClick={onClose} disabled={busy}>取消</button>
            <button type="submit" className={action === 'add' ? styles.addButton : styles.withdrawButton} disabled={!canSubmit}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在提交' : action === 'add' ? '确认追加' : '确认撤出'}</button>
          </footer>
        </form>
      </section>
    </div>
  )
}

function validateQuantity(value: number, quantum: number, available: number | null) {
  if (!Number.isSafeInteger(value) || value <= 0) return '数量必须为正整数'
  if (value % quantum !== 0) return `数量必须是最小量子 ${quantum} 的整数倍`
  if (available !== null && value > available) return '撤出数量不能超过当前可用容量'
  return ''
}

function shortId(value: string) { return value.length <= 28 ? value : `${value.slice(0, 14)}…${value.slice(-8)}` }
