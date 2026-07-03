import { useEffect, useMemo, useState } from 'react'
import { useParams, useNavigate, useLocation } from 'react-router-dom'
import { api } from '../../api/client'
import GroupAiPanel from '../group-ai/GroupAiPanel'
import { useProjectStore } from '../conversation/useProjectStore'
import ProjectReadinessCard from './ProjectReadinessCard'
import WorkspaceAccessPanel from './WorkspaceAccessPanel'
import type { ProjectMember, ProjectRole, ProjectRolesResponse } from '../conversation/types'
import {
  filterMembers,
  memberInitial,
  memberModerationSummary,
  memberPresenceStatus,
  memberPrimaryRoleColor,
  memberRoleSummary,
  presenceLabel,
  projectMemberHasRolePermission,
  ROLE_PERMISSION_INVITE_MEMBERS,
  ROLE_PERMISSION_MANAGE_MEMBERS,
  ROLE_PERMISSION_MODERATE_MEMBERS,
  roleLabel,
} from '../conversation/memberUtils'
import { useAuthStore } from '../../store/auth'
import styles from './ProjectDetailPage.module.css'

type Tab = 'overview' | 'members' | 'workspace' | 'groupAi'

interface WorkspaceHealth {
  workspace_exists?: boolean
  git_initialized?: boolean
  git_remote?: string
  node_online?: boolean
  node_id?: string
  disk_free_bytes?: number
  issues?: string[]
  cli_ready?: boolean
  can_run_on_pc?: boolean
  verified_can_run_on_pc?: boolean | null
  warnings?: string[]
  project?: {
    workspace_path?: string | null
    node_id?: string | null
  }
  node?: {
    node_id?: string
    online?: boolean
    cli_connected?: boolean
    cli_project_ready?: boolean
  } | null
  live_inspect?: {
    is_git_worktree?: boolean
    git_remote_origin?: string | null
    disk_free_bytes?: number | null
  } | null
}

