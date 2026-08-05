import { useState, type FormEvent } from 'react'
import { LoaderCircle, UserRoundCheck, X } from 'lucide-react'
import { type ComputeActivationPlan } from './computeActivationAdminApi'
import styles from './ActivationActionDialog.module.css'

interface Props {
  plan: ComputeActivationPlan
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (note: string | null) => Promise<void>
}

export default function ReviewActivationPlanDialog({ plan, busy, error, onClose, onSubmit }: Props) {
  const [note, setNote] = useState('')
  const [confirmed, setConfirmed] = useState(false)
  const canSubmit = confirmed && note.trim().length <= 1000 && !busy

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (canSubmit) await onSubmit(note.trim() || null)
  }

  return <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}>
    <section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="review-plan-title">
      <header className={styles.header}><div><UserRoundCheck size={18} /><h2 id="review-plan-title">第二人复核激活计划</h2></div><button type="button" onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header>
      <form onSubmit={(event) => void submit(event)}>
        {error && <div className={styles.error} role="alert">{error}</div>}
        <dl className={styles.summary}><div><dt>计划</dt><dd>{plan.plan_id}</dd></div><div><dt>准备人</dt><dd>{plan.prepared_by_user_id}</dd></div><div><dt>计划摘要</dt><dd><code>{plan.plan_digest}</code></dd></div></dl>
        <label className={styles.reason}><span>复核说明（选填）</span><textarea value={note} onChange={(event) => { setNote(event.target.value); setConfirmed(false) }} maxLength={1000} rows={4} placeholder="记录核对依据、工单或异常说明" /></label>
        <div className={styles.boundary}>复核会生成绑定当前计划摘要的不可变回执，不会激活 Provider、Pool，不会发布 Offer，也不会产生资金操作。</div>
        <label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我不是该计划的准备人，并已独立核对目标版本、路由、硬件证据和计划摘要。</span></label>
        <footer className={styles.footer}><button type="button" className={styles.secondary} onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.primary} disabled={!canSubmit}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在复核' : '确认复核'}</button></footer>
      </form>
    </section>
  </div>
}
