import { useMemo, useState, type FormEvent } from 'react'
import { CircleMinus, LoaderCircle, X } from 'lucide-react'
import {
  type ComputePendingSettlementCorrectionCandidate,
  type CorrectComputeAttemptSettlementBody,
} from './computeSettlementCorrectionApi'
import { formatFen, formatMicros } from './settlementFormatting'
import styles from './ComputeSettlementChallengePage.module.css'

interface Props {
  candidate: ComputePendingSettlementCorrectionCandidate
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: CorrectComputeAttemptSettlementBody) => Promise<void>
}

const CONFIRMATION = '确认纠正'
const MICROS_PER_FEN = 10_000

export default function CorrectSettlementDialog({ candidate, busy, error, onClose, onSubmit }: Props) {
  const original = candidate.settlement
  const originalProvider = original.settlement.amounts.provider_payable_micros
  const originalPlatform = original.settlement.amounts.platform_margin_micros
  const [consumerFenText, setConsumerFenText] = useState('')
  const [providerMicrosText, setProviderMicrosText] = useState('')
  const [platformMicrosText, setPlatformMicrosText] = useState('')
  const [statement, setStatement] = useState('')
  const [evidenceText, setEvidenceText] = useState('')
  const [confirmed, setConfirmed] = useState(false)
  const [confirmation, setConfirmation] = useState('')
  const [idempotencyKey] = useState(createKey)
  const evidenceRefs = useMemo(() => normalizeEvidenceRefs(evidenceText), [evidenceText])
  const consumerFen = parseInteger(consumerFenText)
  const providerMicros = parseInteger(providerMicrosText)
  const platformMicros = parseInteger(platformMicrosText)
  const correctedConsumerMicros = consumerFen === null ? null : consumerFen * MICROS_PER_FEN
  const amountsValid = consumerFen !== null && providerMicros !== null && platformMicros !== null
    && correctedConsumerMicros !== null && Number.isSafeInteger(correctedConsumerMicros)
    && consumerFen >= 0 && consumerFen < original.consumer_charged_fen
    && providerMicros >= 0 && providerMicros <= originalProvider
    && platformMicros >= 0 && platformMicros <= originalPlatform
    && correctedConsumerMicros === providerMicros + platformMicros
  const statementLength = Array.from(statement.trim()).length
  const evidenceValid = evidenceRefs.length <= 16 && evidenceRefs.every((value) => Array.from(value).length <= 512)
  const ready = amountsValid && statementLength >= 8 && statementLength <= 1000 && evidenceValid
    && confirmed && confirmation.trim() === CONFIRMATION
  const refundFen = amountsValid && consumerFen !== null ? original.consumer_charged_fen - consumerFen : null
  const providerReversal = amountsValid && providerMicros !== null ? originalProvider - providerMicros : null
  const platformReversal = amountsValid && platformMicros !== null ? originalPlatform - platformMicros : null

  async function submit(event: FormEvent) {
    event.preventDefault()
    if (!ready || busy || consumerFen === null || providerMicros === null || platformMicros === null) return
    await onSubmit({
      expected_challenge_id: candidate.challenge.challenge_id,
      expected_challenge_event_digest: candidate.challenge.event_digest,
      expected_resolution_id: candidate.resolution.resolution_id,
      expected_resolution_event_digest: candidate.resolution.event_digest,
      expected_settlement_receipt_id: original.settlement.settlement_receipt_id,
      expected_settlement_event_digest: original.event_digest,
      corrected_consumer_charge_fen: consumerFen,
      corrected_provider_payable_micros: providerMicros,
      corrected_platform_margin_micros: platformMicros,
      statement: statement.trim(),
      evidence_refs: evidenceRefs,
      idempotency_key: idempotencyKey,
      confirm_consumer_refund_and_pending_reversal: true,
    })
  }

  return (
    <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}>
      <section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="settlement-correction-title">
        <header><div><CircleMinus size={18} /><h2 id="settlement-correction-title">执行向下结算纠正</h2></div><button type="button" className={styles.iconButton} onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header>
        <form onSubmit={(event) => void submit(event)}>
          {error && <div className={styles.dialogError}>{error}</div>}
          <section className={styles.evidenceTable}>
            <header><strong>accepted 证据链</strong><span>提交时由后端重新审计</span></header>
            <div><span>Challenge</span><code>{candidate.challenge.challenge_id}</code><code>{candidate.challenge.event_digest}</code></div>
            <div><span>Resolution</span><code>{candidate.resolution.resolution_id}</code><code>{candidate.resolution.event_digest}</code></div>
            <div><span>Settlement</span><code>{original.settlement.settlement_receipt_id}</code><code>{original.event_digest}</code></div>
          </section>

          <section className={styles.amountTable}>
            <header><strong>原始金额</strong><span>CNY · 整数合同</span></header>
            <div><span>消费者扣结</span><code>{formatFen(original.consumer_charged_fen)}</code></div>
            <div><span>Provider pending</span><code>{formatMicros(originalProvider)}</code></div>
            <div><span>平台 pending</span><code>{formatMicros(originalPlatform)}</code></div>
          </section>

          <div className={styles.numberGrid}>
            <label><span>纠正后消费者费用（分）</span><input type="number" min="0" step="1" value={consumerFenText} onChange={(event) => setConsumerFenText(event.target.value)} disabled={busy} /></label>
            <label><span>纠正后 Provider（微元）</span><input type="number" min="0" step="1" value={providerMicrosText} onChange={(event) => setProviderMicrosText(event.target.value)} disabled={busy} /></label>
            <label><span>纠正后平台价差（微元）</span><input type="number" min="0" step="1" value={platformMicrosText} onChange={(event) => setPlatformMicrosText(event.target.value)} disabled={busy} /></label>
          </div>

          <div className={styles.preview} data-valid={amountsValid ? 'true' : 'false'}>
            <span>消费者退款 <b>{refundFen === null ? '待输入' : formatFen(refundFen)}</b></span>
            <span>Provider 冲减 <b>{providerReversal === null ? '待输入' : formatMicros(providerReversal)}</b></span>
            <span>平台冲减 <b>{platformReversal === null ? '待输入' : formatMicros(platformReversal)}</b></span>
          </div>

          <label className={styles.textField}><span>纠正说明 <small>{statementLength}/1000</small></span><textarea value={statement} onChange={(event) => setStatement(event.target.value)} maxLength={1000} rows={4} disabled={busy} /></label>
          <label className={styles.textField}><span>证据引用 <small>{evidenceRefs.length}/16</small></span><textarea value={evidenceText} onChange={(event) => setEvidenceText(event.target.value)} rows={4} disabled={busy} /></label>
          <div className={styles.boundary}><b>资金影响</b><span>确认后会向消费者平台内余额追加退款，并从 Provider 与平台 pending 原子冲减对应金额。原 v195/v196/v197 记录保持不变，外部付款效果仍为 none。</span></div>
          <label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我已核对整数金额、守恒关系和证据，确认执行不可变 v199 纠正。</span></label>
          <label className={styles.confirmationField}><span>输入“{CONFIRMATION}”继续</span><input value={confirmation} onChange={(event) => setConfirmation(event.target.value)} disabled={busy} autoComplete="off" /></label>
          <footer><button type="button" onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.primary} disabled={!ready || busy}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在纠正' : '确认退款并冲减'}</button></footer>
        </form>
      </section>
    </div>
  )
}

function parseInteger(value: string) {
  if (!/^\d+$/.test(value.trim())) return null
  const parsed = Number(value)
  return Number.isSafeInteger(parsed) ? parsed : null
}

function normalizeEvidenceRefs(value: string) {
  return [...new Set(value.split(/\r?\n/).map((item) => item.trim()).filter(Boolean))]
}

function createKey() { const nonce = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`; return `pc-compute-settlement-correction:${nonce}` }