export default function ProjectDetailPage() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const location = useLocation()
  const currentUserId = useAuthStore((s) => s.user?.id)
  const projects = useProjectStore((s) => s.projects)
  const space = useProjectStore((s) => s.space)
  const selectProject = useProjectStore((s) => s.selectProject)
  const reloadProjectSpace = useProjectStore((s) => s.reloadProjectSpace)

  const [tab, setTab] = useState<Tab>(() => location.pathname.endsWith('/members') ? 'members' : 'overview')
  const [health, setHealth] = useState<WorkspaceHealth | null>(null)
  const [healthLoading, setHealthLoading] = useState(false)
  const [memberList, setMemberList] = useState<ProjectMember[]>([])
  const [roles, setRoles] = useState<ProjectRole[]>([])

  const project = id ? projects.find((p) => p.id === id) : null
  const activeRole = String(space?.my_role ?? project?.my_role ?? project?.role ?? '').toLowerCase()
  const currentMember = currentUserId ? (space?.members ?? memberList).find((member) => member.user_id === currentUserId) : undefined
  const fallbackAdmin = activeRole === 'owner' || activeRole === 'admin'
  const canInviteMembers = canUseProjectPermission(currentMember, roles, ROLE_PERMISSION_INVITE_MEMBERS, fallbackAdmin)
  const canManageMembers = canUseProjectPermission(currentMember, roles, ROLE_PERMISSION_MANAGE_MEMBERS, fallbackAdmin)
  const canModerateMembers = canUseProjectPermission(currentMember, roles, ROLE_PERMISSION_MODERATE_MEMBERS, fallbackAdmin)

  useEffect(() => {
    if (id) selectProject(id).catch(() => {})
  }, [id]) // eslint-disable-line

  useEffect(() => {
    if (location.pathname.endsWith('/members')) setTab('members')
  }, [location.pathname])

  useEffect(() => {
    if (space?.members) setMemberList(space.members)
  }, [space])

  useEffect(() => {
    if (!id) return
    api.get<ProjectRolesResponse>(`/api/projects/${encodeURIComponent(id)}/roles`)
      .then((data) => setRoles(data.roles ?? []))
      .catch(() => setRoles([]))
  }, [id])

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

  function switchTab(nextTab: Tab) {
    setTab(nextTab)
    if (!id) return
    if (nextTab === 'members') {
      if (!location.pathname.endsWith('/members')) navigate(`/projects/${id}/members`)
      return
    }
    if (location.pathname.endsWith('/members')) navigate(`/projects/${id}`)
  }

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
        {(['overview', 'members', 'workspace', 'groupAi'] as Tab[]).map((key) => (
          <button
            key={key}
            className={[styles.tab, tab === key ? styles.tabActive : ''].join(' ')}
            onClick={() => switchTab(key)}
            type="button"
          >
            {{ overview: '概览', members: '成员', workspace: '工作区', groupAi: '群体 AI' }[key]}
          </button>
        ))}
      </nav>

      <div className={styles.content}>
        {tab === 'overview' && <OverviewTab project={project} space={space} />}
        {tab === 'members' && (
          <MembersTab
            members={memberList}
            onRefresh={loadMembers}
            projectId={id ?? ''}
            currentUserId={currentUserId}
            canInviteMembers={canInviteMembers}
            canManageMembers={canManageMembers}
            canModerateMembers={canModerateMembers}
          />
        )}
        {tab === 'groupAi' && <GroupAiPanel projectId={id ?? ''} channels={space?.channels ?? []} />}
        {tab === 'workspace' && (
          <>
            <WorkspaceAccessPanel
              projectId={id ?? ''}
              projectName={space?.project?.name ?? project?.name ?? id ?? '当前项目'}
              workspacePath={space?.project?.workspace_path ?? project?.workspace_path}
              runtimePermission={space?.project?.runtime_permission ?? project?.runtime_permission}
              boundNodeId={space?.project?.node_id ?? project?.node_id}
              onChanged={async () => {
                await reloadProjectSpace()
                await loadHealth()
              }}
            />
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
          </>
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

function canUseProjectPermission(member: ProjectMember | undefined, roles: ProjectRole[], permission: string, fallback: boolean) {
  if (member && roles.length > 0) return projectMemberHasRolePermission(member, roles, permission)
  return fallback
}

type ProjectMemberStatusFilter = 'all' | 'online' | 'offline' | 'restricted'
type ProjectMemberSortMode = 'role' | 'name' | 'joined'
type ProjectMemberBatchAction = 'mute1h' | 'mute1d' | 'unmute' | 'remove'

const PROJECT_MEMBER_STATUS_FILTERS: Array<{ id: ProjectMemberStatusFilter; label: string }> = [
  { id: 'all', label: '全部' },
  { id: 'online', label: '在线' },
  { id: 'offline', label: '离线' },
  { id: 'restricted', label: '受限' },
]

function MembersTab({
  members,
  onRefresh,
  projectId,
  currentUserId,
  canInviteMembers,
  canManageMembers,
  canModerateMembers,
}: {
  members: ProjectMember[]
  onRefresh: () => Promise<void> | void
  projectId: string
  currentUserId?: string
  canInviteMembers: boolean
  canManageMembers: boolean
  canModerateMembers: boolean
}) {
  const [inviteAccount, setInviteAccount] = useState('')
  const [inviteRole, setInviteRole] = useState('member')
  const [inviting, setInviting] = useState(false)
  const [inviteError, setInviteError] = useState('')
  const [inviteSuccess, setInviteSuccess] = useState('')
  const [removing, setRemoving] = useState<string | null>(null)
  const [query, setQuery] = useState('')
  const [statusFilter, setStatusFilter] = useState<ProjectMemberStatusFilter>('all')
  const [roleFilter, setRoleFilter] = useState('')
  const [sortMode, setSortMode] = useState<ProjectMemberSortMode>('role')
  const [selectedIds, setSelectedIds] = useState<Set<string>>(() => new Set())
  const [batchBusy, setBatchBusy] = useState<ProjectMemberBatchAction | ''>('')
  const [batchMessage, setBatchMessage] = useState('')
  const stats = useMemo(() => projectMemberStats(members), [members])
  const roleOptions = useMemo(() => projectMemberRoleOptions(members), [members])
  const visibleMembers = useMemo(() => {
    const searched = filterMembers(members, query)
    return sortProjectMembers(
      searched
        .filter((member) => matchesProjectMemberStatus(member, statusFilter))
        .filter((member) => !roleFilter || projectMemberHasRole(member, roleFilter)),
      sortMode,
    )
  }, [members, query, roleFilter, sortMode, statusFilter])
  const selectedMembers = useMemo(
    () => members.filter((member) => selectedIds.has(member.user_id) && member.user_id !== currentUserId),
    [members, selectedIds, currentUserId],
  )
  const canBatchMembers = canManageMembers || canModerateMembers
  const selectableVisibleMembers = visibleMembers.filter((member) => member.user_id !== currentUserId)
  const selectedVisibleCount = selectableVisibleMembers.filter((member) => selectedIds.has(member.user_id)).length
  const allVisibleSelected = selectableVisibleMembers.length > 0 && selectedVisibleCount === selectableVisibleMembers.length

  useEffect(() => {
    const validIds = new Set(members.map((member) => member.user_id))
    setSelectedIds((current) => {
      const next = new Set(Array.from(current).filter((id) => validIds.has(id) && id !== currentUserId))
      return next.size === current.size ? current : next
    })
  }, [members, currentUserId])

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

  function toggleMember(member: ProjectMember) {
    if (!canBatchMembers || member.user_id === currentUserId) return
    setSelectedIds((current) => {
      const next = new Set(current)
      if (next.has(member.user_id)) next.delete(member.user_id)
      else next.add(member.user_id)
      return next
    })
    setBatchMessage('')
  }

  function toggleVisibleMembers() {
    if (!canBatchMembers || selectableVisibleMembers.length === 0) return
    setSelectedIds((current) => {
      const next = new Set(current)
      selectableVisibleMembers.forEach((member) => {
        if (allVisibleSelected) next.delete(member.user_id)
        else next.add(member.user_id)
      })
      return next
    })
    setBatchMessage('')
  }

  async function handleRemove(userId: string, account: string) {
    if (!canManageMembers) return
    if (userId === currentUserId) return
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

  async function runBatchAction(action: ProjectMemberBatchAction) {
    const targets = selectedMembers.filter((member) => member.user_id !== currentUserId)
    if (!projectId || !canBatchMembers || batchBusy) return
    if (targets.length === 0) {
      setBatchMessage('请先选择要处理的成员')
      return
    }
    if (action === 'remove' && !canManageMembers) return
    if (action !== 'remove' && !canModerateMembers) return
    if (action === 'remove' && !window.confirm(`确定要将 ${targets.length} 位成员移出项目吗？`)) return
    setBatchBusy(action)
    setBatchMessage(`正在处理 ${targets.length} 位成员...`)
    try {
      for (const member of targets) {
        if (action === 'remove') {
          await api.delete(`/api/projects/${encodeURIComponent(projectId)}/members/${encodeURIComponent(member.user_id)}`)
        } else {
          await api.patch(`/api/projects/${encodeURIComponent(projectId)}/members/${encodeURIComponent(member.user_id)}/moderation`, {
            action: action === 'unmute' ? 'unmute' : 'mute',
            duration_minutes: action === 'mute1d' ? 1440 : action === 'mute1h' ? 60 : undefined,
            note: projectMemberBatchNote(action),
          })
        }
      }
      setSelectedIds(new Set())
      setBatchMessage(`已批量更新 ${targets.length} 位成员`)
      await onRefresh()
    } catch (err) {
      setBatchMessage((err as { message?: string }).message ?? '批量操作失败')
    } finally {
      setBatchBusy('')
    }
  }

  return (
    <div className={styles.memberWorkbench}>
      <div className={styles.memberWorkbenchStats}>
        {PROJECT_MEMBER_STATUS_FILTERS.map((item) => (
          <button
            key={item.id}
            type="button"
            data-active={statusFilter === item.id ? 'true' : undefined}
            onClick={() => setStatusFilter(item.id)}
          >
            <strong>{projectMemberStatCount(stats, item.id)}</strong>
            <span>{item.label}</span>
          </button>
        ))}
      </div>

      <form className={styles.memberInviteForm} onSubmit={handleInvite}>
        <label>
          <span>账号（手机号/邮箱）</span>
          <input
            value={inviteAccount}
            onChange={(e) => setInviteAccount(e.target.value)}
            placeholder="15612345678"
            disabled={inviting}
          />
        </label>
        <label>
          <span>角色</span>
          <select value={inviteRole} onChange={(e) => setInviteRole(e.target.value)}>
            {inviteRoleOptions(roleOptions).map((role) => (
              <option key={role.id} value={role.id}>{role.label}</option>
            ))}
          </select>
        </label>
        <button className={styles.primaryBtn} type="submit" disabled={!canInviteMembers || inviting || !inviteAccount.trim()}>
          {inviting ? '邀请中…' : '邀请'}
        </button>
      </form>
      {inviteError && <p className={styles.memberFormError}>{inviteError}</p>}
      {inviteSuccess && <p className={styles.memberFormSuccess}>{inviteSuccess}</p>}
      {!canBatchMembers && !canInviteMembers && <p className={styles.memberFormHint}>当前角色可查看成员数据；邀请、批量限制和移除需要成员管理权限。</p>}

      <div className={styles.memberWorkbenchToolbar}>
        <input
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="搜索账号、ID、角色、状态、备注"
        />
        <select value={roleFilter} onChange={(event) => setRoleFilter(event.target.value)} aria-label="按角色筛选">
          <option value="">全部角色</option>
          {roleOptions.map((role) => (
            <option key={role.id} value={role.id}>{role.label} ({role.count})</option>
          ))}
        </select>
        <select value={sortMode} onChange={(event) => setSortMode(event.target.value as ProjectMemberSortMode)} aria-label="排序">
          <option value="role">按角色层级</option>
          <option value="name">按名称</option>
          <option value="joined">按加入时间</option>
        </select>
        <button className={styles.textBtn} onClick={onRefresh} type="button">刷新</button>
      </div>

      {canBatchMembers && (
        <section className={styles.memberBatchBar}>
          <div>
            <strong>批量成员管理</strong>
            <span>{selectedMembers.length > 0 ? `已选择 ${selectedMembers.length} 位成员` : '选择成员后可批量禁言、解禁言或移出项目'}</span>
          </div>
          <div>
            <button className={styles.textBtn} type="button" disabled={selectableVisibleMembers.length === 0 || !!batchBusy} onClick={toggleVisibleMembers}>
              {allVisibleSelected ? '取消当前' : '选择当前'}
            </button>
            <button className={styles.textBtn} type="button" disabled={selectedMembers.length === 0 || !!batchBusy} onClick={() => setSelectedIds(new Set())}>清空</button>
            {canModerateMembers && <button className={styles.textBtn} type="button" disabled={selectedMembers.length === 0 || !!batchBusy} onClick={() => runBatchAction('mute1h')}>禁言 1 小时</button>}
            {canModerateMembers && <button className={styles.textBtn} type="button" disabled={selectedMembers.length === 0 || !!batchBusy} onClick={() => runBatchAction('mute1d')}>禁言 1 天</button>}
            {canModerateMembers && <button className={styles.textBtn} type="button" disabled={selectedMembers.length === 0 || !!batchBusy} onClick={() => runBatchAction('unmute')}>解禁言</button>}
            {canManageMembers && <button className={styles.textBtn} type="button" data-danger="true" disabled={selectedMembers.length === 0 || !!batchBusy} onClick={() => runBatchAction('remove')}>批量移除</button>}
          </div>
          {batchMessage && <p>{batchMessage}</p>}
        </section>
      )}

      <div className={styles.tabToolbar}>
        <span className={styles.tabCount}>显示 {visibleMembers.length}/{members.length} 位成员</span>
      </div>

      <div className={styles.memberTable}>
        <div className={styles.memberTableHead}>
          <span />
          <span>成员</span>
          <span>角色</span>
          <span>状态</span>
          <span>操作</span>
        </div>
        {visibleMembers.map((member) => {
          const roleColor = memberPrimaryRoleColor(member)
          const roles = projectMemberRoleRefs(member)
          const name = member.account ?? member.user_id ?? '-'
          const status = memberPresenceStatus(member)
          const canSelect = canBatchMembers && member.user_id !== currentUserId
          return (
            <div key={member.user_id} className={styles.memberTableRow} data-selected={selectedIds.has(member.user_id) ? 'true' : undefined}>
              <label className={styles.memberTableCheck}>
                <input
                  type="checkbox"
                  checked={selectedIds.has(member.user_id)}
                  disabled={!canSelect}
                  onChange={() => toggleMember(member)}
                  aria-label={`选择 ${name}`}
                />
              </label>
              <div className={styles.memberTableIdentity}>
                <span className={styles.memberAvatar} style={roleColor ? { boxShadow: `inset 0 0 0 2px ${roleColor}` } : undefined}>
                  {member.avatar_data_url ? <img src={member.avatar_data_url} alt="" /> : memberInitial(member)}
                </span>
                <span>
                  <strong style={roleColor ? { color: roleColor } : undefined}>{name}</strong>
                  <em>{member.user_id}</em>
                </span>
              </div>
              <div className={styles.memberRoleStack}>
                {roles.slice(0, 3).map((role) => (
                  <em key={role.id} style={role.color ? { color: role.color, borderColor: role.color } : undefined}>
                    {role.name || roleLabel(role.id)}
                  </em>
                ))}
                {roles.length > 3 && <em>+{roles.length - 3}</em>}
              </div>
              <div className={styles.memberStatusCell}>
                <strong>{presenceLabel(status)}</strong>
                <span>{memberStatusDetail(member)}</span>
              </div>
              <div className={styles.memberRowActions}>
                <button
                  className={styles.textBtn}
                  title="移除成员"
                  disabled={!canManageMembers || member.user_id === currentUserId || removing === member.user_id}
                  onClick={() => handleRemove(member.user_id, name)}
                  type="button"
                >
                  {removing === member.user_id ? '移除中' : '移除'}
                </button>
              </div>
            </div>
          )
        })}
        {visibleMembers.length === 0 && <p className={styles.empty}>暂无匹配成员</p>}
      </div>
    </div>
  )
}

function projectMemberStats(members: ProjectMember[]) {
  return members.reduce((stats, member) => {
    const status = memberPresenceStatus(member)
    stats.total += 1
    if (status === 'offline') stats.offline += 1
    else stats.online += 1
    if (member.is_banned || member.is_muted) stats.restricted += 1
    return stats
  }, { total: 0, online: 0, offline: 0, restricted: 0 })
}

function projectMemberStatCount(stats: ReturnType<typeof projectMemberStats>, filter: ProjectMemberStatusFilter) {
  if (filter === 'online') return stats.online
  if (filter === 'offline') return stats.offline
  if (filter === 'restricted') return stats.restricted
  return stats.total
}

function matchesProjectMemberStatus(member: ProjectMember, filter: ProjectMemberStatusFilter) {
  if (filter === 'all') return true
  if (filter === 'restricted') return !!(member.is_banned || member.is_muted)
  const status = memberPresenceStatus(member)
  if (filter === 'online') return status !== 'offline'
  return status === 'offline'
}

function projectMemberRoleRefs(member: ProjectMember) {
  if (member.roles?.length) return member.roles
  const roleId = member.role ?? 'member'
  return [{ id: roleId, name: roleLabel(roleId), position: 0 }]
}

function projectMemberRoleOptions(members: ProjectMember[]) {
  const options = new Map<string, { id: string; label: string; color?: string | null; position: number; count: number }>()
  members.forEach((member) => {
    projectMemberRoleRefs(member).forEach((role) => {
      const id = String(role.id || role.name || '').trim().toLowerCase()
      if (!id) return
      const current = options.get(id)
      if (current) {
        current.count += 1
        current.position = Math.max(current.position, role.position ?? 0)
        if (!current.color && role.color) current.color = role.color
        return
      }
      options.set(id, {
        id,
        label: role.name || roleLabel(id),
        color: role.color,
        position: role.position ?? 0,
        count: 1,
      })
    })
  })
  return Array.from(options.values())
    .sort((left, right) => right.position - left.position || right.count - left.count || left.label.localeCompare(right.label))
}

function inviteRoleOptions(roleOptions: ReturnType<typeof projectMemberRoleOptions>) {
  if (roleOptions.length) return roleOptions
  return [
    { id: 'member', label: '成员' },
    { id: 'editor', label: '协作者' },
    { id: 'admin', label: '管理员' },
    { id: 'observer', label: '只读成员' },
  ]
}

function projectMemberHasRole(member: ProjectMember, roleId: string) {
  const target = roleId.trim().toLowerCase()
  return projectMemberRoleRefs(member).some((role) =>
    String(role.id || role.name || '').trim().toLowerCase() === target
  )
}

function sortProjectMembers(members: ProjectMember[], sortMode: ProjectMemberSortMode) {
  return [...members].sort((left, right) => {
    if (sortMode === 'name') return projectMemberName(left).localeCompare(projectMemberName(right))
    if (sortMode === 'joined') return joinedTime(right) - joinedTime(left) || projectMemberName(left).localeCompare(projectMemberName(right))
    return memberPresenceRank(left) - memberPresenceRank(right)
      || projectMemberTopRolePosition(right) - projectMemberTopRolePosition(left)
      || projectMemberName(left).localeCompare(projectMemberName(right))
  })
}

function projectMemberTopRolePosition(member: ProjectMember) {
  return projectMemberRoleRefs(member)[0]?.position ?? 0
}

function projectMemberName(member: ProjectMember) {
  return member.account || member.user_id
}

function joinedTime(member: ProjectMember) {
  const timestamp = Date.parse(member.joined_at ?? '')
  return Number.isFinite(timestamp) ? timestamp : 0
}

function memberPresenceRank(member: ProjectMember) {
  if (member.is_banned) return 5
  if (member.is_muted) return 4
  const status = memberPresenceStatus(member)
  if (status === 'online') return 1
  if (status === 'idle') return 2
  if (status === 'dnd') return 3
  return 6
}

function memberStatusDetail(member: ProjectMember) {
  if (member.is_banned || member.is_muted) return memberModerationSummary(member)
  return member.activity || member.custom_status || memberRoleSummary(member)
}

function projectMemberBatchNote(action: ProjectMemberBatchAction) {
  if (action === 'mute1h') return 'PC 成员管理页批量禁言 1 小时'
  if (action === 'mute1d') return 'PC 成员管理页批量禁言 1 天'
  if (action === 'unmute') return 'PC 成员管理页批量解禁言'
  return 'PC 成员管理页批量移除'
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
    ['磁盘剩余', diskFreeBytes
      ? `${(diskFreeBytes / 1024 / 1024 / 1024).toFixed(1)} GB`
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
      {issues.length > 0 && (
        <div className={styles.issues}>
          <strong>问题：</strong>
          {issues.map((issue, i) => <div key={i}>{issue}</div>)}
        </div>
      )}
    </div>
  )
}
