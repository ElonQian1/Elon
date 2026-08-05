import { useState, type FormEvent } from 'react'
import { LoaderCircle, RotateCcw, X } from 'lucide-react'
import {
  type ComputeActivationRecoveryPlan,
  type ComputeActivationRecoveryPreflight,
} from './computeActivationAdminApi'
import styles from './ActivationActionDialog.module.css'

interface Props {
  plan: ComputeActivationRecoveryPlan
  preflight: ComputeActivationRecoveryPreflight
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: () => Promise<void>
}

export default function ApplyActivationRecoveryDialog({ plan, preflight, busy, error, onClose, onSubmit }: Props) {
  const [confirmed, setConfirmed] = useState(false)
  async function submit(event: FormEvent<HTMLFormElement>) { event.preventDefault(); if (confirmed && preflight.ready_for_apply && !busy) await onSubmit() }
  return <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}>
    <section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="apply-recovery-title">
      <header className={styles.header}><div><RotateCcw size={18} /><h2 id="apply-recovery-title">应用隔离恢复计划</h2></div><button type="button" onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header>
      <form onSubmit={(event) => void submit(event)}>
        {error && <div className={styles.error} role="alert">{error}</div>}
        <dl className={styles.summary}><div><dt>恢复计划</dt><dd>{plan.recovery_plan_id}</dd></div><div><dt>目标版本</dt><dd>Provider revision {plan.target_provider_policy_revision}</dd></div><div><dt>active Offer</dt><dd>{preflight.active_offer_count}</dd></div><div><dt>计划摘要</dt><dd><code>{plan.plan_digest}</code></dd></div></dl>
        <div className={styles.boundary}>应用会在同一事务中新增精确 Provider 版本并把 Pool 恢复为 active。旧 Offer 必须已退出 active，且不会被自动恢复；节点和资金均无直接效果。</div>
        <label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>当前预检无阻断项。我确认按精确计划摘要执行一次隔离恢复。</span></label>
        <footer className={styles.footer}><button type="button" className={styles.secondary} onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.danger} disabled={!confirmed || !preflight.ready_for_apply || busy}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在应用' : '确认恢复'}</button></footer>
      </form>
    </section>
  </div>
}
