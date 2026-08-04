import { useState, type FormEvent } from 'react'
import { Banknote, LoaderCircle, X } from 'lucide-react'
import {
  type ComputePendingAttemptSettlementCandidate,
  type SettleComputeAttemptBody,
} from './computeSettlementIssuanceApi'
import { formatFen, formatMicros } from './settlementFormatting'
import styles from './ComputeSettlementIssuancePage.module.css'

interface Props {
  candidate: ComputePendingAttemptSettlementCandidate
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: SettleComputeAttemptBody) => Promise<void>
}

const CONFIRMATION = '确认扣结'

export default function IssueSettlementReceiptDialog({ candidate, busy, error, onClose, onSubmit }: Props) {
  const finalization = candidate.finalization
  const execution = candidate.execution_receipt.receipt
  const preview = candidate.preview
  const [idempotencyKey] = useState(createKey)
  const [confirmed, setConfirmed] = useState(false)
  const [confirmation, setConfirmation] = useState('')
  const ready = confirmed && confirmation.trim() === CONFIRMATION

  async function submit(event: FormEvent) {
    event.preventDefault()
    if (!ready || busy) return
    await onSubmit({
      expected_finalization_id: finalization.finalization_id,
      expected_finalization_event_digest: finalization.event_digest,
      expected_execution_receipt_id: execution.receipt_id,
      expected_execution_receipt_digest: execution.receipt_digest,
      expected_job_revision: candidate.expected_job.job_revision,
      expected_job_digest: candidate.expected_job.job_digest,
      expected_budget_reservation_id: candidate.expected_budget_reservation_id,
      expected_price_snapshot_id: candidate.price_snapshot.snapshot_id,
      expected_price_snapshot_digest: candidate.price_snapshot.snapshot_digest,
      idempotency_key: idempotencyKey,
      confirm_consumer_capture_and_provider_pending: true,
    })
  }

  return (
    <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}>
      <section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="settlement-issuance-title">
        <header><div><Banknote size={18} /><h2 id="settlement-issuance-title">生成待结算回执</h2></div><button type="button" className={styles.iconButton} onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header>
        <form onSubmit={(event) => void submit(event)}>
          {error && <div className={styles.dialogError}>{error}</div>}

          <section className={styles.evidenceTable}>
            <header><strong>精确资金来源</strong><span>提交时由后端重新审计</span></header>
            <div><span>Finalization</span><code>{finalization.finalization_id}</code><code>{finalization.event_digest}</code></div>
            <div><span>Execution Receipt</span><code>{execution.receipt_id}</code><code>{execution.receipt_digest}</code></div>
            <div><span>Job</span><code>r{candidate.expected_job.job_revision} · {candidate.expected_job.job_id}</code><code>{candidate.expected_job.job_digest}</code></div>
            <div><span>预授权</span><code>{candidate.expected_budget_reservation_id}</code><code>{formatFen(preview.budget_reserved_fen)}</code></div>
            <div><span>Price Snapshot</span><code>{candidate.price_snapshot.snapshot_id}</code><code>{candidate.price_snapshot.snapshot_digest}</code></div>
            <div><span>Provider 账户</span><code>{candidate.provider_account_id}</code><code>pending only</code></div>
          </section>

          <section className={styles.amountTable}>
            <header><strong>金额预览</strong><span>{preview.currency} · 整数金额合同</span></header>
            <div><span>消费者预授权</span><code>{formatFen(preview.budget_reserved_fen)}</code></div>
            <div><span>实际扣结</span><code>{formatFen(preview.consumer_charge_fen)}</code></div>
            <div><span>退回消费者</span><code>{formatFen(preview.consumer_refund_fen)}</code></div>
            <div><span>Provider pending</span><code>{formatMicros(preview.amounts.provider_payable_micros)}</code></div>
            <div><span>平台 pending</span><code>{formatMicros(preview.amounts.platform_margin_micros)}</code></div>
          </section>

          <div className={styles.boundary}><b>资金影响</b><span>确认后将结清消费者平台内预授权、退回未使用金额，并登记 Provider 与平台 pending 余额。pending 不可提现，本操作不会调用银行、支付机构、钱包或 Sui。</span></div>
          <label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我已核对预授权、价格快照、用量与金额，确认生成不可变 v195 Settlement Receipt。</span></label>
          <label className={styles.confirmationField}><span>输入“{CONFIRMATION}”继续</span><input value={confirmation} onChange={(event) => setConfirmation(event.target.value)} disabled={busy} autoComplete="off" /></label>
          <footer><button type="button" onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.danger} disabled={!ready || busy}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在结算' : '确认扣结并生成回执'}</button></footer>
        </form>
      </section>
    </div>
  )
}

function createKey() { const nonce = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`; return `pc-compute-attempt-settlement:${nonce}` }
