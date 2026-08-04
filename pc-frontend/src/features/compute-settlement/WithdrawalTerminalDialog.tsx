import { useEffect, useMemo, useState, type FormEvent } from 'react'
import { BadgeCheck, Ban, LoaderCircle, X } from 'lucide-react'
import {
  type SettlementWithdrawalRequest,
  type TerminalizeSettlementWithdrawalBody,
  type WithdrawalEvidenceKind,
  type WithdrawalTerminalAction,
} from './computeSettlementApi'
import styles from './WithdrawalTerminalDialog.module.css'

interface WithdrawalTerminalDialogProps {
  request: SettlementWithdrawalRequest
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: TerminalizeSettlementWithdrawalBody) => Promise<void>
}

const EVIDENCE_KINDS: Array<{ value: WithdrawalEvidenceKind; label: string }> = [
  { value: 'bank_receipt', label: '银行回单' },
  { value: 'payment_provider_receipt', label: '支付服务回执' },
  { value: 'sui_transaction_digest', label: 'Sui 交易摘要' },
  { value: 'other_receipt', label: '其他付款凭证' },
]

export default function WithdrawalTerminalDialog({
  request,
  busy,
  error,
  onClose,
  onSubmit,
}: WithdrawalTerminalDialogProps) {
  const [action, setAction] = useState<WithdrawalTerminalAction>('rejected')
  const [reasonCode, setReasonCode] = useState('manual_review_rejected')
  const [reasonDetail, setReasonDetail] = useState('')
  const [evidenceKind, setEvidenceKind] = useState<WithdrawalEvidenceKind>('bank_receipt')
  const [evidenceRef, setEvidenceRef] = useState('')
  const [evidenceDigest, setEvidenceDigest] = useState('')
  const [confirmedScope, setConfirmedScope] = useState(false)
  const [confirmedPaid, setConfirmedPaid] = useState(false)
  const [confirmedNoSecret, setConfirmedNoSecret] = useState(false)
  const isPaid = action === 'external_paid_attested'
  const normalizedDigest = evidenceDigest.trim().toLowerCase()

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && !busy) onClose()
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [busy, onClose])

  const canSubmit = useMemo(() => {
    if (!reasonCode.trim() || !confirmedScope || busy) return false
    if (!isPaid) return true
    return Boolean(
      evidenceRef.trim()
      && /^[a-f0-9]{64}$/.test(normalizedDigest)
      && confirmedPaid
      && confirmedNoSecret,
    )
  }, [busy, confirmedNoSecret, confirmedPaid, confirmedScope, evidenceRef, isPaid, normalizedDigest, reasonCode])

  function chooseAction(nextAction: WithdrawalTerminalAction) {
    setAction(nextAction)
    setReasonCode(nextAction === 'rejected' ? 'manual_review_rejected' : 'external_payment_verified')
    setConfirmedScope(false)
    setConfirmedPaid(false)
    setConfirmedNoSecret(false)
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!canSubmit) return
    await onSubmit({
      expected_withdrawal_event_digest: request.event_digest,
      expected_request_posting_id: request.request_posting_id,
      expected_request_posting_digest: request.request_posting_digest,
      action,
      reason_code: reasonCode.trim(),
      reason_detail: reasonDetail.trim() || null,
      external_evidence_kind: isPaid ? evidenceKind : null,
      external_evidence_ref: isPaid ? evidenceRef.trim() : null,
      external_evidence_digest: isPaid ? normalizedDigest : null,
      idempotency_key: `pc-withdrawal-terminal:${request.event_digest}:${action}`,
      confirm_refund_or_attestation_only: confirmedScope,
      confirm_external_payment_already_completed: isPaid && confirmedPaid,
      confirm_evidence_ref_contains_no_secret: isPaid && confirmedNoSecret,
    })
  }

  return (
    <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}>
      <section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="withdrawal-terminal-title">
        <header className={styles.header}>
          <div>
            <span>提款终态</span>
            <h2 id="withdrawal-terminal-title">处理 Provider 提款</h2>
          </div>
          <button type="button" className={styles.iconButton} onClick={onClose} disabled={busy} aria-label="关闭" title="关闭">
            <X size={18} aria-hidden="true" />
          </button>
        </header>

        <div className={styles.summary}>
          <div><span>Provider</span><strong>{shortId(request.provider_id)}</strong></div>
          <div><span>提款金额</span><strong>{formatCny(request.amount_micros)}</strong></div>
          <div><span>提款编号</span><strong>{shortId(request.withdrawal_id)}</strong></div>
        </div>

        <form onSubmit={(event) => void submit(event)}>
          {error && <div className={styles.error} role="alert">{error}</div>}
          <div className={styles.segmented} aria-label="终态操作">
            <button type="button" data-active={action === 'rejected'} onClick={() => chooseAction('rejected')}>
              <Ban size={15} aria-hidden="true" />拒绝并退回余额
            </button>
            <button type="button" data-active={action === 'external_paid_attested'} onClick={() => chooseAction('external_paid_attested')}>
              <BadgeCheck size={15} aria-hidden="true" />登记外部已付款
            </button>
          </div>

          <label className={styles.field}>
            <span>原因代码</span>
            <input value={reasonCode} onChange={(event) => setReasonCode(event.target.value)} maxLength={120} required />
          </label>
          <label className={styles.field}>
            <span>处理说明</span>
            <textarea value={reasonDetail} onChange={(event) => setReasonDetail(event.target.value)} maxLength={1000} rows={3} placeholder="记录人工核验结果或补充说明" />
          </label>

          {isPaid && (
            <div className={styles.evidenceFields}>
              <label className={styles.field}>
                <span>证据类型</span>
                <select value={evidenceKind} onChange={(event) => setEvidenceKind(event.target.value as WithdrawalEvidenceKind)}>
                  {EVIDENCE_KINDS.map((kind) => <option value={kind.value} key={kind.value}>{kind.label}</option>)}
                </select>
              </label>
              <label className={styles.field}>
                <span>证据引用</span>
                <input value={evidenceRef} onChange={(event) => setEvidenceRef(event.target.value)} maxLength={1000} placeholder="回单号、公开交易摘要或受控凭证引用" required />
              </label>
              <label className={styles.field}>
                <span>证据 SHA-256 摘要</span>
                <input value={evidenceDigest} onChange={(event) => setEvidenceDigest(event.target.value)} maxLength={64} spellCheck={false} placeholder="64 位十六进制摘要" required />
              </label>
            </div>
          )}

          <div className={styles.confirmations}>
            <label>
              <input type="checkbox" checked={confirmedScope} onChange={(event) => setConfirmedScope(event.target.checked)} />
              <span>{isPaid ? '我确认这里只登记外部付款证明，不会再次付款' : '我确认只执行内部余额退回，不会发起外部付款'}</span>
            </label>
            {isPaid && (
              <>
                <label><input type="checkbox" checked={confirmedPaid} onChange={(event) => setConfirmedPaid(event.target.checked)} /><span>我确认这笔外部付款已经完成</span></label>
                <label><input type="checkbox" checked={confirmedNoSecret} onChange={(event) => setConfirmedNoSecret(event.target.checked)} /><span>我确认引用中不含密码、私钥或助记词</span></label>
              </>
            )}
          </div>

          <footer className={styles.footer}>
            <button type="button" className={styles.cancelButton} onClick={onClose} disabled={busy}>取消</button>
            <button type="submit" className={isPaid ? styles.paidButton : styles.rejectButton} disabled={!canSubmit}>
              {busy && <LoaderCircle size={15} className={styles.spinning} aria-hidden="true" />}
              {busy ? '正在提交' : isPaid ? '确认登记' : '确认拒绝'}
            </button>
          </footer>
        </form>
      </section>
    </div>
  )
}

function shortId(value: string) {
  return value.length <= 24 ? value : `${value.slice(0, 12)}…${value.slice(-8)}`
}

function formatCny(micros: number) {
  return new Intl.NumberFormat('zh-CN', { style: 'currency', currency: 'CNY' }).format(micros / 1_000_000)
}
