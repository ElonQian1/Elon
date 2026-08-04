import { useState, type FormEvent, type ReactNode } from 'react'
import { CheckCircle2, LoaderCircle, MessageSquareWarning, ShieldX, X } from 'lucide-react'
import { type ComputeActivationEvidenceRequest } from '../compute-supply/computeActivationApi'
import { type ActivationReviewDecision } from './computeActivationAdminApi'
import styles from './ActivationReviewDialog.module.css'

interface Props {
  request: ComputeActivationEvidenceRequest
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (decision: ActivationReviewDecision, note: string | null) => Promise<void>
}

export default function ActivationReviewDialog({ request, busy, error, onClose, onSubmit }: Props) {
  const [decision, setDecision] = useState<ActivationReviewDecision>('approved')
  const [note, setNote] = useState('')
  const [confirmed, setConfirmed] = useState(false)
  const noteRequired = decision !== 'approved'
  const canSubmit = Boolean((!noteRequired || note.trim()) && note.trim().length <= 1000 && confirmed && !busy)

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!canSubmit) return
    await onSubmit(decision, note.trim() || null)
  }

  return <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}>
    <section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="activation-review-title">
      <header className={styles.header}><div><span>{shortId(request.request_id)}</span><h2 id="activation-review-title">审核激活证据</h2></div><button type="button" onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header>
      <form onSubmit={(event) => void submit(event)}>
        {error && <div className={styles.error} role="alert">{error}</div>}
        <div className={styles.decisions}>
          <DecisionButton value="approved" current={decision} icon={<CheckCircle2 size={15} />} label="批准" onSelect={setDecision} />
          <DecisionButton value="changes_requested" current={decision} icon={<MessageSquareWarning size={15} />} label="退回补充" onSelect={setDecision} />
          <DecisionButton value="rejected" current={decision} icon={<ShieldX size={15} />} label="拒绝" onSelect={setDecision} />
        </div>
        <label className={styles.field}><span>审核说明{noteRequired ? '（必填）' : '（选填）'}</span><textarea value={note} onChange={(event) => { setNote(event.target.value); setConfirmed(false) }} maxLength={1000} rows={5} placeholder={noteRequired ? '说明需要补充或拒绝的原因' : '记录批准依据或复核说明'} /></label>
        <label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我已核对当前申请摘要和预检结果，并确认提交此审核决定。</span></label>
        <div className={styles.boundary}>批准只改变申请状态，不会激活 Provider/Pool；后续仍需准备不可变计划、再次预检并显式应用。</div>
        <footer className={styles.footer}><button type="button" className={styles.cancelButton} onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.submitButton} disabled={!canSubmit}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在审核' : '提交决定'}</button></footer>
      </form>
    </section>
  </div>
}

function DecisionButton({ value, current, icon, label, onSelect }: { value: ActivationReviewDecision; current: ActivationReviewDecision; icon: ReactNode; label: string; onSelect: (value: ActivationReviewDecision) => void }) {
  return <button type="button" data-active={value === current} onClick={() => onSelect(value)}>{icon}{label}</button>
}

function shortId(value: string) { return value.length <= 28 ? value : `${value.slice(0, 14)}…${value.slice(-8)}` }
