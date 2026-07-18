import { RefreshCcw, ShieldAlert } from 'lucide-react'
import type { LocalTaskResumeWorkspaceStatus, LocalTaskUpdateRecovery } from './types'
import styles from './LocalTasksPage.module.css'

export default function LocalTaskUpdateRecoveryPanel({
  recovery,
  resumeWorkspace,
}: {
  recovery?: LocalTaskUpdateRecovery
  resumeWorkspace?: LocalTaskResumeWorkspaceStatus
}) {
  if (!recovery && !resumeWorkspace) return null
  const remoteFailClosed = recovery != null
    && recovery.transport_kind !== 'local_loopback'
    && (!recovery.capabilities.includes('update_recovery_v1') || !recovery.replay_from_cursor)
  return (
    <section className={styles.recoveryCard} data-state={recovery?.state || 'resume_check'}>
      <div className={styles.sectionHeading}>
        <h3><RefreshCcw size={15} aria-hidden="true" />更新恢复全过程</h3>
        <span>{recovery ? recoveryStateLabel(recovery.state) : 'Resume 安全检查'}</span>
      </div>
      {recovery && (
        <>
          <p className={styles.supervisionIntro}>
            节点以持久 receipt、journal 游标、sidecar 游标和 completion event 作为同一恢复事实源。
          </p>
          <dl className={styles.recoveryMeta}>
            <Item label="旧 runtime" value={releaseLabel(recovery.from_version, recovery.from_git_sha)} />
            <Item label="新 runtime" value={releaseLabel(recovery.to_version, recovery.to_git_sha)} />
            <Item label="恢复阶段" value={recoveryStateLabel(recovery.state)} />
            <Item label="预计中断" value={`${recovery.expected_downtime_ms || 0} ms`} />
            <Item label="旧任务 / Resume" value={`${recovery.original_task_id}${recovery.resume_task_id ? ` / ${recovery.resume_task_id}` : ' / 未派生'}`} />
            <Item label="sidecar" value={recovery.sidecar_session_id || '无'} />
            <Item label="重放游标" value={`journal ${recovery.journal_cursor} · output ${recovery.sidecar_output_offset} / #${recovery.sidecar_output_sequence}`} />
            <Item label="可靠终态" value={recovery.completion_event_id || recovery.terminal_task_status || '等待中'} />
          </dl>
          <div className={styles.recoveryFoot}>
            <span>策略：{recovery.resume_strategy || '优先 reattach，必要时幂等 Resume'}</span>
            <span>Desktop review：{recovery.review_verdict || '待审核'}{recovery.review_summary ? ` · ${recovery.review_summary}` : ''}</span>
          </div>
          {recovery.state_reason && <p className={styles.recoveryReason}>{recovery.state_reason}</p>}
          {remoteFailClosed && (
            <p className={styles.remoteWarning}><ShieldAlert size={14} aria-hidden="true" />remote v1 字段已保留，但重放能力未实测，当前明确 fail-closed。</p>
          )}
        </>
      )}
      {resumeWorkspace && (
        <div className={styles.resumeCheck} data-eligible={resumeWorkspace.eligible}>
          <strong>Resume 工作区：{resumeWorkspace.eligible ? '可安全继承' : '已拒绝'}</strong>
          <span>{resumeWorkspace.derivation || resumeWorkspace.reason || '节点没有足够的工作区身份凭据'}</span>
          {resumeWorkspace.active_workspace_path && <code>{resumeWorkspace.active_workspace_path}</code>}
          {resumeWorkspace.git_head && <span>{resumeWorkspace.branch} · HEAD {resumeWorkspace.git_head.slice(0, 12)}</span>}
        </div>
      )}
    </section>
  )
}

function Item({ label, value }: { label: string; value: string }) {
  return <div><dt>{label}</dt><dd title={value}>{value}</dd></div>
}

function releaseLabel(version: string, sha: string): string {
  return [version, sha ? sha.slice(0, 12) : ''].filter(Boolean).join(' · ') || '未知'
}

function recoveryStateLabel(state: string): string {
  const labels: Record<string, string> = {
    planned: '已计划', downloaded: '已下载', checkpoint_saved: '已保存 checkpoint',
    applying: '正在更新', runtime_online: '新 runtime 在线', reattaching: '正在重连 sidecar',
    resume_created: '已创建 Resume', resumed: '已恢复执行', verified: '终态已核验',
    paused: '安全暂停', approval_required: '等待审批', conflict: '身份冲突',
    timeout: '恢复超时', failed: '恢复失败',
  }
  return labels[state] || state || '等待恢复'
}
