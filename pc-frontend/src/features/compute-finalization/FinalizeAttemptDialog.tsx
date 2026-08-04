import { useState, type FormEvent } from 'react'
import { LoaderCircle, LockKeyhole, X } from 'lucide-react'
import {
  type ComputePendingAttemptFinalizationCandidate,
  type FinalizeComputeAttemptBody,
} from './computeAttemptFinalizationApi'
import styles from './ComputeAttemptFinalizationPage.module.css'

interface Props {
  candidate: ComputePendingAttemptFinalizationCandidate
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: FinalizeComputeAttemptBody) => Promise<void>
}

const CONFIRMATION = '确认收口'

export default function FinalizeAttemptDialog({ candidate, busy, error, onClose, onSubmit }: Props) {
  const receipt = candidate.execution_receipt.receipt
  const [idempotencyKey] = useState(createKey)
  const [confirmed, setConfirmed] = useState(false)
  const [confirmation, setConfirmation] = useState('')
  const ready = confirmed && confirmation.trim() === CONFIRMATION

  async function submit(event: FormEvent) {
    event.preventDefault()
    if (!ready || busy) return
    await onSubmit({
      expected_execution_receipt_id: receipt.receipt_id,
      expected_execution_receipt_digest: receipt.receipt_digest,
      expected_lease_revision: candidate.expected_lease.revision,
      expected_lease_digest: candidate.expected_lease.digest,
      expected_fencing_generation: candidate.expected_fencing_generation,
      expected_job_revision: candidate.expected_job.job_revision,
      expected_job_digest: candidate.expected_job.job_digest,
      expected_reservation_revision: candidate.expected_reservation.revision,
      expected_reservation_digest: candidate.expected_reservation.digest,
      expected_claim_revision: candidate.expected_claim.claim_revision,
      expected_claim_digest: candidate.expected_claim.claim_digest,
      idempotency_key: idempotencyKey,
      confirm_trusted_terminal_and_capacity: true,
    })
  }

  return (
    <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}>
      <section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="attempt-finalization-title">
        <header><div><LockKeyhole size={18} /><h2 id="attempt-finalization-title">应用可信终态</h2></div><button type="button" className={styles.iconButton} onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header>
        <form onSubmit={(event) => void submit(event)}>
          {error && <div className={styles.dialogError}>{error}</div>}

          <section className={styles.evidenceTable}>
            <header><strong>精确状态绑定</strong><span>提交时由后端再次核对</span></header>
            <div><span>Execution Receipt</span><code>{receipt.receipt_id}</code><code>{receipt.receipt_digest}</code></div>
            <div><span>Lease</span><code>r{candidate.expected_lease.revision} · g{candidate.expected_fencing_generation}</code><code>{candidate.expected_lease.digest}</code></div>
            <div><span>Job</span><code>r{candidate.expected_job.job_revision} · {candidate.expected_job.job_id}</code><code>{candidate.expected_job.job_digest}</code></div>
            <div><span>Reservation</span><code>r{candidate.expected_reservation.revision} · {receipt.reservation_id}</code><code>{candidate.expected_reservation.digest}</code></div>
            <div><span>Capacity Claim</span><code>r{candidate.expected_claim.claim_revision} · {candidate.expected_claim.claim_id}</code><code>{candidate.expected_claim.claim_digest}</code></div>
          </section>

          <section className={styles.impactTable}>
            <header><strong>将要发生的变化</strong><span>原子写入</span></header>
            <div><span>Attempt Lease</span><code>running</code><code>terminal</code></div>
            <div><span>Job</span><code>running</code><code>verification_pending</code></div>
            <div><span>Reservation</span><code>active</code><code>consumed</code></div>
            <div><span>Capacity Claim</span><code>active</code><code>消费用量并归还余量</code></div>
          </section>

          <section className={styles.meterTable}>
            <header><strong>可补偿用量</strong><span>{candidate.compensable_usage.length} 个计量项</span></header>
            {!candidate.compensable_usage.length && <p>本次没有可补偿用量，剩余容量仍按规则归还。</p>}
            {candidate.compensable_usage.map((reading) => <div key={`${reading.meter}:${reading.reading_digest}`}><span>{reading.meter}</span><code>{reading.quantity}</code><code>{reading.source_kind}</code></div>)}
          </section>

          <div className={styles.boundary}><b>资金边界</b><span>本操作不会扣除消费者预授权、不会增加 Provider 可提现收益，也不会生成资金结算；结算状态仍为 pending。</span></div>
          <label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我已核对回执、版本和容量影响，确认应用不可逆的可信终态。</span></label>
          <label className={styles.confirmationField}><span>输入“{CONFIRMATION}”继续</span><input value={confirmation} onChange={(event) => setConfirmation(event.target.value)} disabled={busy} autoComplete="off" /></label>
          <footer><button type="button" onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.danger} disabled={!ready || busy}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在收口' : '应用可信终态'}</button></footer>
        </form>
      </section>
    </div>
  )
}

function createKey() { const nonce = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`; return `pc-compute-trusted-finalization:${nonce}` }
