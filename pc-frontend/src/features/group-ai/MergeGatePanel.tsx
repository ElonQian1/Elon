import { CheckCircle2, GitMerge, ShieldCheck, XCircle } from 'lucide-react'
import { useState } from 'react'
import { applyMatterMergeRequest, checkMatterMergeRequest } from './api'
import type { MergeGateReport, ProjectAiMergeRequest } from './types'
import styles from './MergeGatePanel.module.css'

interface Props {
  projectId: string
  matterId: string
  request: ProjectAiMergeRequest
  onChanged: () => void
}

export default function MergeGatePanel({ projectId, matterId, request, onChanged }: Props) {
  const [report, setReport] = useState<MergeGateReport | null>(null)
  const [busy, setBusy] = useState('')
  const [error, setError] = useState('')

  async function checkGate() {
    setBusy('check')
    setError('')
    try {
      const response = await checkMatterMergeRequest(projectId, matterId, request.id)
      setReport(response.merge_gate)
    } catch (err) {
      setError((err as { message?: string }).message ?? '合并门禁检查失败')
    } finally {
      setBusy('')
    }
  }

  async function applyGate() {
    setBusy('apply')
    setError('')
    try {
      const response = await applyMatterMergeRequest(projectId, matterId, request.id, {
        verificationCommands: report?.verification_commands ?? [],
      })
      setReport(response.merge_apply.gate)
      onChanged()
    } catch (err) {
      setError((err as { message?: string }).message ?? '执行合并失败')
    } finally {
      setBusy('')
    }
  }

  const activeReport = report
  return (
    <div className={styles.panel}>
      <div className={styles.actions}>
        <button disabled={busy === 'check'} onClick={checkGate} type="button">
          <ShieldCheck size={13} />
          {busy === 'check' ? '检查中' : '检查门禁'}
        </button>
        <button
          disabled={busy === 'apply' || !activeReport?.can_apply || request.status === 'merged'}
          onClick={applyGate}
          type="button"
        >
          <GitMerge size={13} />
          {busy === 'apply' ? '合并中' : '执行合并'}
        </button>
      </div>
      {error && <div className={styles.error}>{error}</div>}
      {activeReport && (
        <div className={styles.result}>
          <div className={styles.gateState} data-state={activeReport.can_apply ? 'passed' : 'blocked'}>
            {activeReport.can_apply ? <CheckCircle2 size={14} /> : <XCircle size={14} />}
            <strong>{activeReport.can_apply ? '可合并' : '不可合并'}</strong>
            <span>{activeReport.review_gate.status}</span>
          </div>
          {activeReport.review_gate.blockers.map((item) => (
            <p className={styles.blocker} key={item}>{item}</p>
          ))}
          <div className={styles.checks}>
            {activeReport.checks.slice(0, 5).map((check) => (
              <span data-state={check.status} key={`${check.name}:${check.detail}`}>
                {check.name}: {check.status}
              </span>
            ))}
          </div>
          {!!activeReport.verification_commands.length && (
            <small>{activeReport.verification_commands.join(' · ')}</small>
          )}
        </div>
      )}
    </div>
  )
}
