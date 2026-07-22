import { useEffect, useState } from 'react'
import { GitBranch, Play, RefreshCw } from 'lucide-react'
import type { LocalTaskContinuationInput, LocalTaskDetail } from './types'
import styles from './LocalTasksPage.module.css'

interface Props {
  detail: LocalTaskDetail
  busy: boolean
  onContinue: (input: LocalTaskContinuationInput) => Promise<boolean>
}

const TERMINAL = new Set([
  'done', 'failed', 'canceled', 'cancelled', 'interrupted', 'resume_required',
])

export default function LocalTaskContinuationPanel({ detail, busy, onContinue }: Props) {
  const contractCriteria = detail.supervision.contract?.acceptance_criteria.join('\n') ?? ''
  const [editingRevision, setEditingRevision] = useState(false)
  const [prompt, setPrompt] = useState(detail.task.prompt)
  const [criteria, setCriteria] = useState(contractCriteria)
  const [reason, setReason] = useState('')

  useEffect(() => {
    setEditingRevision(false)
    setPrompt(detail.task.prompt)
    setCriteria(contractCriteria)
    setReason('')
  }, [contractCriteria, detail.task.id, detail.task.prompt])

  const contract = detail.supervision.contract
  const eligibleRole = contract && ['requirement', 'resume_original'].includes(contract.task_role)
  if (!eligibleRole || !TERMINAL.has(detail.task.status.toLowerCase())) return null

  const workspaceEligible = detail.resume_workspace_status?.eligible === true
  const revisedCriteria = criteria.split(/\r?\n/).map((item) => item.trim()).filter(Boolean)
  const canSupersede = workspaceEligible && prompt.trim() && revisedCriteria.length > 0 && reason.trim()

  return (
    <section className={styles.continuationCard} data-testid="task-continuation">
      <div className={styles.sectionHeading}>
        <h3>继续这个任务</h3>
        <span>{workspaceEligible ? '工作区身份已验证' : '工作区暂不可继续'}</span>
      </div>
      <p>目标不变请选择“继续原任务”；目标或验收标准改变请选择“需求变更承接”。</p>
      <div className={styles.continuationActions}>
        <button
          type="button"
          disabled={busy || !workspaceEligible}
          onClick={() => void onContinue({ mode: 'resume', prompt: '', acceptance_criteria: [], reason: '' })}
        >
          <RefreshCw size={14} aria-hidden="true" />
          {busy ? '正在承接…' : '继续原任务'}
        </button>
        <button type="button" disabled={busy || !workspaceEligible} onClick={() => setEditingRevision((value) => !value)}>
          <GitBranch size={14} aria-hidden="true" />需求变更承接
        </button>
      </div>
      {!workspaceEligible && <small>{detail.resume_workspace_status?.reason || '节点没有足够的身份、租约或工作区证据。'}</small>}
      {editingRevision && (
        <div className={styles.continuationForm}>
          <label>
            <span>新的完整需求</span>
            <textarea value={prompt} onChange={(event) => setPrompt(event.target.value)} rows={5} />
          </label>
          <label>
            <span>新的验收条件（每行一项）</span>
            <textarea value={criteria} onChange={(event) => setCriteria(event.target.value)} rows={5} />
          </label>
          <label>
            <span>为什么改变需求</span>
            <input value={reason} onChange={(event) => setReason(event.target.value)} placeholder="例如：用户改变了页面目标" />
          </label>
          <button
            className={styles.primaryButton}
            type="button"
            disabled={busy || !canSupersede}
            onClick={() => void onContinue({
              mode: 'supersede', prompt: prompt.trim(), acceptance_criteria: revisedCriteria, reason: reason.trim(),
            }).then((ok) => { if (ok) setEditingRevision(false) })}
          >
            <Play size={14} fill="currentColor" aria-hidden="true" />
            {busy ? '正在保存修订…' : '保存修订并开始承接'}
          </button>
        </div>
      )}
    </section>
  )
}
