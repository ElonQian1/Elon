import { useState, type FormEvent } from 'react'
import { Gauge, LoaderCircle, X } from 'lucide-react'
import { type ComputeAttemptUsageTemplateReceipt, type DeclareComputeAttemptUsageBody } from './computeExecutionApi'
import styles from './ComputeExecutionDialog.module.css'

interface Props {
  template: ComputeAttemptUsageTemplateReceipt
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: DeclareComputeAttemptUsageBody) => Promise<void>
}

export default function DeclareUsageDialog({ template, busy, error, onClose, onSubmit }: Props) {
  const [identity] = useState(createIdentity)
  const [usageRef, setUsageRef] = useState('')
  const [quantities, setQuantities] = useState<Record<string, string>>(() => Object.fromEntries(template.meters.map((line) => [line.meter, String(line.previous_cumulative_quantity)])))
  const [confirmed, setConfirmed] = useState(false)
  const readings = template.meters.map((line) => ({ meter: line.meter, cumulative_quantity: Number(quantities[line.meter]) }))
  const validReadings = readings.every((reading, index) => Number.isSafeInteger(reading.cumulative_quantity) && reading.cumulative_quantity >= template.meters[index].previous_cumulative_quantity)
  const valid = usageRef.trim() && validReadings && confirmed && !busy

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!valid) return
    await onSubmit({
      expected_lease_revision: template.lease_revision,
      expected_lease_digest: template.lease_digest,
      expected_fencing_generation: template.fencing_generation,
      sequence_no: template.next_sequence_no,
      executor_usage_ref: usageRef.trim(),
      cumulative_declared_usage: readings,
      idempotency_key: identity,
      confirm_provider_declaration_only: true,
    })
  }

  return <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}><section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="declare-usage-title"><header><div><Gauge size={18} /><h2 id="declare-usage-title">登记累计用量</h2></div><button type="button" className={styles.iconButton} onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header><form onSubmit={(event) => void submit(event)}>{error && <div className={styles.error}>{error}</div>}<div className={styles.facts}><div><span>任务</span><strong>{template.task_kind}</strong></div><div><span>序号</span><strong>{template.next_sequence_no}</strong></div><div><span>Lease revision</span><strong>{template.lease_revision}</strong></div></div><label className={styles.singleField}><span>外部执行器用量引用</span><input value={usageRef} onChange={(event) => { setUsageRef(event.target.value); setConfirmed(false) }} placeholder="usage://executor/event/..." /></label><div className={styles.meterList}>{template.meters.map((line) => <label className={styles.meterRow} key={line.meter}><span><strong>{line.meter}</strong><small>上次 {line.previous_cumulative_quantity} · 预留 {line.reserved_quantity}</small></span><input type="number" min={line.previous_cumulative_quantity} step="1" value={quantities[line.meter]} onChange={(event) => { setQuantities((current) => ({ ...current, [line.meter]: event.target.value })); setConfirmed(false) }} /></label>)}</div>{!validReadings && <div className={styles.error}>累计值必须是安全整数，且不能低于上一份快照。</div>}<div className={styles.boundary}>该快照只是 Provider 对外部执行器累计值的声明。超出预留量会被标记，但不会自动计费；平台尚未验证这些用量。</div><label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我确认 meter 齐全且累计值来自外部执行器记录，并理解该操作不会完成结算。</span></label><code>{template.lease_digest}</code><footer><button type="button" onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.primary} disabled={!valid}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在登记' : '登记快照'}</button></footer></form></section></div>
}

function createIdentity() { const nonce = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`; return `pc-compute-usage:${nonce}` }
