import { useMemo, useState, type FormEvent } from 'react'
import { LoaderCircle, Scale, Send, X } from 'lucide-react'
import {
  type ComputePendingSettlementChallengeCandidate,
  type ComputeSettlementChallengeReasonCode,
  type OpenComputeSettlementChallengeBody,
} from './computeSettlementChallengeApi'
import { formatFen, formatMicros } from './settlementFormatting'
import styles from './ComputeSettlementChallengePage.module.css'

interface Props {
  candidate: ComputePendingSettlementChallengeCandidate
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: OpenComputeSettlementChallengeBody) => Promise<void>
}

const REASONS: Array<{ value: ComputeSettlementChallengeReasonCode; label: string }> = [
  { value: 'amount', label: '结算金额' },
  { value: 'metering', label: '用量计量' },
  { value: 'price_snapshot', label: '价格快照' },
  { value: 'execution_evidence', label: '执行证据' },
  { value: 'provider_identity', label: 'Provider 身份' },
  { value: 'other', label: '其他问题' },
]

export default function OpenSettlementChallengeDialog({ candidate, busy, error, onClose, onSubmit }: Props) {
  const receipt = candidate.settlement
  const [reasonCode, setReasonCode] = useState<ComputeSettlementChallengeReasonCode>('amount')
  const [summary, setSummary] = useState('')
  const [evidenceText, setEvidenceText] = useState('')
  const [confirmed, setConfirmed] = useState(false)
  const [idempotencyKey] = useState(createKey)
  const evidenceRefs = useMemo(() => normalizeEvidenceRefs(evidenceText), [evidenceText])
  const summaryLength = Array.from(summary.trim()).length
  const evidenceValid = evidenceRefs.length <= 16 && evidenceRefs.every((value) => Array.from(value).length <= 512)
  const ready = confirmed && summaryLength >= 8 && summaryLength <= 1000 && evidenceValid

  async function submit(event: FormEvent) {
    event.preventDefault()
    if (!ready || busy) return
    await onSubmit({
      expected_settlement_receipt_id: receipt.settlement.settlement_receipt_id,
      expected_settlement_event_digest: receipt.event_digest,
      expected_posting_id: receipt.posting_id,
      expected_posting_digest: receipt.posting_digest,
      reason_code: reasonCode,
      summary: summary.trim(),
      evidence_refs: evidenceRefs,
      idempotency_key: idempotencyKey,
      confirm_pending_release_block: true,
    })
  }

  return (
    <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}>
      <section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="settlement-challenge-title">
        <header><div><Scale size={18} /><h2 id="settlement-challenge-title">提出结算申诉</h2></div><button type="button" className={styles.iconButton} onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header>
        <form onSubmit={(event) => void submit(event)}>
          {error && <div className={styles.dialogError}>{error}</div>}

          <section className={styles.evidenceTable}>
            <header><strong>结算绑定</strong><span>提交时由后端重新审计</span></header>
            <div><span>Settlement Receipt</span><code>{receipt.settlement.settlement_receipt_id}</code><code>{receipt.event_digest}</code></div>
            <div><span>Posting</span><code>{receipt.posting_id}</code><code>{receipt.posting_digest}</code></div>
            <div><span>Attempt Lease</span><code>{receipt.lease_id}</code><code>截止 {formatTime(candidate.challenge_deadline)}</code></div>
          </section>

          <section className={styles.amountTable}>
            <header><strong>既有金额</strong><span>CNY · 本次提交不移动余额</span></header>
            <div><span>消费者扣结</span><code>{formatFen(receipt.consumer_charged_fen)}</code></div>
            <div><span>消费者已退</span><code>{formatFen(receipt.consumer_refunded_fen)}</code></div>
            <div><span>Provider pending</span><code>{formatMicros(receipt.settlement.amounts.provider_payable_micros)}</code></div>
            <div><span>平台 pending</span><code>{formatMicros(receipt.settlement.amounts.platform_margin_micros)}</code></div>
          </section>

          <div className={styles.fieldGrid}>
            <label><span>申诉原因</span><select value={reasonCode} onChange={(event) => setReasonCode(event.target.value as ComputeSettlementChallengeReasonCode)} disabled={busy}>{REASONS.map((reason) => <option key={reason.value} value={reason.value}>{reason.label}</option>)}</select></label>
            <label className={styles.summaryField}><span>事实摘要 <small>{summaryLength}/1000</small></span><textarea value={summary} onChange={(event) => setSummary(event.target.value)} maxLength={1000} rows={4} disabled={busy} /></label>
            <label className={styles.summaryField}><span>证据引用 <small>{evidenceRefs.length}/16</small></span><textarea value={evidenceText} onChange={(event) => setEvidenceText(event.target.value)} rows={4} disabled={busy} /></label>
          </div>

          <div className={styles.boundary}><b>状态影响</b><span>申诉登记后，Provider 与平台 pending 余额保持不变，该笔收益的后续释放会被阻断，直至管理员决议和必要纠正完成。</span></div>
          <label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我确认提交的是该笔结算的首份不可变申诉，并理解它不会立即退款或撤销原结算。</span></label>
          <footer><button type="button" onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.primary} disabled={!ready || busy}>{busy ? <LoaderCircle size={15} className={styles.spinning} /> : <Send size={15} />}{busy ? '正在提交' : '提交申诉'}</button></footer>
        </form>
      </section>
    </div>
  )
}

function normalizeEvidenceRefs(value: string) {
  return [...new Set(value.split(/\r?\n/).map((item) => item.trim()).filter(Boolean))]
}

function createKey() { const nonce = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`; return `pc-compute-settlement-challenge:${nonce}` }
function formatTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN', { hour12: false }) }
