import { useState, type FormEvent } from 'react'
import { LoaderCircle, OctagonX, X } from 'lucide-react'
import {
  type AbortComputeAttemptBody,
  type ComputeAttemptActivationReceipt,
  type ComputeAttemptLeaseStateReceipt,
} from './computeExecutionApi'
import styles from './ComputeExecutionDialog.module.css'

interface Props {
  activation: ComputeAttemptActivationReceipt
  state: ComputeAttemptLeaseStateReceipt
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: AbortComputeAttemptBody) => Promise<void>
}

export default function AbortAttemptDialog({ activation, state, busy, error, onClose, onSubmit }: Props) {
  const [idempotencyKey] = useState(createKey)
  const [abortRef, setAbortRef] = useState('')
  const [reasonCode, setReasonCode] = useState('staging_setup_failed')
  const [confirmed, setConfirmed] = useState(false)
  const valid = abortRef.trim() && reasonCode.trim() && confirmed && !busy

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!valid) return
    await onSubmit({
      expected_lease_revision: state.lease_revision, expected_lease_digest: state.lease_digest,
      expected_fencing_generation: state.lease.fencing_generation,
      expected_job_revision: activation.running_job.job_revision, expected_job_digest: activation.running_job.job_digest,
      expected_reservation_revision: activation.active_reservation_revision, expected_reservation_digest: activation.active_reservation_digest,
      expected_claim_revision: activation.active_claim.claim_revision, expected_claim_digest: activation.active_claim.claim_digest,
      executor_abort_ref: abortRef.trim(), reason_code: reasonCode.trim(), idempotency_key: idempotencyKey,
      confirm_no_execution_started: true,
    })
  }

  return <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}><section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="abort-attempt-title"><header><div><OctagonX size={18} /><h2 id="abort-attempt-title">中止未执行 Attempt</h2></div><button type="button" className={styles.iconButton} onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header><form onSubmit={(event) => void submit(event)}>{error && <div className={styles.error}>{error}</div>}<div className={styles.facts}><div><span>Lease revision</span><strong>{state.lease_revision}</strong></div><div><span>心跳</span><strong>{state.lease.last_heartbeat_at ? '已有' : '无'}</strong></div><div><span>冻结余额</span><strong>CNY {(activation.budget_reserved_fen / 100).toFixed(2)}</strong></div></div><div className={styles.grid}><label data-wide="true"><span>外部执行器中止凭据引用</span><input value={abortRef} onChange={(event) => { setAbortRef(event.target.value); setConfirmed(false) }} placeholder="receipt://executor/aborted-before-start/..." /></label><label data-wide="true"><span>原因码</span><select value={reasonCode} onChange={(event) => { setReasonCode(event.target.value); setConfirmed(false) }}><option value="staging_setup_failed">staging_setup_failed</option><option value="executor_rejected_before_start">executor_rejected_before_start</option><option value="provider_manual_abort">provider_manual_abort</option></select></label></div><div className={styles.boundary}>仅当前 revision 1、从未心跳的 staging Lease 可以走此路径。成功后会原子全额退回预授权、归还 active 容量，并终结 Job、Reservation 与 Lease；该声明不会验证外部中止证明。</div><label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我确认执行器从未开始任何计算、未产生用量或输出，并核对所有当前版本和摘要。</span></label><code>{state.lease_digest}</code><footer><button type="button" onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.danger} disabled={!valid}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在中止' : '确认中止并退款'}</button></footer></form></section></div>
}

function createKey() { const nonce = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`; return `pc-compute-attempt-abort:${nonce}` }
