import { useMemo, useState, type FormEvent } from 'react'
import { FileSearch, LoaderCircle, X } from 'lucide-react'
import {
  type ComputeAttemptTerminalCandidateReceipt,
  type ComputeConsumerReviewDecision,
  type ReviewComputeAttemptTerminalCandidateBody,
} from './computeConsumerReviewApi'
import styles from './ComputeConsumerReviewPage.module.css'

interface Props {
  candidate: ComputeAttemptTerminalCandidateReceipt
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: ReviewComputeAttemptTerminalCandidateBody) => Promise<void>
}

export default function ReviewTerminalCandidateDialog({ candidate, busy, error, onClose, onSubmit }: Props) {
  const [decision, setDecision] = useState<ComputeConsumerReviewDecision>('accepted')
  const [reasonCode, setReasonCode] = useState(defaultReason('accepted'))
  const [reviewRef, setReviewRef] = useState('')
  const [evidenceText, setEvidenceText] = useState('')
  const [idempotencyKey] = useState(createKey)
  const [confirmed, setConfirmed] = useState(false)
  const evidence = useMemo(() => evidenceText.split(/\r?\n/).map((value) => value.trim()).filter(Boolean), [evidenceText])
  const evidenceUnique = new Set(evidence).size === evidence.length
  const evidenceValid = evidence.length <= 16 && evidenceUnique && evidence.every((value) => value.length <= 1000)
  const needsEvidence = decision !== 'accepted'
  const reasonValid = /^[a-z0-9._-]+$/.test(reasonCode) && reasonCode.length <= 100
  const valid = reviewRef.trim().length > 0 && reviewRef.trim().length <= 1000
    && reasonValid && evidenceValid && (!needsEvidence || evidence.length > 0) && confirmed && !busy

  function choose(next: ComputeConsumerReviewDecision) {
    setDecision(next)
    setReasonCode(defaultReason(next))
    setConfirmed(false)
  }

  async function submit(event: FormEvent) {
    event.preventDefault()
    if (!valid) return
    await onSubmit({
      expected_terminal_candidate_id: candidate.terminal_candidate_id,
      expected_terminal_candidate_event_digest: candidate.event_digest,
      decision,
      reason_code: reasonCode,
      consumer_review_ref: reviewRef.trim(),
      evidence_refs: [...evidence].sort(),
      idempotency_key: idempotencyKey,
      confirm_consumer_attestation_only: true,
    })
  }

  return (
    <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}>
      <section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="consumer-review-title">
        <header>
          <div><FileSearch size={18} /><h2 id="consumer-review-title">审核交付候选</h2></div>
          <button type="button" className={styles.iconButton} onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button>
        </header>
        <form onSubmit={(event) => void submit(event)}>
          {error && <div className={styles.dialogError}>{error}</div>}
          <div className={styles.segmented} aria-label="审核决定">
            {(['accepted', 'rejected', 'disputed'] as ComputeConsumerReviewDecision[]).map((value) => (
              <button type="button" data-active={decision === value} key={value} onClick={() => choose(value)}>{decisionLabel(value)}</button>
            ))}
          </div>
          <div className={styles.bindingFacts}>
            <div><span>Job</span><strong>{candidate.job_id}</strong></div>
            <div><span>Provider 结果</span><strong>{outcomeLabel(candidate.outcome)}</strong></div>
            <div><span>最终用量序号</span><strong>{candidate.final_usage_sequence_no}</strong></div>
            <div><span>工件数量</span><strong>{candidate.result_artifacts.length}</strong></div>
          </div>
          {candidate.result_artifacts.length > 0 && (
            <section className={styles.artifacts}>
              <header><strong>结果工件引用</strong><span>{candidate.result_artifacts.length}</span></header>
              {candidate.result_artifacts.map((artifact) => (
                <div key={artifact.artifact_id}>
                  <strong>{artifact.artifact_id}</strong>
                  <span>{artifact.media_type} · {artifact.size_bytes} bytes</span>
                  <code>{artifact.digest}</code>
                  <small>{artifact.location_ref}</small>
                </div>
              ))}
            </section>
          )}
          <div className={styles.formGrid}>
            <label><span>审核引用</span><input value={reviewRef} maxLength={1000} onChange={(event) => { setReviewRef(event.target.value); setConfirmed(false) }} placeholder="review://consumer/receipt/..." /></label>
            <label><span>原因码</span><input value={reasonCode} maxLength={100} onChange={(event) => { setReasonCode(event.target.value.toLowerCase()); setConfirmed(false) }} /></label>
            <label data-wide="true"><span>证据引用（每行一条，最多 16 条）</span><textarea value={evidenceText} onChange={(event) => { setEvidenceText(event.target.value); setConfirmed(false) }} rows={4} /></label>
          </div>
          {!reasonValid && <div className={styles.validation}>原因码只能使用小写字母、数字、点、下划线和连字符。</div>}
          {!evidenceValid && <div className={styles.validation}>证据引用不能重复，每条最多 1000 字符，总数最多 16 条。</div>}
          {needsEvidence && evidence.length === 0 && <div className={styles.validation}>拒绝或争议必须提供至少一条证据引用。</div>}
          <div className={styles.boundary}>该记录仅是消费者证据，不代表平台验证、付款、退款或最终结算。</div>
          <label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我已核对候选摘要，并确认只登记第一份消费者审核意见。</span></label>
          <code className={styles.eventDigest}>{candidate.event_digest}</code>
          <footer>
            <button type="button" onClick={onClose} disabled={busy}>取消</button>
            <button type="submit" className={styles.primary} disabled={!valid}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在保存' : '保存审核'}</button>
          </footer>
        </form>
      </section>
    </div>
  )
}

function defaultReason(value: ComputeConsumerReviewDecision) { return `consumer.${value}` }
function decisionLabel(value: ComputeConsumerReviewDecision) { return ({ accepted: '接受', rejected: '拒绝', disputed: '争议' })[value] }
function outcomeLabel(value: string) { return ({ succeeded: '已完成', failed: '执行失败', canceled: '已取消' } as Record<string, string>)[value] ?? value }
function createKey() { const nonce = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`; return `pc-compute-consumer-review:${nonce}` }
