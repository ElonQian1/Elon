import { useEffect, useState } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { api } from '../../api/client'
import { useProjectStore } from '../conversation/useProjectStore'
import ProjectReadinessCard from './ProjectReadinessCard'
import type { ProjectMember } from '../conversation/types'
import styles from './ProjectDetailPage.module.css'

type Tab = 'overview' | 'members' | 'workspace'

interface WorkspaceHealth {
  workspace_exists?: boolean
  git_initialized?: boolean
  git_remote?: string
  node_online?: boolean
  node_id?: string
  disk_free_bytes?: number
  issues?: string[]
  cli_ready?: boolean
}

export default function ProjectDetailPage() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const projects = useProjectStore((s) => s.projects)
  const space = useProjectStore((s) => s.space)
  const selectProject = useProjectStore((s) => s.selectProject)
  const reloadProjectSpace = useProjectStore((s) => s.reloadProjectSpace)

  const [tab, setTab] = useState<Tab>('overview')
  const [health, setHealth] = useState<WorkspaceHealth | null>(null)
  const [healthLoading, setHealthLoading] = useState(false)
  const [memberList, setMemberList] = useState<ProjectMember[]>([])

  const project = id ? projects.find((p) => p.id === id) : null

  useEffect(() => {
    if (id) selectProject(id).catch(() => {})
  }, [id]) // eslint-disable-line

  useEffect(() => {
    if (space?.members) setMemberList(space.members)
  }, [space])

  async function loadHealth() {
    if (!id) return
    setHealthLoading(true)
    try {
      const data = await api.get<WorkspaceHealth>(
        `/api/projects/${encodeURIComponent(id)}/workspace/health`,
      )
      setHealth(data)
    } catch {
      setHealth(null)
    } finally {
      setHealthLoading(false)
    }
  }

  async function loadMembers() {
    if (!id) return
    try {
      const data = await api.get<{ members?: ProjectMember[] }>(
        `/api/projects/${encodeURIComponent(id)}/members`,
      )
      setMemberList(data.members ?? [])
    } catch {
      // keep existing
    }
  }

  useEffect(() => {
    if (tab === 'workspace') loadHealth()
    if (tab === 'members') loadMembers()
  }, [tab, id]) // eslint-disable-line

  if (!project && !id) return <div className={styles.empty}>未选择项目</div>

  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <button className={styles.backBtn} onClick={() => navigate('/')} type="button">{'← 返回'}</button>
        <div className={styles.headerInfo}>
          <h1 className={styles.title}>{project?.name ?? id}</h1>
          {project?.description && <p className={styles.desc}>{project.description}</p>}
        </div>
        <button className={styles.refreshBtn} onClick={() => reloadProjectSpace()} type="button" title="刷新">{'↺'}</button>
      </header>

      <nav className={styles.tabs}>
        {(['overview', 'members', 'workspace'] as Tab[]).map((key) => (
          <button
            key={key}
            className={[styles.tab, tab === key ? styles.tabActive : ''].join(' ')}
            onClick={() => setTab(key)}
            type="button"
          >
            {{ overview: '概览', members: '成员', workspace: '工作区' }[key]}
          </button>
        ))}
      </nav>

      <div className={styles.content}>
        {tab === 'overview' && <OverviewTab project={project} space={space} />}
        {tab === 'members' && <MembersTab members={memberList} onRefresh={loadMembers} projectId={id ?? ''} />}
        {tab === 'workspace' && (
          <WorkspaceTab
            health={health}
            loading={healthLoading}
            onRefresh={loadHealth}
            channels={space?.channels ?? []}
            onOpenChannel={(channelId) => {
              // 跳回主页并激活该频道
              navigate('/')
              setTimeout(() => {
                if (id) useProjectStore.getState().selectProject(id).then(() => {
                  useProjectStore.getState().selectChannel(channelId)
                })
              }, 100)
            }}
          />
        )}
      </div>
    </div>
  )
}

type StoreProject = ReturnType<typeof useProjectStore.getState>['projects'][number] | null | undefined
type StoreSpace = ReturnType<typeof useProjectStore.getState>['space']

