import { useMemo, useState, type FormEvent } from 'react'
import { CalendarClock, LoaderCircle, X } from 'lucide-react'
import {
  type CreateMyComputeCapacityBucketBody,
  type MyComputeCapacityPool,
} from './computeSupplyApi'
import styles from './CreateCapacityBucketDialog.module.css'

interface Props {
  pool: MyComputeCapacityPool
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: CreateMyComputeCapacityBucketBody) => Promise<void>
}

export default function CreateCapacityBucketDialog({ pool, busy, error, onClose, onSubmit }: Props) {
  const [bucketId] = useState(() => `bucket-${globalThis.crypto.randomUUID()}`)
  const [windowId] = useState(() => `window-${globalThis.crypto.randomUUID()}`)
  const [meter, setMeter] = useState(pool.meter_policies[0]?.meter ?? '')
  const [startsAt, setStartsAt] = useState(() => localInputTime(Date.now() + 15 * 60_000))
  const [endsAt, setEndsAt] = useState(() => localInputTime(Date.now() + 24 * 60 * 60_000))
  const validation = useMemo(() => validateWindow(startsAt, endsAt), [endsAt, startsAt])
  const canSubmit = Boolean(meter && !validation && !busy)

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!canSubmit) return
    await onSubmit({
      bucket_id: bucketId,
      window_id: windowId,
      starts_at_utc: new Date(startsAt).toISOString(),
      ends_at_utc: new Date(endsAt).toISOString(),
      meter,
    })
  }

  return (
    <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}>
      <section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="create-bucket-title">
        <header className={styles.header}>
          <div><span>{shortId(pool.pool_id)}</span><h2 id="create-bucket-title">登记交付窗口</h2></div>
          <button type="button" onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button>
        </header>
        <div className={styles.identity}><CalendarClock size={18} /><div><span>Bucket ID</span><strong>{bucketId}</strong><small>{windowId}</small></div></div>
        <form onSubmit={(event) => void submit(event)}>
          {error && <div className={styles.error} role="alert">{error}</div>}
          <label className={styles.field}><span>计量单位</span><select value={meter} onChange={(event) => setMeter(event.target.value)}>{pool.meter_policies.map((policy) => <option key={policy.meter} value={policy.meter}>{policy.meter} · 量子 {policy.quantum_units}</option>)}</select></label>
          <div className={styles.twoColumns}>
            <label className={styles.field}><span>窗口开始时间</span><input type="datetime-local" value={startsAt} onChange={(event) => setStartsAt(event.target.value)} required /></label>
            <label className={styles.field}><span>窗口结束时间</span><input type="datetime-local" value={endsAt} onChange={(event) => setEndsAt(event.target.value)} required /></label>
          </div>
          {validation && <div className={styles.validation}>{validation}</div>}
          <div className={styles.boundary}>此操作只登记 UTC 交付窗口和计量边界，初始供给为 0，不激活 Pool、不发布报价。</div>
          <footer className={styles.footer}>
            <button type="button" className={styles.cancelButton} onClick={onClose} disabled={busy}>取消</button>
            <button type="submit" className={styles.submitButton} disabled={!canSubmit}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在登记' : '确认登记'}</button>
          </footer>
        </form>
      </section>
    </div>
  )
}

function localInputTime(timestamp: number) {
  const date = new Date(timestamp - new Date(timestamp).getTimezoneOffset() * 60_000)
  return date.toISOString().slice(0, 16)
}

function validateWindow(startsAt: string, endsAt: string) {
  const starts = new Date(startsAt).getTime()
  const ends = new Date(endsAt).getTime()
  if (!Number.isFinite(starts) || !Number.isFinite(ends)) return '请选择有效的开始和结束时间'
  if (starts >= ends) return '结束时间必须晚于开始时间'
  if (ends <= Date.now()) return '结束时间必须晚于当前时间'
  return ''
}

function shortId(value: string) { return value.length <= 28 ? value : `${value.slice(0, 14)}…${value.slice(-8)}` }
