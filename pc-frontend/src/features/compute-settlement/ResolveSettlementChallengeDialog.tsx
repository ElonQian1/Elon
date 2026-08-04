import { useState, type FormEvent } from 'react'
import { Gavel, LoaderCircle, X } from 'lucide-react'
import {
  type ComputeSettlementChallengeReceipt,
  type PlatformSettlementChallengeDecision,
  type ResolveComputeSettlementChallengeBody,
} from './computeSettlementChallengeResolutionApi'
import styles from './ComputeSettlementChallengePage.module.css'

interface Props {
  challenge: ComputeSettlementChallengeReceipt
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: ResolveComputeSettlementChallengeBody) => Promise<void>
}

export default function ResolveSettlementChallengeDialog({ challenge, busy, error, onClose, onSubmit }: Props) {
  const [decision, setDecision] = useState<PlatformSettlementChallengeDecision | ''>('')
  const [statement, setStatement] = useState('')
  const [confirmed, setConfirmed] = useState(false)
  const [idempotencyKey] = useState(createKey)
  const statementLength = Array.from(statement.trim()).length
  const ready = decision !== '' && confirmed && statementLength >= 8 && statementLength <= 1000

  async function submit(event: FormEvent) {
    event.preventDefault()
    if (!ready || busy || decision === '') return
    await onSubmit({
      expected_challenge_id: challenge.challenge_id,
      expected_challenge_event_digest: challenge.event_digest,
      decision,
      statement: statement.trim(),
      idempotency_key: idempotencyKey,
      confirm_no_money_movement: true,
    })
  }

  return (
    <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}>
      <section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="settlement-challenge-resolution-title">
        <header><div><Gavel size={18} /><h2 id="settlement-challenge-resolution-title">裁决结算申诉</h2></div><button type="button" className={styles.iconButton} onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header>
        <form onSubmit={(event) => void submit(event)}>
          {error && <div className={styles.dialogError}>{error}</div>}
          <section className={styles.evidenceTable}>
            <header><strong>申诉证据</strong><span>裁决提交时重新审计</span></header>
            <div><span>Challenge</span><code>{challenge.challenge_id}</code><code>{challenge.event_digest}</code></div>
            <div><span>Settlement</span><code>{challenge.settlement_receipt_id}</code><code>{challenge.settlement_event_digest}</code></div>
            <div><span>消费者主张</span><code>{challenge.reason_code}</code><code>{challenge.summary}</code></div>
          </section>
          <div className={styles.fieldGrid}>
            <label><span>裁决结果</span><select value={decision} onChange={(event) => setDecision(event.target.value as PlatformSettlementChallengeDecision | '')} disabled={busy}><option value="">未选择</option><option value="accepted">接受申诉</option><option value="rejected">驳回申诉</option></select></label>
            <label className={styles.summaryField}><span>裁决说明 <small>{statementLength}/1000</small></span><textarea value={statement} onChange={(event) => setStatement(event.target.value)} maxLength={1000} rows={5} disabled={busy} /></label>
          </div>
          <div className={styles.boundary}><b>状态影响</b><span>{decision === 'accepted' ? 'accepted 会继续阻断 pending 释放，并要求独立 v199 纠正后才能解除。' : decision === 'rejected' ? 'rejected 会解除挑战门卫，但不会立即执行 v198 释放。' : '裁决本身不退款、不纠正，也不移动任何余额。'}</span></div>
          <label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我确认该决议只改变挑战门卫，不执行消费者退款、金额纠正或外部付款。</span></label>
          <footer><button type="button" onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.primary} disabled={!ready || busy}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在裁决' : '登记唯一终态'}</button></footer>
        </form>
      </section>
    </div>
  )
}

function createKey() { const nonce = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`; return `pc-compute-settlement-challenge-resolution:${nonce}` }
