import { useState, type FormEvent } from 'react'
import { LoaderCircle, ShieldAlert, X } from 'lucide-react'
import styles from './ActivationActionDialog.module.css'

interface Props {
  title: string
  description: string
  confirmLabel: string
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (reason: string) => Promise<void>
}

export default function LifecycleReasonDialog({ title, description, confirmLabel, busy, error, onClose, onSubmit }: Props) {
  const [reason, setReason] = useState('')
  const [confirmed, setConfirmed] = useState(false)
  const canSubmit = Boolean(reason.trim() && reason.trim().length <= 1000 && confirmed && !busy)
  async function submit(event: FormEvent<HTMLFormElement>) { event.preventDefault(); if (canSubmit) await onSubmit(reason.trim()) }
  return <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}>
    <section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="lifecycle-action-title">
      <header className={styles.header}><div><ShieldAlert size={18} /><h2 id="lifecycle-action-title">{title}</h2></div><button type="button" onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header>
      <form onSubmit={(event) => void submit(event)}>
        {error && <div className={styles.error} role="alert">{error}</div>}
        <div className={styles.boundary}>{description}</div>
        <label className={styles.reason}><span>操作原因</span><textarea value={reason} onChange={(event) => { setReason(event.target.value); setConfirmed(false) }} maxLength={1000} rows={5} required /></label>
        <label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我确认该操作不会删除或改写既有审计历史，并接受它产生的状态变化。</span></label>
        <footer className={styles.footer}><button type="button" className={styles.secondary} onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.danger} disabled={!canSubmit}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在提交' : confirmLabel}</button></footer>
      </form>
    </section>
  </div>
}
