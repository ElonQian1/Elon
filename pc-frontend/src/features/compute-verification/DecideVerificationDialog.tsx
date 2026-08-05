import { useMemo, useState, type FormEvent } from 'react'
import { LoaderCircle, Scale, X } from 'lucide-react'
import {
  type ComputePendingAttemptVerificationCandidate,
  type ComputeVerificationDecision,
  type DecideComputeAttemptVerificationBody,
} from './computeVerificationApi'
import styles from './ComputeVerificationPage.module.css'

interface Props {
  candidate: ComputePendingAttemptVerificationCandidate
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: DecideComputeAttemptVerificationBody) => Promise<void>
}

export default function DecideVerificationDialog({ candidate, busy, error, onClose, onSubmit }: Props) {
  const terminal = candidate.terminal_candidate
  const review = candidate.consumer_review
  const observation = candidate.platform_observation
  const canAccept = review.decision === 'accepted'
    && terminal.outcome === observation.observed_outcome
  const [decision, setDecision] = useState<ComputeVerificationDecision>(canAccept ? 'accepted' : 'disputed')
  const [reasonText, setReasonText] = useState(canAccept ? 'evidence_chain_consistent' : 'evidence_chain_requires_review')
  const [decisionRef, setDecisionRef] = useState('')
  const [idempotencyKey] = useState(createKey)
  const [confirmed, setConfirmed] = useState(false)
  const reasonCodes = useMemo(() => normalizeReasonCodes(reasonText), [reasonText])
  const reasonCodesValid = reasonCodes.length > 0 && reasonCodes.length <= 16 && reasonCodes.every((value) => value.length <= 100)
  const preview = useMemo(() => buildUsagePreview(candidate, decision), [candidate, decision])
  const valid = reasonCodesValid && decisionRef.trim().length > 0 && decisionRef.trim().length <= 1000 && confirmed && !busy

  async function submit(event: FormEvent) {
    event.preventDefault()
    if (!valid || (decision === 'accepted' && !canAccept)) return
    await onSubmit({
      expected_terminal_candidate_id: terminal.terminal_candidate_id,
      expected_terminal_candidate_event_digest: terminal.event_digest,
      expected_consumer_review_id: review.consumer_review_id,
      expected_consumer_review_event_digest: review.event_digest,
      expected_platform_observation_id: observation.platform_observation_id,
      expected_platform_observation_event_digest: observation.event_digest,
      policy_id: 'conservative_min_v1',
      policy_version: 1,
      decision,
      reason_codes: reasonCodes,
      decision_ref: decisionRef.trim(),
      idempotency_key: idempotencyKey,
      confirm_no_state_or_settlement_effect: true,
    })
  }

  return (
    <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}>
      <section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="verification-title">
        <header><div><Scale size={18} /><h2 id="verification-title">形成 Verification 决定</h2></div><button type="button" className={styles.iconButton} onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header>
        <form onSubmit={(event) => void submit(event)}>
          {error && <div className={styles.dialogError}>{error}</div>}
          <div className={styles.decisionTabs} aria-label="Verification 决定">
            {(['accepted', 'rejected', 'disputed'] as ComputeVerificationDecision[]).map((value) => (
              <button
                type="button"
                data-active={decision === value}
                key={value}
                disabled={value === 'accepted' && !canAccept}
                title={value === 'accepted' && !canAccept ? '消费者未接受，或 Provider 与平台结果不一致' : decisionLabel(value)}
                onClick={() => { setDecision(value); setConfirmed(false) }}
              >{decisionLabel(value)}</button>
            ))}
          </div>

          <section className={styles.evidenceTable}>
            <header><strong>三方证据</strong><span>v189 / v190 / v191</span></header>
            <div><span>Provider 结果</span><b>{terminal.outcome}</b></div>
            <div><span>消费者审核</span><b>{review.decision} · {review.reason_code}</b></div>
            <div><span>平台观测</span><b>{observation.observed_outcome} · {observation.observation_source}</b></div>
          </section>

          <section className={styles.meterTable}>
            <header><strong>保守计量预览</strong><span>声明 / 观测 / 验证 / 可补偿</span></header>
            {preview.map((line) => <div key={line.meter}><span>{line.meter}</span><code>{line.declared}</code><code>{line.observed}</code><code>{line.verified}</code><code>{line.compensable}</code></div>)}
          </section>

          {!canAccept && <div className={styles.validation}>当前证据链不满足 accepted 条件，只能登记拒绝或争议。</div>}
          <div className={styles.formGrid}>
            <label><span>原因码（每行一项）</span><textarea rows={3} value={reasonText} onChange={(event) => { setReasonText(event.target.value); setConfirmed(false) }} /></label>
            <label><span>外部决定引用</span><input value={decisionRef} maxLength={1000} onChange={(event) => { setDecisionRef(event.target.value); setConfirmed(false) }} placeholder="verification://review/..." /></label>
          </div>
          {!reasonCodesValid && <div className={styles.validation}>填写 1 至 16 个唯一原因码，每项最多 100 字符。</div>}
          <div className={styles.boundary}>该决定只记录保守验证量，不签发执行回执，不推进任务、容量或资金状态。</div>
          <label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我已核对三方证据与计量预览，并确认本操作没有状态或结算效果。</span></label>
          <code className={styles.eventDigest}>{observation.event_digest}</code>
          <footer><button type="button" onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.primary} disabled={!valid || (decision === 'accepted' && !canAccept)}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在保存' : '保存决定'}</button></footer>
        </form>
      </section>
    </div>
  )
}

function normalizeReasonCodes(value: string) {
  return [...new Set(value.split(/\r?\n/).map((item) => item.trim().toLowerCase()).filter(Boolean))].sort()
}

function buildUsagePreview(candidate: ComputePendingAttemptVerificationCandidate, decision: ComputeVerificationDecision) {
  const observed = new Map(candidate.platform_observation.cumulative_observed_usage.map((line) => [line.meter, line.quantity]))
  const reserved = new Map(candidate.provider_usage.reserved_contract.map((line) => [line.meter, line.reserved_quantity]))
  return candidate.provider_usage.cumulative_declared_usage.map((line) => {
    const observedQuantity = observed.get(line.meter) ?? 0
    const verified = decision === 'accepted' ? Math.min(line.quantity, observedQuantity) : 0
    return { meter: line.meter, declared: line.quantity, observed: observedQuantity, verified, compensable: Math.min(verified, reserved.get(line.meter) ?? 0) }
  })
}

function decisionLabel(value: ComputeVerificationDecision) { return ({ accepted: '接受', rejected: '拒绝', disputed: '争议' })[value] }
function createKey() { const nonce = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`; return `pc-compute-verification:${nonce}` }
