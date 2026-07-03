import { useEffect, useMemo, useRef, useState } from 'react'
import { api } from '../../api/client'
import { clean, formatTime } from '../../lib/utils'
import type { Channel, ProjectMember } from './types'
import {
  filterMembers,
  memberChannelCanView,
  memberChannelPermissions,
  memberInitial,
  memberModerationSummary,
  memberPresenceStatus,
  memberRoleSummary,
  presenceLabel,
  roleLabel,
} from './memberUtils'
import styles from './ConversationPage.module.css'

type DirectoryFilter = 'all' | 'online' | 'offline' | 'restricted' | 'recent'
type DirectorySort = 'status' | 'name' | 'joined'
type DirectoryBatchAction = 'mute1h' | 'mute1d' | 'unmute' | 'remove'

const DIRECTORY_FILTERS: Array<{ id: DirectoryFilter; label: string }> = [
  { id: 'all', label: '全部' },
  { id: 'online', label: '在线' },
  { id: 'offline', label: '离线' },
  { id: 'restricted', label: '受限' },
  { id: 'recent', label: '新加入' },
]

const RECENT_JOIN_MS = 7 * 24 * 60 * 60 * 1000
const DIRECTORY_ROW_HEIGHT = 82
const DIRECTORY_LIST_OVERSCAN = 6
const DIRECTORY_LIST_MIN_WINDOW = 10

