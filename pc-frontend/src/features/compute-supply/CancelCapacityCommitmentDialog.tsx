import { useState, type FormEvent } from 'react'
import { LoaderCircle, LockKeyholeOpen, X } from 'lucide-react'
import { type CapacityCommitmentDetail } from './computeCapacityCommitmentApi'
import styles from './OfferDraftActionDialog.module.css'

interface Props {
  detail: CapacityCommitmentDetail
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (reason: string, idempotencyKey: string) => Promise<void>
}

export default function CancelCapacityCommitmentDialog({ detail, busy, error, onClose, onSubmit }: Props) {
  const [reason, setReason] = useState('provider canceled before delivery')
  const [confirmed, setConfirmed] = useState(false)
  const [idempotencyKey] = useState(() => `capacity-commitment-cancel:${detail.commitment.commitment_digest}`)
  async function submit(event: FormEvent<HTMLFormElement>) { event.preventDefault(); if (confirmed && reason.trim() && !busy) await onSubmit(reason.trim(), idempotencyKey) }
  return <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}><section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="cancel-capacity-commitment-title">
    <header><div><LockKeyholeOpen size={18} /><h2 id="cancel-capacity-commitment-title">取消容量承诺</h2></div><button type="button" onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button></header>
    <form onSubmit={(event) => void submit(event)}>{error && <div className={styles.error}>{error}</div>}<div className={styles.identity}><span>{detail.commitment.commitment_id}</span><code>{detail.commitment.commitment_digest}</code></div><label className={styles.dataClasses}>取消原因<textarea rows={3} maxLength={1000} value={reason} onChange={(event) => setReason(event.target.value)} /></label><div className={styles.warning}>取消会追加唯一 `canceled` 回执并归还 held 容量。历史承诺不会删除，也不能重开。</div><label className={styles.confirm}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我确认在交付窗口开始前取消这份精确承诺。</span></label><footer><button type="button" onClick={onClose} disabled={busy}>返回</button><button type="submit" className={styles.danger} disabled={!confirmed || !reason.trim() || busy}>{busy && <LoaderCircle size={15} className={styles.spinning} />}{busy ? '正在取消' : '确认取消'}</button></footer></form>
  </section></div>
}
