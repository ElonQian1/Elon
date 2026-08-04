import { useState, type FormEvent } from 'react'
import { FileSignature, LoaderCircle, X } from 'lucide-react'
import {
  type ComputePendingExecutionReceiptCandidate,
  type IssueComputeAttemptExecutionReceiptBody,
} from './computeExecutionReceiptApi'
import styles from './ComputeExecutionReceiptPage.module.css'

interface Props {
  candidate: ComputePendingExecutionReceiptCandidate
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: IssueComputeAttemptExecutionReceiptBody) => Promise<void>
}

export default function IssueExecutionReceiptDialog({ candidate, busy, error, onClose, onSubmit }: Props) {
  const terminal = candidate.terminal_candidate
  const verification = candidate.verification_decision
  const [idempotencyKey] = useState(createKey)
  const [confirmed, setConfirmed] = useState(false)

  async function submit(event: FormEvent) {
    event.preventDefault()
    if (!confirmed || busy) return
    await onSubmit({
      expected_verification_decision_id: verification.verification_decision_id,
      expected_verification_event_digest: verification.event_digest,
      idempotency_key: idempotencyKey,
      confirm_execution_receipt_only: true,
    })
  }

  return (
    <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}>
      <section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="execution-receipt-title">
        <header><div><FileSignature size={18} /><h2 id="execution-receipt-title">签发 Execution Receipt</h2></div><button type="button" className={styles.iconButton} onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header>
        <form onSubmit={(event) => void submit(event)}>
          {error && <div className={styles.dialogError}>{error}</div>}
          <section className={styles.evidenceTable}>
            <header><strong>固定执行事实</strong><span>v193 将重新审计全部来源</span></header>
            <div><span>Verification</span><code>{verification.verification_decision_id}</code></div>
            <div><span>Job / Provider</span><code>{verification.job_id} / {verification.provider_id}</code></div>
            <div><span>执行结果</span><code>{terminal.outcome}</code></div>
            <div><span>输出摘要</span><code>{terminal.output_digest ?? '无输出摘要'}</code></div>
          </section>

          <section className={styles.meterTable}>
            <header><strong>签发用量</strong><span>verified / compensable</span></header>
            {verification.verified_usage.map((reading) => {
              const compensable = verification.compensable_usage.find((value) => value.meter === reading.meter)?.quantity ?? 0
              return <div key={reading.meter}><span>{reading.meter}</span><code>{reading.quantity}</code><code>{compensable}</code></div>
            })}
          </section>

          <section className={styles.artifacts}>
            <header><strong>输出工件引用</strong><span>{terminal.result_artifacts.length}</span></header>
            {!terminal.result_artifacts.length && <p>该候选没有输出工件。</p>}
            {terminal.result_artifacts.map((artifact) => <div key={artifact.artifact_id}><span>{artifact.media_type} · {artifact.size_bytes} B</span><code>{artifact.location_ref}</code><code>{artifact.digest}</code></div>)}
          </section>

          <div className={styles.boundary}>签发只固化执行身份、工件、用量和证明，不下载或验证工件正文，也不推进任务、容量或资金状态。</div>
          <label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我已核对 accepted Verification、输出引用与用量，并确认只签发不可变 Execution Receipt。</span></label>
          <code className={styles.eventDigest}>{verification.event_digest}</code>
          <footer><button type="button" onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.primary} disabled={!confirmed || busy}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在签发' : '确认签发'}</button></footer>
        </form>
      </section>
    </div>
  )
}

function createKey() { const nonce = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`; return `pc-compute-execution-receipt:${nonce}` }
