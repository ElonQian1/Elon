import { useState, type FormEvent } from 'react'
import { X } from 'lucide-react'
import {
  type ReferenceCurveBatchDetail,
  type ReferenceCurveReviewDecision,
} from './computeReferenceCurveApi'
import styles from './ReferenceCurveDialog.module.css'

interface Props {
  action: 'review' | 'apply'
  detail: ReferenceCurveBatchDetail
  busy: boolean
  error: string
  onClose: () => void
  onReview: (decision: ReferenceCurveReviewDecision, note: string | null) => Promise<void>
  onApply: (note: string) => Promise<void>
}

export default function ReferenceCurveActionDialog({ action, detail, busy, error, onClose, onReview, onApply }: Props) {
  const [decision, setDecision] = useState<ReferenceCurveReviewDecision>('approved')
  const [note, setNote] = useState('')
  const [confirmed, setConfirmed] = useState(false)
  const [localError, setLocalError] = useState('')

  async function submit(event: FormEvent) {
    event.preventDefault()
    if (busy) return
    setLocalError('')
    try {
      if (!confirmed) throw new Error('请确认本次治理操作')
      if (action === 'review') {
        if (decision !== 'approved' && !note.trim()) throw new Error('退回或拒绝时必须填写说明')
        await onReview(decision, note.trim() || null)
      } else {
        await onApply(note.trim())
      }
    } catch (reason) { setLocalError(messageOf(reason, '治理操作失败')) }
  }

  return <div className={styles.overlay} role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget && !busy) onClose() }}>
    <form className={styles.actionDialog} onSubmit={(event) => void submit(event)}>
      <header><div><span>{detail.batch.curve_id} · v{detail.batch.curve_version}</span><h2>{action === 'review' ? '独立复核批次' : '应用参考价格'}</h2></div><button type="button" title="关闭" aria-label="关闭" onClick={onClose} disabled={busy}><X size={18} /></button></header>
      <code>{detail.batch.batch_digest}</code>
      {action === 'review' && <div className={styles.decisionTabs} role="tablist" aria-label="复核决定">
        {([['approved', '批准'], ['changes_requested', '退回补充'], ['rejected', '拒绝']] as const).map(([value, label]) => <button type="button" role="tab" aria-selected={decision === value} data-active={decision === value} key={value} onClick={() => setDecision(value)}>{label}</button>)}
      </div>}
      {action === 'apply' && <div className={styles.applySummary}><strong>{detail.batch.entries.length} 个快照将被原子登记</strong><span>报价来源固定为 fallback_curve，不创建 Job 或 Reservation。</span></div>}
      <label><span>{action === 'review' ? '复核说明' : '应用说明'}</span><textarea value={note} maxLength={2000} rows={4} onChange={(event) => setNote(event.target.value)} /></label>
      {(localError || error) && <div className={styles.dialogError}>{localError || error}</div>}
      <label className={styles.confirmRow}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>{action === 'review' ? '确认复核人与提交人不同，决定绑定当前批次摘要' : '确认消费当前批准回执并原子登记 Price Snapshot'}</span></label>
      <footer><button type="button" onClick={onClose} disabled={busy}>取消</button><button type="submit" className={styles.primary} disabled={busy || !confirmed}>{busy ? '处理中' : action === 'review' ? '提交复核' : '应用批次'}</button></footer>
    </form>
  </div>
}

function messageOf(reason: unknown, fallback: string) { return reason instanceof Error && reason.message ? reason.message : fallback }