function OverviewTab({ project, space }: { project: StoreProject; space: StoreSpace }) {
  const rows: [string, string][] = [
    ['ID', project?.id ?? '-'],
    ['模板', project?.template ?? '-'],
    ['成员数', String(project?.member_count ?? space?.members?.length ?? '-')],
    ['频道数', String(space?.channels?.length ?? '-')],
    ['创建时间', project?.created_at ? new Date(project.created_at).toLocaleString('zh-CN') : '-'],
    ['最后更新', project?.updated_at ? new Date(project.updated_at).toLocaleString('zh-CN') : '-'],
  ]
  return (
    <div className={styles.overviewGrid}>
      {rows.map(([label, value]) => (
        <div key={label} className={styles.kv}>
          <span>{label}</span>
          <strong>{value}</strong>
        </div>
      ))}
    </div>
  )
}

function MembersTab({ members, onRefresh, projectId }: { members: ProjectMember[]; onRefresh: () => void; projectId: string }) {
  const [inviteAccount, setInviteAccount] = useState('')
  const [inviteRole, setInviteRole] = useState('member')
  const [inviting, setInviting] = useState(false)
  const [inviteError, setInviteError] = useState('')
  const [inviteSuccess, setInviteSuccess] = useState('')
  const [removing, setRemoving] = useState<string | null>(null)

  async function handleInvite(e: React.FormEvent) {
    e.preventDefault()
    const account = inviteAccount.trim()
    if (!account) return
    setInviting(true)
    setInviteError('')
    setInviteSuccess('')
    try {
      await api.post(`/api/projects/${encodeURIComponent(projectId)}/members`, {
        account,
        role: inviteRole,
      })
      setInviteAccount('')
      setInviteSuccess(`${account} 已邀请`)
      await onRefresh()
    } catch (err) {
      setInviteError((err as { message?: string }).message ?? '邀请失败')
    } finally {
      setInviting(false)
    }
  }

  async function handleRemove(userId: string, account: string) {
    if (!window.confirm(`确认移除成员 ${account}？`)) return
    setRemoving(userId)
    try {
      await api.delete(`/api/projects/${encodeURIComponent(projectId)}/members/${encodeURIComponent(userId)}`)
      await onRefresh()
    } catch (err) {
      alert((err as { message?: string }).message ?? '移除失败')
    } finally {
      setRemoving(null)
    }
  }

  return (
    <div>
      {/* 邀请表单 */}
      <form onSubmit={handleInvite} style={{ marginBottom: 18, display: 'flex', gap: 8, alignItems: 'flex-end', flexWrap: 'wrap' }}>
        <label style={{ display: 'flex', flexDirection: 'column', gap: 4, flex: '1 1 180px', minWidth: 160 }}>
          <span style={{ fontSize: 11, color: 'var(--text-muted)', fontWeight: 700 }}>账号（手机号/邮箱）</span>
          <input
            value={inviteAccount}
            onChange={(e) => setInviteAccount(e.target.value)}
            placeholder="15612345678"
            style={{ height: 34, border: '1px solid var(--line)', borderRadius: 6, background: '#1a1c21', color: 'var(--text)', padding: '0 10px', outline: 'none', fontSize: 13 }}
            disabled={inviting}
          />
        </label>
        <label style={{ display: 'flex', flexDirection: 'column', gap: 4, width: 110 }}>
          <span style={{ fontSize: 11, color: 'var(--text-muted)', fontWeight: 700 }}>角色</span>
          <select
            value={inviteRole}
            onChange={(e) => setInviteRole(e.target.value)}
            style={{ height: 34, border: '1px solid var(--line)', borderRadius: 6, background: '#1a1c21', color: 'var(--text)', padding: '0 8px', fontSize: 13 }}
          >
            <option value="member">成员</option>
            <option value="editor">协作者</option>
            <option value="admin">管理员</option>
            <option value="observer">只读</option>
          </select>
        </label>
        <button
          type="submit"
          disabled={inviting || !inviteAccount.trim()}
          style={{ height: 34, padding: '0 16px', background: 'var(--green)', border: 'none', borderRadius: 6, color: 'white', fontWeight: 700, fontSize: 13, cursor: 'pointer', alignSelf: 'flex-end' }}
        >
          {inviting ? '邀请中…' : '邀请'}
        </button>
      </form>
      {inviteError && <p style={{ fontSize: 12, color: 'var(--red)', marginBottom: 10 }}>{inviteError}</p>}
      {inviteSuccess && <p style={{ fontSize: 12, color: '#4caf78', marginBottom: 10 }}>{inviteSuccess}</p>}

      <div className={styles.tabToolbar}>
        <span className={styles.tabCount}>{members.length} 位成员</span>
        <button className={styles.textBtn} onClick={onRefresh} type="button">刷新</button>
      </div>
      <div className={styles.memberGrid}>
        {members.map((m) => (
          <div key={m.user_id} className={styles.memberCard}>
            <div className={styles.memberAvatar}>
              {(m.account ?? m.user_id ?? '?')[0]?.toUpperCase()}
            </div>
            <div className={styles.memberInfo} style={{ flex: 1 }}>
              <strong>{m.account ?? m.user_id ?? '-'}</strong>
              <span>{m.user_id}</span>
              <span className={styles.roleBadge}>{m.role ?? 'member'}</span>
            </div>
            <button
              style={{ flexShrink: 0, width: 28, height: 28, borderRadius: '50%', border: 'none', background: 'transparent', color: 'var(--text-muted)', fontSize: 16, cursor: 'pointer' }}
              title="移除成员"
              disabled={removing === m.user_id}
              onClick={() => handleRemove(m.user_id, m.account ?? m.user_id)}
              type="button"
            >
              {removing === m.user_id ? '…' : '×'}
            </button>
          </div>
        ))}
        {members.length === 0 && <p className={styles.empty}>暂无成员数据</p>}
      </div>
    </div>
  )
}

