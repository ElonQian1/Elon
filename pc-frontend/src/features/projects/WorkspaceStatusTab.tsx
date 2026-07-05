import { GitBranch } from 'lucide-react'
import type { Channel } from '../conversation/types'
import ProjectReadinessCard from './ProjectReadinessCard'
import type { WorkspaceHealth } from './projectManagementTypes'
import styles from './ProjectDetailPage.module.css'

interface Props {
  projectId: string
  health: WorkspaceHealth | null
  loading: boolean
  channels: Channel[]
  onRefresh: () => void
  onOpenGitWorktrees: () => void
  onOpenChannel: (channelId: string) => void
}

export default function WorkspaceStatusTab({
  projectId,
  health,
  loading,
  channels,
  onRefresh,
  onOpenGitWorktrees,
  onOpenChannel,
}: Props) {
  if (loading) return <div className={styles.loading}>检查工作区状态…</div>
  if (!health) return (
    <div className={styles.empty}>
      无法读取工作区状态
      <button className={styles.textBtn} style={{ marginLeft: 8 }} onClick={onRefresh} type="button">重试</button>
    </div>
  )

  const nodeOnline = health.node_online ?? health.node?.online ?? false
  const nodeId = health.node_id ?? health.node?.node_id ?? health.project?.node_id ?? '未知'
  const cliReady = health.cli_ready ?? (health.node?.cli_connected && health.node?.cli_project_ready) ?? health.can_run_on_pc ?? false
  const diskFreeBytes = health.disk_free_bytes ?? health.live_inspect?.disk_free_bytes ?? undefined
  const gitInitialized = health.git_initialized ?? health.live_inspect?.is_git_worktree
  const gitRemote = health.git_remote ?? health.live_inspect?.git_remote_origin ?? undefined
  const issues = health.issues ?? health.warnings ?? []
  const rows: [string, string][] = [
    ['Git 初始化', gitInitialized === false ? '未初始化' : '已初始化'],
    ['Git 远端', gitRemote ?? '未配置'],
    ['节点在线', nodeOnline ? '在线' : '离线'],
    ['节点 ID', nodeId],
    ['AI Agent', cliReady ? '就绪' : '未就绪'],
    ['磁盘剩余', diskFreeBytes ? `${(diskFreeBytes / 1024 / 1024 / 1024).toFixed(1)} GB` : '未知'],
  ]

  return (
    <div>
      <ProjectReadinessCard
        health={health}
        loading={false}
        channels={channels}
        onRefresh={onRefresh}
        onOpenChannel={onOpenChannel}
      />

      <div className={styles.tabToolbar}>
        <span className={styles.tabCount}>工作区详情</span>
        <div className={styles.workspaceActions}>
          <button className={styles.textBtn} onClick={onOpenGitWorktrees} disabled={!projectId} type="button">
            <GitBranch size={14} aria-hidden="true" />
            <span>Git 现场</span>
          </button>
          <button className={styles.textBtn} onClick={onRefresh} type="button">刷新</button>
        </div>
      </div>
      <div className={styles.overviewGrid}>
        {rows.map(([label, value]) => (
          <div key={label} className={styles.kv}>
            <span>{label}</span>
            <strong>{value}</strong>
          </div>
        ))}
      </div>
      {issues.length > 0 && (
        <div className={styles.issues}>
          <strong>问题：</strong>
          {issues.map((issue, i) => <div key={i}>{issue}</div>)}
        </div>
      )}
    </div>
  )
}
