import { useState, type FormEvent } from 'react'
import { X } from 'lucide-react'
import { type GovernanceDecision, type ReleaseDetail } from './externalPoolApi'
import styles from './ExternalPoolDialog.module.css'

interface Props {
  action: 'review' | 'stage'; detail: ReleaseDetail; busy: boolean; error: string
  onClose: () => void
  onReview: (decision: GovernanceDecision, note: string | null) => Promise<void>
  onStage: (note: string) => Promise<void>
}

export default function AdapterReleaseActionDialog({ action, detail, busy, error, onClose, onReview, onStage }: Props) {
  const [decision, setDecision] = useState<GovernanceDecision>('approved')
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
      } else await onStage(note.trim())
    } catch (reason) { setLocalError(messageOf(reason, '治理操作失败')) }
  }

  return <div className={styles.overlay} role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget && !busy) onClose() }}><form className={styles.actionDialog} onSubmit={(event) => void submit(event)}>
    <header><div><span>{detail.request.adapter_id} · {detail.request.release_version}</span><h2>{action === 'review' ? '独立复核 release' : '暂存候选 release'}</h2></div><button type="button" title="关闭" aria-label="关闭" onClick={onClose} disabled={busy}><X size={18} /></button></header>
    <code>{detail.request.request_digest}</code>
    {action === 'review' && <div className={styles.decisionTabs}>{([['approved', '批准'], ['changes_requested', '退回补充'], ['rejected', '拒绝']] as const).map(([value, label]) => <button type="button" key={value} data-active={decision === value} onClick={() => setDecision(value)}>{label}</button>)}</div>}
    {action === 'stage' && <div className={styles.actionSummary}><strong>只形成 staged admission</strong><span>不解析工件、不验证 verifier、不创建 registry、credential、service actor 或 route。</span></div>}
    <label className={styles.noteField}><span>{action === 'review' ? '复核说明' : '暂存说明'}</span><textarea rows={4} maxLength={2000} value={note} onChange={(event) => setNote(event.target.value)} /></label>
    {(localError || error) && <div className={styles.dialogError}>{localError || error}</div>}
    <label className={styles.confirmRow}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>{action === 'review' ? '确认复核人与提交人不同，决定绑定当前 request/material 双摘要' : '确认消费 exact approved review，只暂存候选来源'}</span></label>
    <footer><span /><div><button type="button" onClick={onClose} disabled={busy}>返回</button><button type="submit" className={styles.primary} disabled={busy || !confirmed}>{busy ? '处理中' : '确认执行'}</button></div></footer>
  </form></div>
}

function messageOf(reason: unknown, fallback: string) { return reason instanceof Error && reason.message ? reason.message : fallback }