export function MemberDirectoryDrawer({
  projectId,
  members,
  channels,
  currentUserId,
  canManageMembers,
  canManageRoles,
  canModerate,
  onSaved,
  onClose,
  onOpenDetails,
  onOpenConversations,
  onOpenRoles,
  onOpenModerationCenter,
}: {
  projectId: string
  members: ProjectMember[]
  channels: Channel[]
  currentUserId?: string
  canManageMembers?: boolean
  canManageRoles?: boolean
  canModerate?: boolean
  onSaved?: () => Promise<void>
  onClose: () => void
  onOpenDetails?: (member: ProjectMember) => void
  onOpenConversations?: (member: ProjectMember) => void
  onOpenRoles?: (member: ProjectMember) => void
  onOpenModerationCenter?: (member?: ProjectMember) => void
}) {
  const [query, setQuery] = useState('')
  const [filter, setFilter] = useState<DirectoryFilter>('all')
  const [sortMode, setSortMode] = useState<DirectorySort>('status')
  const [roleFilter, setRoleFilter] = useState('')
  const [listScrollTop, setListScrollTop] = useState(0)
  const [listHeight, setListHeight] = useState(0)
  const [selectedIds, setSelectedIds] = useState<Set<string>>(() => new Set())
  const [batchBusy, setBatchBusy] = useState<DirectoryBatchAction | ''>('')
  const [batchMessage, setBatchMessage] = useState('')
  const listRef = useRef<HTMLDivElement | null>(null)
  const stats = useMemo(() => memberDirectoryStats(members), [members])
  const roleOptions = useMemo(() => memberDirectoryRoles(members), [members])
  const visibleMembers = useMemo(() => {
    const searched = filterMembers(members, query)
    return sortDirectoryMembers(
      searched.filter((member) => matchesDirectoryFilter(member, filter))
        .filter((member) => !roleFilter || memberHasDirectoryRole(member, roleFilter)),
      sortMode,
    )
  }, [members, query, filter, roleFilter, sortMode])
  const listWindowSize = Math.max(
    DIRECTORY_LIST_MIN_WINDOW,
    Math.ceil((listHeight || DIRECTORY_ROW_HEIGHT * DIRECTORY_LIST_MIN_WINDOW) / DIRECTORY_ROW_HEIGHT) + DIRECTORY_LIST_OVERSCAN * 2,
  )
  const listStart = Math.max(0, Math.floor(listScrollTop / DIRECTORY_ROW_HEIGHT) - DIRECTORY_LIST_OVERSCAN)
  const listEnd = Math.min(visibleMembers.length, listStart + listWindowSize)
  const virtualMembers = visibleMembers.slice(listStart, listEnd)
  const canBatchManage = !!(projectId && (canModerate || canManageMembers))
  const selectedMembers = useMemo(
    () => members.filter((member) => selectedIds.has(member.user_id) && member.user_id !== currentUserId),
    [members, selectedIds, currentUserId],
  )
  const selectableVisibleMembers = useMemo(
    () => visibleMembers.filter((member) => member.user_id !== currentUserId),
    [visibleMembers, currentUserId],
  )
  const selectedVisibleCount = selectableVisibleMembers.filter((member) => selectedIds.has(member.user_id)).length
  const allVisibleSelected = selectableVisibleMembers.length > 0 && selectedVisibleCount === selectableVisibleMembers.length

  useEffect(() => {
    setListScrollTop(0)
    if (listRef.current) listRef.current.scrollTop = 0
  }, [query, filter, roleFilter, sortMode, visibleMembers.length])

  useEffect(() => {
    const node = listRef.current
    if (!node) return
    const updateHeight = () => setListHeight(node.clientHeight)
    updateHeight()
    if (typeof ResizeObserver === 'undefined') {
      window.addEventListener('resize', updateHeight)
      return () => window.removeEventListener('resize', updateHeight)
    }
    const observer = new ResizeObserver(updateHeight)
    observer.observe(node)
    return () => observer.disconnect()
  }, [])

  useEffect(() => {
    const validIds = new Set(members.map((member) => member.user_id))
    setSelectedIds((current) => {
      const next = new Set(Array.from(current).filter((id) => validIds.has(id) && id !== currentUserId))
      return next.size === current.size ? current : next
    })
  }, [members, currentUserId])

  function openDetails(member: ProjectMember) {
    onOpenDetails?.(member)
    onClose()
  }

  function openConversations(member: ProjectMember) {
    onOpenConversations?.(member)
    onClose()
  }

  function openRoles(member: ProjectMember) {
    onOpenRoles?.(member)
    onClose()
  }

  function openModeration(member?: ProjectMember) {
    onOpenModerationCenter?.(member)
    onClose()
  }

  function toggleMemberSelection(member: ProjectMember) {
    if (!canBatchManage || member.user_id === currentUserId) return
    setSelectedIds((current) => {
      const next = new Set(current)
      if (next.has(member.user_id)) next.delete(member.user_id)
      else next.add(member.user_id)
      return next
    })
    setBatchMessage('')
  }

  function toggleVisibleSelection() {
    if (!canBatchManage || selectableVisibleMembers.length === 0) return
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

  async function runBatchAction(action: DirectoryBatchAction) {
    if (!projectId || batchBusy) return
    const targets = selectedMembers.filter((member) => member.user_id !== currentUserId)
    if (targets.length === 0) {
      setBatchMessage('请先选择要处理的成员')
      return
    }
    if (action === 'remove' && !canManageMembers) return
    if (action !== 'remove' && !canModerate) return
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
            note: batchActionNote(action),
          })
        }
      }
      setSelectedIds(new Set())
      setBatchMessage(`已批量更新 ${targets.length} 位成员`)
      await onSaved?.()
    } catch (err) {
      setBatchMessage((err as { message?: string }).message ?? '批量操作失败')
    } finally {
      setBatchBusy('')
    }
  }

  return (
    <div className={styles.drawerBackdrop}>
      <section className={[styles.permissionDrawer, styles.memberDirectoryDrawer].join(' ')} role="dialog" aria-modal="true">
        <header className={styles.drawerHeader}>
          <div>
            <strong>成员目录</strong>
            <span>全项目成员视角 · {members.length} 位成员 · {stats.online} 在线 · {stats.restricted} 受限</span>
          </div>
          <div className={styles.drawerHeaderActions}>
            {canModerate && <button className={styles.drawerCloseBtn} onClick={() => openModeration()}>限制中心</button>}
            <button className={styles.drawerCloseBtn} onClick={onClose}>关闭</button>
          </div>
        </header>

        <div className={styles.memberDirectoryBody}>
          <div className={styles.memberDirectoryStats}>
            {DIRECTORY_FILTERS.map((item) => (
              <button
                key={item.id}
                type="button"
                data-active={filter === item.id ? 'true' : undefined}
                onClick={() => setFilter(item.id)}
              >
                <strong>{directoryStatCount(stats, item.id)}</strong>
                <span>{item.label}</span>
              </button>
            ))}
          </div>

          <div className={styles.memberDirectoryToolbar}>
            <input
              className={styles.drawerSearchInput}
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="搜索账号、ID、角色、状态"
            />
            <select value={roleFilter} onChange={(event) => setRoleFilter(event.target.value)} aria-label="按角色筛选">
              <option value="">全部角色</option>
              {roleOptions.map((role) => (
                <option key={role.id} value={role.id}>{role.label} ({role.count})</option>
              ))}
            </select>
            <select value={sortMode} onChange={(event) => setSortMode(event.target.value as DirectorySort)} aria-label="成员排序">
              <option value="status">按状态</option>
              <option value="name">按名称</option>
              <option value="joined">按加入时间</option>
            </select>
          </div>

          <div className={styles.memberDirectoryMeta}>
            显示 {visibleMembers.length}/{members.length}
            {roleFilter ? ` · 角色 ${roleOptions.find((role) => role.id === roleFilter)?.label ?? roleFilter}` : ''}
            {filter !== 'all' ? ` · ${DIRECTORY_FILTERS.find((item) => item.id === filter)?.label ?? filter}` : ''}
          </div>

          {canBatchManage && (
            <section className={styles.memberDirectoryBatchBar}>
              <div className={styles.memberDirectoryBatchCopy}>
                <strong>批量管理</strong>
                <span>{selectedMembers.length > 0 ? `已选择 ${selectedMembers.length} 位成员` : '勾选成员后批量禁言、解禁言或移出项目'}</span>
              </div>
              <div className={styles.memberDirectoryBatchActions}>
                <button
                  className={styles.drawerCloseBtn}
                  type="button"
                  disabled={selectableVisibleMembers.length === 0 || !!batchBusy}
                  onClick={toggleVisibleSelection}
                >
                  {allVisibleSelected ? '取消当前' : '选择当前'}
                </button>
                <button
                  className={styles.drawerCloseBtn}
                  type="button"
                  disabled={selectedMembers.length === 0 || !!batchBusy}
                  onClick={() => setSelectedIds(new Set())}
                >
                  清空
                </button>
                {canModerate && (
                  <>
                    <button
                      className={styles.drawerCloseBtn}
                      type="button"
                      disabled={selectedMembers.length === 0 || !!batchBusy}
                      onClick={() => runBatchAction('mute1h')}
                    >
                      禁言 1 小时
                    </button>
                    <button
                      className={styles.drawerCloseBtn}
                      type="button"
                      disabled={selectedMembers.length === 0 || !!batchBusy}
                      onClick={() => runBatchAction('mute1d')}
                    >
                      禁言 1 天
                    </button>
                    <button
                      className={styles.drawerCloseBtn}
                      type="button"
                      disabled={selectedMembers.length === 0 || !!batchBusy}
                      onClick={() => runBatchAction('unmute')}
                    >
                      解禁言
                    </button>
                  </>
                )}
                {canManageMembers && (
                  <button
                    className={styles.drawerCloseBtn}
                    type="button"
                    data-danger="true"
                    disabled={selectedMembers.length === 0 || !!batchBusy}
                    onClick={() => runBatchAction('remove')}
                  >
                    批量移除
                  </button>
                )}
              </div>
              {batchMessage && <p>{batchMessage}</p>}
            </section>
          )}

          <div
            ref={listRef}
            className={styles.memberDirectoryList}
            onScroll={(event) => setListScrollTop(event.currentTarget.scrollTop)}
          >
            {visibleMembers.length === 0 && <p className={styles.sideHint}>没有匹配成员</p>}
            {visibleMembers.length > 0 && (
              <div className={styles.memberDirectoryVirtualCanvas} style={{ height: visibleMembers.length * DIRECTORY_ROW_HEIGHT }}>
                <div style={{ transform: `translateY(${listStart * DIRECTORY_ROW_HEIGHT}px)` }}>
                  {virtualMembers.map((member) => (
                    <div key={member.user_id} className={styles.memberDirectoryVirtualSlot}>
                      <MemberDirectoryRow
                        member={member}
                        channels={channels}
                        canSelect={canBatchManage && member.user_id !== currentUserId}
                        selected={selectedIds.has(member.user_id)}
                        canManageRoles={canManageRoles}
                        canModerate={canModerate}
                        onToggleSelected={toggleMemberSelection}
                        onOpenDetails={openDetails}
                        onOpenConversations={onOpenConversations ? openConversations : undefined}
                        onOpenRoles={canManageRoles && onOpenRoles ? openRoles : undefined}
                        onOpenModerationCenter={canModerate && onOpenModerationCenter ? openModeration : undefined}
                      />
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        </div>
      </section>
    </div>
  )
}

function MemberDirectoryRow({
  member,
  channels,
  canSelect,
  selected,
  canManageRoles,
  canModerate,
  onToggleSelected,
  onOpenDetails,
  onOpenConversations,
  onOpenRoles,
  onOpenModerationCenter,
}: {
  member: ProjectMember
  channels: Channel[]
  canSelect?: boolean
  selected?: boolean
  canManageRoles?: boolean
  canModerate?: boolean
  onToggleSelected?: (member: ProjectMember) => void
  onOpenDetails: (member: ProjectMember) => void
  onOpenConversations?: (member: ProjectMember) => void
  onOpenRoles?: (member: ProjectMember) => void
  onOpenModerationCenter?: (member: ProjectMember) => void
}) {
  const status = memberPresenceStatus(member)
  const visibleChannels = visibleChannelCount(member, channels)
  const joined = member.joined_at ? formatTime(member.joined_at) : '未知'
  const state = member.is_banned ? 'banned' : member.is_muted ? 'muted' : status
  return (
    <article className={styles.memberDirectoryRow} data-state={state} data-selectable={canSelect ? 'true' : undefined} data-selected={selected ? 'true' : undefined}>
      {canSelect && (
        <label className={styles.memberDirectorySelect} title={`选择 ${memberName(member)}`}>
          <input
            type="checkbox"
            checked={!!selected}
            onChange={() => onToggleSelected?.(member)}
            onClick={(event) => event.stopPropagation()}
            aria-label={`选择 ${memberName(member)}`}
          />
        </label>
      )}
      <span className={[styles.memberAvatar, directoryAvatarClass(status)].join(' ')}>
        {member.avatar_data_url
          ? <img src={member.avatar_data_url} alt="" />
          : memberInitial(member)
        }
      </span>
      <div className={styles.memberDirectoryMain}>
        <strong title={member.account || member.user_id}>{member.account || member.user_id}</strong>
        <span>{presenceLabel(status)} · {memberRoleSummary(member)}</span>
        <div className={styles.memberDirectoryBadges}>
          <em>{memberModerationSummary(member)}</em>
          <em>可见频道 {visibleChannels}/{channels.length}</em>
          <em>加入 {joined}</em>
        </div>
      </div>
      <div className={styles.memberDirectoryActions}>
        <button className={styles.drawerCloseBtn} type="button" onClick={() => onOpenDetails(member)}>资料</button>
        {onOpenConversations && <button className={styles.drawerCloseBtn} type="button" onClick={() => onOpenConversations(member)}>会话</button>}
        {canManageRoles && onOpenRoles && <button className={styles.drawerCloseBtn} type="button" onClick={() => onOpenRoles(member)}>角色</button>}
        {canModerate && onOpenModerationCenter && <button className={styles.drawerCloseBtn} type="button" onClick={() => onOpenModerationCenter(member)}>限制</button>}
      </div>
    </article>
  )
}

function memberDirectoryStats(members: ProjectMember[]) {
  return members.reduce((stats, member) => {
    const status = memberPresenceStatus(member)
    stats.total += 1
    if (status === 'offline') stats.offline += 1
    else stats.online += 1
    if (member.is_banned || member.is_muted) stats.restricted += 1
    if (isRecentMember(member)) stats.recent += 1
    return stats
  }, { total: 0, online: 0, offline: 0, restricted: 0, recent: 0 })
}

function directoryStatCount(stats: ReturnType<typeof memberDirectoryStats>, filter: DirectoryFilter) {
  if (filter === 'online') return stats.online
  if (filter === 'offline') return stats.offline
  if (filter === 'restricted') return stats.restricted
  if (filter === 'recent') return stats.recent
  return stats.total
}

function matchesDirectoryFilter(member: ProjectMember, filter: DirectoryFilter) {
  if (filter === 'all') return true
  if (filter === 'restricted') return !!(member.is_banned || member.is_muted)
  if (filter === 'recent') return isRecentMember(member)
  const status = memberPresenceStatus(member)
  if (filter === 'online') return status !== 'offline'
  return status === 'offline'
}

function isRecentMember(member: ProjectMember) {
  const timestamp = Date.parse(member.joined_at ?? '')
  return Number.isFinite(timestamp) && Date.now() - timestamp <= RECENT_JOIN_MS
}

function sortDirectoryMembers(members: ProjectMember[], sortMode: DirectorySort) {
  return [...members].sort((left, right) => {
    if (sortMode === 'name') return memberName(left).localeCompare(memberName(right))
    if (sortMode === 'joined') return joinedTime(right) - joinedTime(left) || memberName(left).localeCompare(memberName(right))
    return presenceRank(left) - presenceRank(right) || memberName(left).localeCompare(memberName(right))
  })
}

function memberDirectoryRoles(members: ProjectMember[]) {
  const roles = new Map<string, { id: string; label: string; count: number; position: number }>()
  members.forEach((member) => {
    const memberRoles = member.roles?.length
      ? member.roles
      : [{ id: member.role ?? 'member', name: roleLabel(member.role ?? 'member'), position: 0 }]
    const seen = new Set<string>()
    memberRoles.forEach((role) => {
      const id = clean(role.id ?? role.name ?? '').toLowerCase()
      if (!id || seen.has(id)) return
      seen.add(id)
      const current = roles.get(id)
      if (current) {
        current.count += 1
        current.position = Math.max(current.position, role.position ?? 0)
        return
      }
      roles.set(id, {
        id,
        label: role.name || roleLabel(id),
        count: 1,
        position: role.position ?? 0,
      })
    })
  })
  return Array.from(roles.values())
    .sort((left, right) => right.position - left.position || right.count - left.count || left.label.localeCompare(right.label))
}

function memberHasDirectoryRole(member: ProjectMember, roleId: string) {
  const target = clean(roleId).toLowerCase()
  if (!target) return true
  const ids = [
    member.role,
    ...(member.roles ?? []).map((role) => role.id || role.name),
  ].map((id) => clean(id ?? '').toLowerCase()).filter(Boolean)
  return ids.includes(target)
}

function visibleChannelCount(member: ProjectMember, channels: Channel[]) {
  return channels.filter((channel) => {
    const permissions = memberChannelPermissions(member, channel.id)
    return !permissions || memberChannelCanView(permissions)
  }).length
}

function memberName(member: ProjectMember) {
  return member.account || member.user_id
}

function batchActionNote(action: DirectoryBatchAction) {
  if (action === 'mute1h') return 'PC 成员目录批量禁言 1 小时'
  if (action === 'mute1d') return 'PC 成员目录批量禁言 1 天'
  if (action === 'unmute') return 'PC 成员目录批量解禁言'
  return 'PC 成员目录批量移除'
}

function joinedTime(member: ProjectMember) {
  const timestamp = Date.parse(member.joined_at ?? '')
  return Number.isFinite(timestamp) ? timestamp : 0
}

function presenceRank(member: ProjectMember) {
  if (member.is_banned) return 5
  if (member.is_muted) return 4
  const status = memberPresenceStatus(member)
  if (status === 'online') return 1
  if (status === 'idle') return 2
  if (status === 'dnd') return 3
  return 6
}

function directoryAvatarClass(status: string) {
  if (status === 'online') return styles.memberAvatarOnline
  if (status === 'idle') return styles.memberAvatarIdle
  if (status === 'dnd') return styles.memberAvatarDnd
  return styles.memberAvatarOffline
}
