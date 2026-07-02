import { useMemo, useState } from 'react'
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

const DIRECTORY_FILTERS: Array<{ id: DirectoryFilter; label: string }> = [
  { id: 'all', label: '全部' },
  { id: 'online', label: '在线' },
  { id: 'offline', label: '离线' },
  { id: 'restricted', label: '受限' },
  { id: 'recent', label: '新加入' },
]

const RECENT_JOIN_MS = 7 * 24 * 60 * 60 * 1000

export function MemberDirectoryDrawer({
  members,
  channels,
  canManageRoles,
  canModerate,
  onClose,
  onOpenDetails,
  onOpenConversations,
  onOpenRoles,
  onOpenModerationCenter,
}: {
  members: ProjectMember[]
  channels: Channel[]
  canManageRoles?: boolean
  canModerate?: boolean
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

          <div className={styles.memberDirectoryList}>
            {visibleMembers.length === 0 && <p className={styles.sideHint}>没有匹配成员</p>}
            {visibleMembers.map((member) => (
              <MemberDirectoryRow
                key={member.user_id}
                member={member}
                channels={channels}
                canManageRoles={canManageRoles}
                canModerate={canModerate}
                onOpenDetails={openDetails}
                onOpenConversations={onOpenConversations ? openConversations : undefined}
                onOpenRoles={canManageRoles && onOpenRoles ? openRoles : undefined}
                onOpenModerationCenter={canModerate && onOpenModerationCenter ? openModeration : undefined}
              />
            ))}
          </div>
        </div>
      </section>
    </div>
  )
}

function MemberDirectoryRow({
  member,
  channels,
  canManageRoles,
  canModerate,
  onOpenDetails,
  onOpenConversations,
  onOpenRoles,
  onOpenModerationCenter,
}: {
  member: ProjectMember
  channels: Channel[]
  canManageRoles?: boolean
  canModerate?: boolean
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
    <article className={styles.memberDirectoryRow} data-state={state}>
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
