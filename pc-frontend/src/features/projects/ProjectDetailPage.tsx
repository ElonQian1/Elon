import { useEffect, useState } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { api } from '../../api/client'
import { useProjectStore } from '../conversation/useProjectStore'
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
        {tab === 'members' && <MembersTab members={memberList} onRefresh={loadMembers} />}
        {tab === 'workspace' && <WorkspaceTab health={health} loading={healthLoading} onRefresh={loadHealth} />}
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

function MembersTab({ members, onRefresh }: { members: ProjectMember[]; onRefresh: () => void }) {
  return (
    <div>
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
            <div className={styles.memberInfo}>
              <strong>{m.account ?? m.user_id ?? '-'}</strong>
              <span>{m.user_id}</span>
              <span className={styles.roleBadge}>{m.role ?? 'member'}</span>
            </div>
          </div>
        ))}
        {members.length === 0 && <p className={styles.empty}>暂无成员数据</p>}
      </div>
    </div>
  )
}

function WorkspaceTab({ health, loading, onRefresh }: {
  health: WorkspaceHealth | null
  loading: boolean
  onRefresh: () => void
}) {
  if (loading) return <div className={styles.loading}>检查工作区状态…</div>
  if (!health) return (
    <div className={styles.empty}>
      无法读取工作区状态
      <button className={styles.textBtn} style={{ marginLeft: 8 }} onClick={onRefresh} type="button">重试</button>
    </div>
  )

  const rows: [string, string][] = [
    ['工作区目录', health.workspace_exists ? '存在' : '不存在'],
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
      <div className={styles.tabToolbar}>
        <span className={styles.tabCount}>工作区健康状态</span>
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