function WorkspaceTab({ health, loading, channels, onRefresh, onOpenChannel }: {
  health: WorkspaceHealth | null
  loading: boolean
  channels: { id: string; name: string; kind?: string }[]
  onRefresh: () => void
  onOpenChannel: (channelId: string) => void
}) {
  if (loading) return <div className={styles.loading}>检查工作区状态…</div>
  if (!health) return (
    <div className={styles.empty}>
      无法读取工作区状态
      <button className={styles.textBtn} style={{ marginLeft: 8 }} onClick={onRefresh} type="button">重试</button>
    </div>
  )

  const rows: [string, string][] = [
    ['Git 初始化', health.git_initialized ? '已初始化' : '未初始化'],
    ['Git 远端', health.git_remote ?? '未配置'],
    ['节点在线', health.node_online ? '在线' : '离线'],
    ['节点 ID', health.node_id ?? '未知'],
    ['AI Agent', health.cli_ready ? '就绪' : '未就绪'],
    ['磁盘剩余', health.disk_free_bytes
      ? `${(health.disk_free_bytes / 1024 / 1024 / 1024).toFixed(1)} GB`
      : '未知'],
  ]

  return (
    <div>
      {/* P2.3：开发就绪进度卡片 */}
      <ProjectReadinessCard
        health={health}
        loading={false}
        channels={channels}
        onRefresh={onRefresh}
        onOpenChannel={onOpenChannel}
      />

      <div className={styles.tabToolbar}>
        <span className={styles.tabCount}>工作区详情</span>
        <button className={styles.textBtn} onClick={onRefresh} type="button">刷新</button>
      </div>
      <div className={styles.overviewGrid}>
        {rows.map(([label, value]) => (
          <div key={label} className={styles.kv}>
            <span>{label}</span>
            <strong>{value}</strong>
          </div>
        ))}
      </div>
      {(health.issues ?? []).length > 0 && (
        <div className={styles.issues}>
          <strong>问题：</strong>
          {(health.issues ?? []).map((issue, i) => <div key={i}>{issue}</div>)}
        </div>
      )}
    </div>
  )
}
