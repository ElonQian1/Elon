import { useState, type FormEvent } from 'react'
import { LoaderCircle, Undo2, X } from 'lucide-react'
import {
  type ComputeSettlementChallengeReceipt,
  type WithdrawComputeSettlementChallengeBody,
} from './computeSettlementChallengeResolutionApi'
import styles from './ComputeSettlementChallengePage.module.css'

interface Props {
  challenge: ComputeSettlementChallengeReceipt
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: WithdrawComputeSettlementChallengeBody) => Promise<void>
}

export default function WithdrawSettlementChallengeDialog({ challenge, busy, error, onClose, onSubmit }: Props) {
  const [statement, setStatement] = useState('')
  const [confirmed, setConfirmed] = useState(false)
  const [idempotencyKey] = useState(createKey)
  const statementLength = Array.from(statement.trim()).length
  const ready = confirmed && statementLength >= 8 && statementLength <= 1000

  async function submit(event: FormEvent) {
    event.preventDefault()
    if (!ready || busy) return
    await onSubmit({
      expected_challenge_id: challenge.challenge_id,
      expected_challenge_event_digest: challenge.event_digest,
      statement: statement.trim(),
      idempotency_key: idempotencyKey,
      confirm_balances_unchanged: true,
    })
  }

  return (
    <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}>
      <section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="settlement-challenge-withdraw-title">
        <header><div><Undo2 size={18} /><h2 id="settlement-challenge-withdraw-title">撤回结算申诉</h2></div><button type="button" className={styles.iconButton} onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header>
        <form onSubmit={(event) => void submit(event)}>
          {error && <div className={styles.dialogError}>{error}</div>}
          <section className={styles.evidenceTable}>
            <header><strong>申诉绑定</strong><span>撤回后不可改为其他终态</span></header>
            <div><span>Challenge</span><code>{challenge.challenge_id}</code><code>{challenge.event_digest}</code></div>
            <div><span>Settlement</span><code>{challenge.settlement_receipt_id}</code><code>{challenge.settlement_event_digest}</code></div>
            <div><span>原因</span><code>{challenge.reason_code}</code><code>{challenge.summary}</code></div>
          </section>
          <label className={styles.textField}><span>撤回说明 <small>{statementLength}/1000</small></span><textarea value={statement} onChange={(event) => setStatement(event.target.value)} maxLength={1000} rows={5} disabled={busy} /></label>
          <div className={styles.boundary}><b>状态影响</b><span>撤回会生成不可覆盖的 withdrawn 决议并解除挑战门卫，但不会立即释放 pending 收益、修改余额或证明外部付款。</span></div>
          <label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我确认撤回该申诉，并理解消费者、Provider 与平台余额均保持不变。</span></label>
          <footer><button type="button" onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.primary} disabled={!ready || busy}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在撤回' : '确认撤回'}</button></footer>
        </form>
      </section>
    </div>
  )
}

function createKey() { const nonce = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`; return `pc-compute-settlement-challenge-withdraw:${nonce}` }
