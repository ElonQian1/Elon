import { useCallback, useEffect, useState } from 'react'
import { RefreshCw, X } from 'lucide-react'
import { taskEconomyApi } from './taskEconomyApi'
import type { SuiPreflightJob } from './suiPreflightTypes'
import { errorText } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import {
  actionStyle,
  badgeStyle,
  commerceStyles,
  errorMessageStyle,
  listItemStyle,
} from './openCommerceStyles'

export default function SuiPreflightJobsPanel({
  projectId,
  canEdit,
}: {
  projectId: string
  canEdit: boolean
}) {
  const [jobs, setJobs] = useState<SuiPreflightJob[]>([])
  const [busy, setBusy] = useState('')
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    setMessage('')
    try {
      const response = await taskEconomyApi.suiPreflightJobs(projectId)
      setJobs(response.jobs)
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [projectId])

  useEffect(() => {
    refresh()
  }, [refresh])

  async function cancel(job: SuiPreflightJob) {
    setBusy(job.id)
    setMessage('')
    try {
      await taskEconomyApi.cancelSuiPreflightJob(
        projectId,
        job.id,
        '由项目编辑者在 PC 工作台取消',
      )
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy('')
    }
  }

  return (
    <section className={base.integrationSection}>
      <header>
        <div>
          <strong>Sui 离线预检队列</strong>
          <small>短时租约 · 摘要漂移阻断 · 不签名、不广播</small>
        </div>
        <button style={actionStyle('icon')} type="button" onClick={refresh} title="刷新预检任务">
          <RefreshCw size={14} />
        </button>
      </header>
      <div style={commerceStyles.sectionBody}>
        <div style={{ ...commerceStyles.list, ...commerceStyles.scrollArea }}>
          {jobs.map((job) => (
            <article className={base.formCard} style={listItemStyle()} key={job.id}>
              <header style={commerceStyles.itemHeader}>
                <strong>{job.package_kind} · {job.target_network}</strong>
                <span style={badgeStyle(statusTone(job.status))}>{job.status}</span>
              </header>
              <p style={commerceStyles.itemText}>
                尝试 {job.attempt_no} 次
                {job.lease_expires_at ? ` · 租约至 ${formatTime(job.lease_expires_at)}` : ''}
              </p>
              <code style={commerceStyles.itemMeta}>{job.projection_package_id}</code>
              <code style={commerceStyles.itemMeta}>{job.handoff_digest.slice(0, 24)}</code>
              {job.last_error && <small style={commerceStyles.itemMeta}>{job.last_error}</small>}
              {(job.status === 'pending' || job.status === 'blocked') && (
                <button
                  style={actionStyle('icon')}
                  type="button"
                  onClick={() => cancel(job)}
                  disabled={!canEdit || busy !== ''}
                  title="取消预检任务"
                >
                  <X size={14} />
                </button>
              )}
            </article>
          ))}
          {jobs.length === 0 && <p className={base.empty}>暂无离线预检任务。</p>}
        </div>
        {message && <div style={{ ...commerceStyles.message, ...errorMessageStyle }}>{message}</div>}
      </div>
    </section>
  )
}

function statusTone(status: SuiPreflightJob['status']) {
  if (status === 'completed') return 'neutral' as const
  if (status === 'blocked') return 'danger' as const
  return 'warn' as const
}

function formatTime(value: string) {
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString()
}
