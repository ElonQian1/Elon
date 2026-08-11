import { useState, type FormEvent } from 'react'
import { X } from 'lucide-react'
import { type GovernanceDecision, type OnboardingDetail } from './externalPoolApi'
import styles from './ExternalPoolDialog.module.css'

interface Props {
  action: 'cancel' | 'review' | 'apply'
  detail: OnboardingDetail
  busy: boolean
  error: string
  onClose: () => void
  onCancel: () => Promise<void>
  onReview: (decision: GovernanceDecision, note: string | null) => Promise<void>
  onApply: () => Promise<void>
}

export default function OnboardingActionDialog({ action, detail, busy, error, onClose, onCancel, onReview, onApply }: Props) {
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
        if (decision !== 'approved' && !note.trim()) throw new Error('退回或拒绝时必须填写原因')
        await onReview(decision, note.trim() || null)
      } else if (action === 'cancel') await onCancel()
      else await onApply()
    } catch (reason) { setLocalError(messageOf(reason, '治理操作失败')) }
  }

  return <div className={styles.overlay} role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget && !busy) onClose() }}><form className={styles.actionDialog} onSubmit={(event) => void submit(event)}>
    <header><div><span>{detail.request.provider_id}</span><h2>{title(action)}</h2></div><button type="button" title="关闭" aria-label="关闭" onClick={onClose} disabled={busy}><X size={18} /></button></header>
    <code>{detail.request.request_digest}</code>
    {action === 'review' && <><div className={styles.decisionTabs}>{([['approved', '批准'], ['changes_requested', '退回补充'], ['rejected', '拒绝']] as const).map(([value, label]) => <button type="button" key={value} data-active={decision === value} onClick={() => setDecision(value)}>{label}</button>)}</div><label className={styles.noteField}><span>复核说明</span><textarea rows={4} maxLength={2000} value={note} onChange={(event) => setNote(event.target.value)} /></label></>}
    {action !== 'review' && <div className={styles.actionSummary}><strong>{action === 'cancel' ? '取消仍为 submitted 的申请' : '登记 registering/self_declared Provider'}</strong><span>{action === 'cancel' ? '不会登记 Provider，重复取消返回同一状态。' : '不会激活 Provider，不生成 route、容量、报价或结算。'}</span></div>}
    {(localError || error) && <div className={styles.dialogError}>{localError || error}</div>}
    <label className={styles.confirmRow}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>{confirmLabel(action)}</span></label>
    <footer><span /><div><button type="button" onClick={onClose} disabled={busy}>返回</button><button type="submit" className={styles.primary} disabled={busy || !confirmed}>{busy ? '处理中' : '确认执行'}</button></div></footer>
  </form></div>
}

function title(value: string) { return ({ cancel: '取消接入申请', review: '独立复核接入申请', apply: '登记外部 Provider' } as Record<string, string>)[value] }
function confirmLabel(value: string) { return ({ cancel: '确认取消当前 exact 申请', review: '确认复核人与 Provider owner 不同，决定绑定当前摘要', apply: '确认消费 approved 回执，只登记 registering Provider' } as Record<string, string>)[value] }
function messageOf(reason: unknown, fallback: string) { return reason instanceof Error && reason.message ? reason.message : fallback }
