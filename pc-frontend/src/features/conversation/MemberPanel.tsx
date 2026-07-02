import { useState, useEffect, useMemo, useRef } from 'react'
import { api } from '../../api/client'
import { clean, formatTime } from '../../lib/utils'
import type { Channel, ProjectMember, ProjectMemberAuditEntry, ProjectMemberAuditResponse, ProjectRoleRef } from './types'
import {
  memberPresenceStatus,
  memberChannelSubtitle,
  memberChannelPermissions,
  memberChannelCanView,
  memberChannelCapabilityLabels,
  memberModerationSummary,
  memberRoleSummary,
  presenceLabel,
  memberPrimaryRoleKey,
  roleLabel,
} from './memberUtils'
import styles from './ConversationPage.module.css'

/* ── 虚拟列表类型和常量 ── */
export type MemberVirtualRow =
  | { kind: 'status-header'; id: string; label: string; count: number; collapsed: boolean }
  | { kind: 'role-header'; id: string; label: string; count: number }
  | { kind: 'member'; id: string; member: ProjectMember }

export const MEMBER_VIRTUAL_ROW_HEIGHT = 48
const MEMBER_LIST_OVERSCAN = 6
const MEMBER_LIST_WINDOW = 28
const MEMBER_LIST_MIN_WINDOW = 12
const MEMBER_PANEL_COLLAPSED_KEY = 'elon.pc.memberPanel.collapsedStatusSections.v1'
const MEMBER_PANEL_FILTERS_KEY = 'elon.pc.memberPanel.filters.v1'
type MemberStatusSectionId = 'online' | 'offline'
type MemberPanelStatusFilter = 'all' | 'online' | 'offline' | 'restricted'
type MemberPanelSortMode = 'role' | 'name' | 'joined'
type MemberRoleFilterOption = { id: string; label: string; count: number }
type MemberPanelFilterPrefs = {
  scope: string
  statusFilter: MemberPanelStatusFilter
  roleFilter: string
  sortMode: MemberPanelSortMode
}
export type MemberModerationAction = 'mute' | 'unmute' | 'ban' | 'unban'
export type MemberMenuRequest = { member: ProjectMember; x: number; y: number }

const MEMBER_STATUS_FILTERS: Array<{ id: MemberPanelStatusFilter; label: string }> = [
  { id: 'all', label: '全部' },
  { id: 'online', label: '在线' },
  { id: 'offline', label: '离线' },
  { id: 'restricted', label: '受限' },
]

const DEFAULT_MEMBER_PANEL_FILTERS: Omit<MemberPanelFilterPrefs, 'scope'> = {
  statusFilter: 'all',
  roleFilter: '',
  sortMode: 'role',
}

/* ── 角色分组 ── */
export const ROLE_GROUPS = [
  { id: 'owner', label: '拥有者' },
  { id: 'admin', label: '管理员' },
  { id: 'builder', label: '开发协作' },
  { id: 'reviewer', label: '测试与观察' },
  { id: 'restricted', label: '受限成员' },
  { id: 'member', label: '成员' },
] as const

type RoleGroupId = typeof ROLE_GROUPS[number]['id']

function memberRoleGroup(member: ProjectMember): RoleGroupId {
  if (member.is_banned || member.is_muted) return 'restricted'
  const keys = memberRoleKeys(member)
  if (keys.some(key => ['owner', '拥有者'].includes(key))) return 'owner'
  if (keys.some(key => ['admin', 'administrator', '管理员'].includes(key))) return 'admin'
  if (keys.some(key => ['developer', 'dev', 'maintainer', 'editor', 'collaborator', 'builder', '开发者', '维护者', '协作者', '构建发布员'].includes(key))) return 'builder'
  if (keys.some(key => ['tester', 'qa', 'observer', 'viewer', 'guest', 'readonly', '只读成员', '观察者', '测试者', '访客'].includes(key))) return 'reviewer'
  return 'member'
}

function memberRoleKeys(member: ProjectMember) {
  return [
    member.role,
    ...(member.roles ?? []).flatMap(role => [role.id, role.name]),
  ].filter(Boolean).map(value => clean(String(value)).toLowerCase())
}

function memberPrimaryRoleLabel(member: ProjectMember) {
  const role = member.roles?.[0]
  if (role?.name) return role.name
  const labels: Record<string, string> = {
    owner: '拥有者', admin: '管理员', editor: '协作者',
    developer: '开发者', maintainer: '维护者', member: '成员', observer: '只读成员',
  }
  return labels[member.role ?? 'member'] ?? (member.role ?? 'member')
}

function memberRolePosition(member: ProjectMember) {
  return member.roles?.[0]?.position ?? 0
}

function sortMembersForPanel(members: ProjectMember[]) {
  return [...members].sort(compareMembersForPanel)
}

export function compareMembersForPanel(left: ProjectMember, right: ProjectMember) {
  const leftStatus = memberPresenceStatus(left)
  const rightStatus = memberPresenceStatus(right)
  const leftOnline = leftStatus === 'offline' ? 0 : 1
  const rightOnline = rightStatus === 'offline' ? 0 : 1
  return rightOnline - leftOnline
    || memberRolePosition(right) - memberRolePosition(left)
    || clean(left.account ?? left.user_id).localeCompare(clean(right.account ?? right.user_id))
}

function memberRolePillClass(roleKey: string) {
  if (roleKey === 'owner') return styles.memberRolePillOwner
  if (roleKey === 'admin') return styles.memberRolePillAdmin
  if (['developer', 'dev', 'maintainer', 'editor', 'collaborator', 'builder'].includes(roleKey)) return styles.memberRolePillEditor
  if (['tester', 'qa', 'observer', 'viewer', 'guest'].includes(roleKey)) return styles.memberRolePillObserver
  return ''
}

export function memberAvatarRoleClass(roleKey: string) {
  if (roleKey === 'owner') return styles.memberAvatarOwner
  if (roleKey === 'admin') return styles.memberAvatarAdmin
  if (['developer', 'dev', 'maintainer', 'editor', 'collaborator', 'builder'].includes(roleKey)) return styles.memberAvatarEditor
  if (['tester', 'qa', 'observer', 'viewer', 'guest'].includes(roleKey)) return styles.memberAvatarObserver
  return ''
}

export function memberPresenceAvatarClass(status: string) {
  if (status === 'idle') return styles.memberAvatarIdle
  if (status === 'dnd') return styles.memberAvatarDnd
  if (status === 'offline') return styles.memberAvatarOffline
  return styles.memberAvatarOnline
}

function memberPresencePillClass(status: string) {
  if (status === 'idle') return styles.memberPresencePillIdle
  if (status === 'dnd') return styles.memberPresencePillDnd
  if (status === 'offline') return styles.memberPresencePillOffline
  return styles.memberPresencePillOnline
}

function readCollapsedStatusSections(): Record<MemberStatusSectionId, boolean> {
  if (typeof window === 'undefined') return { online: false, offline: false }
  try {
    const value = window.localStorage.getItem(MEMBER_PANEL_COLLAPSED_KEY)
    if (!value) return { online: false, offline: false }
    const parsed = JSON.parse(value) as Partial<Record<MemberStatusSectionId, boolean>>
    return { online: !!parsed.online, offline: !!parsed.offline }
  } catch {
    return { online: false, offline: false }
  }
}

function memberPanelFilterScope(channelId?: string) {
  return channelId ? `channel:${channelId}` : 'project'
}

function isMemberPanelStatusFilter(value: unknown): value is MemberPanelStatusFilter {
  return value === 'all' || value === 'online' || value === 'offline' || value === 'restricted'
}

function isMemberPanelSortMode(value: unknown): value is MemberPanelSortMode {
  return value === 'role' || value === 'name' || value === 'joined'
}

function readMemberPanelFilters(scope: string): MemberPanelFilterPrefs {
  if (typeof window === 'undefined') return { scope, ...DEFAULT_MEMBER_PANEL_FILTERS }
  try {
    const value = window.localStorage.getItem(MEMBER_PANEL_FILTERS_KEY)
    if (!value) return { scope, ...DEFAULT_MEMBER_PANEL_FILTERS }
    const parsed = JSON.parse(value) as Record<string, Partial<MemberPanelFilterPrefs>>
    const prefs = parsed[scope] ?? {}
    return {
      scope,
      statusFilter: isMemberPanelStatusFilter(prefs.statusFilter) ? prefs.statusFilter : DEFAULT_MEMBER_PANEL_FILTERS.statusFilter,
      roleFilter: typeof prefs.roleFilter === 'string' ? prefs.roleFilter : DEFAULT_MEMBER_PANEL_FILTERS.roleFilter,
      sortMode: isMemberPanelSortMode(prefs.sortMode) ? prefs.sortMode : DEFAULT_MEMBER_PANEL_FILTERS.sortMode,
    }
  } catch {
    return { scope, ...DEFAULT_MEMBER_PANEL_FILTERS }
  }
}

function writeMemberPanelFilters(prefs: MemberPanelFilterPrefs) {
  if (typeof window === 'undefined') return
  try {
    const value = window.localStorage.getItem(MEMBER_PANEL_FILTERS_KEY)
    const parsed = value ? JSON.parse(value) as Record<string, Partial<MemberPanelFilterPrefs>> : {}
    parsed[prefs.scope] = {
      statusFilter: prefs.statusFilter,
      roleFilter: prefs.roleFilter,
      sortMode: prefs.sortMode,
    }
    window.localStorage.setItem(MEMBER_PANEL_FILTERS_KEY, JSON.stringify(parsed))
  } catch {
    window.localStorage.setItem(MEMBER_PANEL_FILTERS_KEY, JSON.stringify({
      [prefs.scope]: {
        statusFilter: prefs.statusFilter,
        roleFilter: prefs.roleFilter,
        sortMode: prefs.sortMode,
      },
    }))
  }
}

function buildMemberRows(
  members: ProjectMember[],
  collapsedStatusSections: Record<MemberStatusSectionId, boolean>,
  sortMode: MemberPanelSortMode = 'role',
): MemberVirtualRow[] {
  const buckets = [
    {
      id: 'online' as const,
      label: '在线',
      members: members.filter((member) => memberPresenceStatus(member) !== 'offline'),
    },
    {
      id: 'offline' as const,
      label: '离线',
      members: members.filter((member) => memberPresenceStatus(member) === 'offline'),
    },
  ]
  return buckets.flatMap((bucket): MemberVirtualRow[] => {
    if (!bucket.members.length) return []
    const collapsed = !!collapsedStatusSections[bucket.id]
    if (collapsed) {
      return [{ kind: 'status-header' as const, id: `status-${bucket.id}`, label: bucket.label, count: bucket.members.length, collapsed }]
    }
    if (sortMode !== 'role') {
      return [
        { kind: 'status-header' as const, id: `status-${bucket.id}`, label: bucket.label, count: bucket.members.length, collapsed },
        ...sortMembersByMode(bucket.members, sortMode).map(member => ({ kind: 'member' as const, id: `${bucket.id}-${member.user_id}`, member })),
      ]
    }
    const roleRows = ROLE_GROUPS.flatMap((group) => {
      const list = sortMembersForPanel(bucket.members.filter((member) => memberRoleGroup(member) === group.id))
      if (!list.length) return []
      return [
        { kind: 'role-header' as const, id: `role-${bucket.id}-${group.id}`, label: group.label, count: list.length },
        ...list.map(member => ({ kind: 'member' as const, id: `${bucket.id}-${member.user_id}`, member })),
      ]
    })
    return [
      { kind: 'status-header' as const, id: `status-${bucket.id}`, label: bucket.label, count: bucket.members.length, collapsed },
      ...roleRows,
    ]
  })
}

function filterVisibleMembers(members: ProjectMember[], query: string) {
  return members.filter(member => {
    const haystack = [
      member.account,
      member.user_id,
      member.role,
      member.custom_status,
      member.activity,
      ...(member.roles ?? []).map(role => role.name || role.id),
    ].join(' ').toLowerCase()
    return haystack.includes(query)
  })
}

function memberDisplayName(member: ProjectMember) {
  return clean(member.account ?? member.user_id)
}

function memberJoinedTime(member: ProjectMember) {
  const timestamp = Date.parse(member.joined_at ?? '')
  return Number.isNaN(timestamp) ? 0 : timestamp
}

function sortMembersByMode(members: ProjectMember[], sortMode: MemberPanelSortMode) {
  if (sortMode === 'name') {
    return [...members].sort((left, right) =>
      memberDisplayName(left).localeCompare(memberDisplayName(right)) || compareMembersForPanel(left, right)
    )
  }
  if (sortMode === 'joined') {
    return [...members].sort((left, right) =>
      memberJoinedTime(right) - memberJoinedTime(left) || compareMembersForPanel(left, right)
    )
  }
  return sortMembersForPanel(members)
}

function memberMatchesStatusFilter(member: ProjectMember, statusFilter: MemberPanelStatusFilter) {
  if (statusFilter === 'all') return true
  if (statusFilter === 'restricted') return !!(member.is_banned || member.is_muted)
  const status = memberPresenceStatus(member)
  if (statusFilter === 'online') return status !== 'offline'
  return status === 'offline'
}

function memberMatchesRoleFilter(member: ProjectMember, roleFilter: string) {
  if (!roleFilter) return true
  const keys = memberRoleKeys(member)
  if (roleFilter === 'member' && keys.length === 0) return true
  return keys.includes(roleFilter)
}

function memberRoleFilterOptions(members: ProjectMember[]): MemberRoleFilterOption[] {
  const options = new Map<string, MemberRoleFilterOption & { position: number }>()
  members.forEach((member) => {
    const roles = member.roles?.length
      ? member.roles
      : [{ id: member.role ?? 'member', name: roleLabel(member.role ?? 'member'), position: memberRolePosition(member) }]
    const seen = new Set<string>()
    roles.forEach((role) => {
      const rawId = role.id || role.name || 'member'
      const id = clean(String(rawId)).toLowerCase()
      if (!id || seen.has(id)) return
      seen.add(id)
      const current = options.get(id)
      if (current) {
        current.count += 1
        current.position = Math.max(current.position, role.position ?? 0)
        return
      }
      options.set(id, {
        id,
        label: role.name || roleLabel(String(rawId)),
        count: 1,
        position: role.position ?? 0,
      })
    })
  })
  return Array.from(options.values())
    .sort((left, right) =>
      right.position - left.position || right.count - left.count || left.label.localeCompare(right.label)
    )
    .map(({ position: _position, ...option }) => option)
}

/* ── MemberSearch ── */
export function MemberSearch({
  members,
  onSelect,
  onOpenConversations,
  onOpenMenu,
  activeConversationMemberId,
  placeholder,
  channelId,
}: {
  members: ProjectMember[]
  onSelect: (member: ProjectMember, y: number) => void
  onOpenConversations?: (member: ProjectMember) => void
  onOpenMenu?: (request: MemberMenuRequest) => void
  activeConversationMemberId?: string | null
  placeholder: string
  channelId?: string
}) {
  const [query, setQuery] = useState('')
  const [scrollTop, setScrollTop] = useState(0)
  const [listHeight, setListHeight] = useState(0)
  const listRef = useRef<HTMLDivElement | null>(null)
  const [collapsedStatusSections, setCollapsedStatusSections] = useState(readCollapsedStatusSections)
  const filterScope = memberPanelFilterScope(channelId)
  const [filterPrefs, setFilterPrefs] = useState(() => readMemberPanelFilters(filterScope))
  const { statusFilter, roleFilter, sortMode } = filterPrefs
  const q = query.trim().toLowerCase()
  const roleOptions = useMemo(() => memberRoleFilterOptions(members), [members])
  const filtered = useMemo(
    () => {
      const visible = q ? filterVisibleMembers(members, q) : members
      return visible.filter((member) =>
        memberMatchesStatusFilter(member, statusFilter) && memberMatchesRoleFilter(member, roleFilter)
      )
    },
    [members, q, roleFilter, statusFilter],
  )
  const rows = useMemo(() => buildMemberRows(filtered, collapsedStatusSections, sortMode), [filtered, collapsedStatusSections, sortMode])
  const windowSize = Math.max(
    MEMBER_LIST_MIN_WINDOW,
    Math.ceil((listHeight || MEMBER_VIRTUAL_ROW_HEIGHT * MEMBER_LIST_WINDOW) / MEMBER_VIRTUAL_ROW_HEIGHT) + MEMBER_LIST_OVERSCAN * 2,
  )
  const start = Math.max(0, Math.floor(scrollTop / MEMBER_VIRTUAL_ROW_HEIGHT) - MEMBER_LIST_OVERSCAN)
  const end = Math.min(rows.length, start + windowSize)
  const visibleRows = rows.slice(start, end)
  const activeFilterCount = (q ? 1 : 0) + (statusFilter !== 'all' ? 1 : 0) + (roleFilter ? 1 : 0) + (sortMode !== 'role' ? 1 : 0)
  useEffect(() => {
    window.localStorage.setItem(MEMBER_PANEL_COLLAPSED_KEY, JSON.stringify(collapsedStatusSections))
  }, [collapsedStatusSections])
  useEffect(() => {
    setFilterPrefs(readMemberPanelFilters(filterScope))
    setScrollTop(0)
  }, [filterScope])
  useEffect(() => {
    if (filterPrefs.scope !== filterScope) return
    writeMemberPanelFilters(filterPrefs)
  }, [filterPrefs, filterScope])
  useEffect(() => {
    if (!roleFilter || roleOptions.some((option) => option.id === roleFilter)) return
    setFilterPrefs((current) => ({ ...current, scope: filterScope, roleFilter: '' }))
  }, [roleFilter, roleOptions])
  useEffect(() => {
    setScrollTop(0)
  }, [members, q, roleFilter, sortMode, statusFilter])
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
  function toggleStatusSection(rowId: string) {
    const id = rowId === 'status-online' ? 'online' : rowId === 'status-offline' ? 'offline' : null
    if (!id) return
    setCollapsedStatusSections((current) => ({ ...current, [id]: !current[id] }))
    setScrollTop(0)
  }
  return (
    <>
      <div className={styles.memberSearch}>
        <input
          className={styles.memberSearchInput}
          value={query}
          onChange={e => {
            setQuery(e.target.value)
            setScrollTop(0)
          }}
          placeholder={placeholder}
          autoComplete="off"
        />
        {query && (
          <button className={styles.memberSearchClear} type="button" onClick={() => {
            setQuery('')
            setScrollTop(0)
          }}>×</button>
        )}
        <div className={styles.memberFilterBar}>
          <div className={styles.memberStatusFilter} role="group" aria-label="成员状态筛选">
            {MEMBER_STATUS_FILTERS.map((filter) => (
              <button
                key={filter.id}
                type="button"
                data-active={statusFilter === filter.id ? 'true' : undefined}
                onClick={() => setFilterPrefs((current) => ({ ...current, scope: filterScope, statusFilter: filter.id }))}
              >
                {filter.label}
              </button>
            ))}
          </div>
          <select
            className={styles.memberFilterSelect}
            value={roleFilter}
            onChange={(event) => setFilterPrefs((current) => ({ ...current, scope: filterScope, roleFilter: event.target.value }))}
            aria-label="按角色筛选成员"
          >
            <option value="">全部角色</option>
            {roleOptions.map((option) => (
              <option key={option.id} value={option.id}>{option.label} ({option.count})</option>
            ))}
          </select>
          <select
            className={styles.memberFilterSelect}
            value={sortMode}
            onChange={(event) => {
              const next = event.target.value
              if (!isMemberPanelSortMode(next)) return
              setFilterPrefs((current) => ({ ...current, scope: filterScope, sortMode: next }))
            }}
            aria-label="成员排序"
          >
            <option value="role">按角色</option>
            <option value="name">按名称</option>
            <option value="joined">按加入</option>
          </select>
        </div>
        <div className={styles.memberFilterMeta}>
          显示 {filtered.length}/{members.length}{activeFilterCount ? ` · ${activeFilterCount} 个条件` : ''}
        </div>
      </div>
      <div
        ref={listRef}
        className={styles.memberVirtualList}
        onScroll={event => setScrollTop(event.currentTarget.scrollTop)}
      >
        {rows.length === 0 && <div className={styles.memberSection}>没有匹配成员</div>}
        {rows.length > 0 && (
          <div className={styles.memberVirtualCanvas} style={{ height: rows.length * MEMBER_VIRTUAL_ROW_HEIGHT }}>
            <div style={{ transform: `translateY(${start * MEMBER_VIRTUAL_ROW_HEIGHT}px)` }}>
              {visibleRows.map(row => {
                if (row.kind === 'status-header') {
                  return (
                    <div key={row.id} className={styles.memberVirtualStatusHeader}>
                      <button
                        className={styles.memberStatusSection}
                        type="button"
                        onClick={() => toggleStatusSection(row.id)}
                        aria-expanded={!row.collapsed}
                      >
                        <span>{row.collapsed ? '>' : 'v'} {row.label}</span>
                        <em>{row.count}</em>
                      </button>
                    </div>
                  )
                }
                if (row.kind === 'role-header') {
                  return (
                    <div key={row.id} className={styles.memberVirtualHeader}>
                      <div className={styles.memberSection}>{row.label} · {row.count}</div>
                    </div>
                  )
                }
                return (
                  <MemberListItem
                    key={row.id}
                    member={row.member}
                    onSelect={onSelect}
                    onOpenConversations={onOpenConversations}
                    onOpenMenu={onOpenMenu}
                    activeConversationMemberId={activeConversationMemberId}
                    channelId={channelId}
                  />
                )
              })}
            </div>
          </div>
        )}
      </div>
    </>
  )
}

/* ── MemberListItem ── */
function MemberListItem({
  member,
  onSelect,
  onOpenConversations,
  onOpenMenu,
  activeConversationMemberId,
  channelId,
}: {
  member: ProjectMember
  onSelect: (member: ProjectMember, y: number) => void
  onOpenConversations?: (member: ProjectMember) => void
  onOpenMenu?: (request: MemberMenuRequest) => void
  activeConversationMemberId?: string | null
  channelId?: string
}) {
  const roleKey = memberPrimaryRoleKey(member)
  const roleBadge = memberPrimaryRoleLabel(member)
  const status = memberPresenceStatus(member)
  const name = member.account ?? member.user_id
  const avatarCls = [
    styles.memberAvatar,
    memberAvatarRoleClass(roleKey),
    memberPresenceAvatarClass(status),
  ].filter(Boolean).join(' ')
  const active = activeConversationMemberId === member.user_id
  function openProfile(e: React.MouseEvent<HTMLElement>) {
    const rect = e.currentTarget.getBoundingClientRect()
    onSelect(member, rect.top + rect.height / 2)
  }
  function openMenu(e: React.MouseEvent<HTMLElement>) {
    e.preventDefault()
    e.stopPropagation()
    onOpenMenu?.({ member, x: e.clientX, y: e.clientY })
  }
  return (
    <div
      className={[styles.memberItem, active ? styles.memberItemActive : ''].join(' ')}
      onContextMenu={openMenu}
    >
      <button
        className={styles.memberAvatarButton}
        type="button"
        onClick={() => onOpenConversations?.(member)}
        title={`查看 ${name} 的项目会话`}
        aria-label={`查看 ${name} 的项目会话`}
      >
        <span className={avatarCls}>
          {member.avatar_data_url
            ? <img src={member.avatar_data_url} alt="" style={{ width: '100%', height: '100%', borderRadius: '50%', objectFit: 'cover', display: 'block' }} />
            : name[0].toUpperCase()
          }
        </span>
      </button>
      <button className={styles.memberInfoButton} type="button" onClick={openProfile}>
        <span className={styles.memberCopy}>
          <span className={styles.memberLine}>
            <strong className={styles.memberItemName}>{name}</strong>
            {roleBadge && <em className={[styles.memberRolePill, memberRolePillClass(roleKey)].join(' ')}>{roleBadge}</em>}
            <em className={[styles.memberPresencePill, memberPresencePillClass(status)].join(' ')}>{presenceLabel(status)}</em>
          </span>
          <span className={styles.memberSub}>{memberChannelSubtitle(member, channelId)}</span>
        </span>
      </button>
      {onOpenMenu && (
        <button
          className={styles.memberMoreButton}
          type="button"
          onClick={openMenu}
          title={`打开 ${name} 的成员菜单`}
          aria-label={`打开 ${name} 的成员菜单`}
        >
          ...
        </button>
      )}
    </div>
  )
}

/* ── MemberContextMenu ── */
export function MemberContextMenu({
  member,
  x,
  y,
  canModerate,
  canRemove,
  onClose,
  onOpenProfile,
  onOpenDetails,
  onOpenConversations,
  onOpenPermissions,
  onOpenRoles,
  onModerate,
  onRemove,
}: {
  member: ProjectMember
  x: number
  y: number
  canModerate?: boolean
  canRemove?: boolean
  onClose: () => void
  onOpenProfile: (member: ProjectMember, y: number) => void
  onOpenDetails?: (member: ProjectMember) => void
  onOpenConversations?: (member: ProjectMember) => void
  onOpenPermissions?: (member: ProjectMember) => void
  onOpenRoles?: (member: ProjectMember) => void
  onModerate?: (member: ProjectMember, action: MemberModerationAction, durationMinutes?: number) => Promise<void>
  onRemove?: (member: ProjectMember) => Promise<boolean | void>
}) {
  const menuRef = useRef<HTMLDivElement>(null)
  const [moderating, setModerating] = useState<MemberModerationAction | ''>('')
  const [removing, setRemoving] = useState(false)
  const [message, setMessage] = useState('')
  const name = member.account || member.user_id
  const status = memberPresenceStatus(member)
  const roleKey = memberPrimaryRoleKey(member)
  const avatarCls = [
    styles.memberMenuAvatar,
    memberAvatarRoleClass(roleKey),
    memberPresenceAvatarClass(status),
  ].filter(Boolean).join(' ')

  useEffect(() => {
    function onDown(e: MouseEvent) {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) onClose()
    }
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') onClose()
    }
    document.addEventListener('mousedown', onDown)
    document.addEventListener('keydown', onKey)
    return () => {
      document.removeEventListener('mousedown', onDown)
      document.removeEventListener('keydown', onKey)
    }
  }, [onClose])

  function run(action: () => void) {
    action()
    onClose()
  }

  function copyId() {
    navigator.clipboard.writeText(member.user_id).catch(() => {})
    onClose()
  }

  async function moderate(action: MemberModerationAction, durationMinutes?: number) {
    if (!onModerate || moderating) return
    setModerating(action)
    setMessage('提交中...')
    try {
      await onModerate(member, action, durationMinutes)
      setMessage('已更新')
      onClose()
    } catch (err) {
      setMessage((err as { message?: string }).message ?? '操作失败')
    } finally {
      setModerating('')
    }
  }

  async function removeMember() {
    if (!onRemove || removing) return
    setRemoving(true)
    setMessage('移除中...')
    try {
      const removed = await onRemove(member)
      if (removed !== false) onClose()
    } catch (err) {
      setMessage((err as { message?: string }).message ?? '移除失败')
    } finally {
      setRemoving(false)
    }
  }

  const MENU_WIDTH = 220
  const MENU_HEIGHT = canModerate || canRemove ? 438 : 236
  const left = Math.min(Math.max(x, 8), Math.max(8, window.innerWidth - MENU_WIDTH - 8))
  const top = Math.min(Math.max(y, 8), Math.max(8, window.innerHeight - MENU_HEIGHT - 8))

  return (
    <div
      ref={menuRef}
      className={styles.memberContextMenu}
      style={{ position: 'fixed', left, top, width: MENU_WIDTH, zIndex: 10000 }}
      role="menu"
    >
      <div className={styles.memberContextMenuHead}>
        <span className={avatarCls}>
          {member.avatar_data_url
            ? <img src={member.avatar_data_url} alt="" />
            : name[0]?.toUpperCase() ?? '?'
          }
        </span>
        <div>
          <strong>{name}</strong>
          <em>{presenceLabel(status)} · {memberRoleSummary(member)}</em>
        </div>
      </div>
      <div className={styles.memberContextMenuGroup}>
        <button type="button" role="menuitem" onClick={() => run(() => onOpenProfile(member, y))}>查看资料</button>
        {onOpenDetails && (
          <button type="button" role="menuitem" onClick={() => run(() => onOpenDetails(member))}>完整资料</button>
        )}
        {onOpenConversations && (
          <button type="button" role="menuitem" onClick={() => run(() => onOpenConversations(member))}>打开会话</button>
        )}
        <button type="button" role="menuitem" onClick={copyId}>复制用户 ID</button>
        {onOpenPermissions && (
          <button type="button" role="menuitem" onClick={() => run(() => onOpenPermissions(member))}>频道权限</button>
        )}
        {onOpenRoles && (
          <button type="button" role="menuitem" onClick={() => run(() => onOpenRoles(member))}>编辑角色</button>
        )}
      </div>
      {((canModerate && onModerate) || (canRemove && onRemove)) && (
        <div className={styles.memberContextMenuGroup}>
          <span className={styles.memberContextMenuLabel}>{message || (canModerate ? memberModerationSummary(member) : '成员管理')}</span>
          {canModerate && onModerate && (
            <>
              <button type="button" role="menuitem" onClick={() => moderate('mute', 60)} disabled={!!moderating || !!member.is_banned || removing}>禁言 1 小时</button>
              <button type="button" role="menuitem" onClick={() => moderate('mute', 1440)} disabled={!!moderating || !!member.is_banned || removing}>禁言 1 天</button>
              <button type="button" role="menuitem" onClick={() => moderate('unmute')} disabled={!!moderating || !member.is_muted || removing}>解禁言</button>
              <button type="button" role="menuitem" onClick={() => moderate('ban')} disabled={!!moderating || !!member.is_banned || removing} data-tone="danger">封禁</button>
              <button type="button" role="menuitem" onClick={() => moderate('unban')} disabled={!!moderating || !member.is_banned || removing}>解封</button>
            </>
          )}
          {canRemove && onRemove && (
            <button type="button" role="menuitem" onClick={removeMember} disabled={!!moderating || removing} data-tone="danger">
              {removing ? '移除中...' : '移除成员'}
            </button>
          )}
        </div>
      )}
    </div>
  )
}

/* ── MemberProfilePopover ── */
export function MemberProfilePopover({
  member,
  anchorY,
  projectId,
  channels,
  channel,
  canModerate,
  canRemove,
  onClose,
  onOpenDetails,
  onOpenConversations,
  onOpenRoles,
  onModerate,
  onRemove,
}: {
  member: ProjectMember
  anchorY: number
  projectId?: string
  channels?: Channel[]
  channel?: Channel
  canModerate?: boolean
  canRemove?: boolean
  onClose: () => void
  onOpenDetails?: (member: ProjectMember) => void
  onOpenConversations?: (member: ProjectMember) => void
  onOpenRoles?: (member: ProjectMember) => void
  onModerate?: (member: ProjectMember, action: MemberModerationAction, durationMinutes?: number) => Promise<void>
  onRemove?: (member: ProjectMember) => Promise<boolean | void>
}) {
  const popRef = useRef<HTMLDivElement>(null)
  const status = memberPresenceStatus(member)
  const name = member.account || member.user_id
  const roleKey = memberPrimaryRoleKey(member)
  const roleHeadCls = {
    owner: styles.memberPopoverHeadOwner, admin: styles.memberPopoverHeadAdmin,
    developer: styles.memberPopoverHeadEditor, dev: styles.memberPopoverHeadEditor,
    maintainer: styles.memberPopoverHeadEditor, editor: styles.memberPopoverHeadEditor,
    collaborator: styles.memberPopoverHeadEditor, builder: styles.memberPopoverHeadEditor,
  }[roleKey] ?? ''
  const [isFriend, setIsFriend] = useState(false)
  const [addingFriend, setAddingFriend] = useState(false)
  const [addMsg, setAddMsg] = useState('')
  const [moderating, setModerating] = useState<MemberModerationAction | ''>('')
  const [removing, setRemoving] = useState(false)
  const [moderationMsg, setModerationMsg] = useState('')
  const [auditEntries, setAuditEntries] = useState<ProjectMemberAuditEntry[]>([])
  const [auditLoading, setAuditLoading] = useState(false)
  const [auditMsg, setAuditMsg] = useState('')

  useEffect(() => {
    if (!member.user_id) return
    setIsFriend(false)
    setAddMsg('')
    api.get<{ already_friend?: boolean }>(`/api/me/friends/search?query=${encodeURIComponent(member.user_id)}&search_type=user_id`)
      .then(d => setIsFriend(!!d.already_friend))
      .catch(() => {})
  }, [member.user_id])

  useEffect(() => {
    setModerating('')
    setRemoving(false)
    setModerationMsg('')
  }, [member.user_id, member.is_muted, member.is_banned])

  useEffect(() => {
    if (!projectId || !canModerate || !member.user_id) {
      setAuditEntries([])
      setAuditMsg('')
      setAuditLoading(false)
      return
    }
    let alive = true
    const targetId = clean(member.user_id)
    const targetAccount = clean(member.account ?? '').toLowerCase()
    setAuditLoading(true)
    setAuditMsg('')
    api.get<ProjectMemberAuditResponse>(`/api/projects/${encodeURIComponent(projectId)}/member-audit?limit=80`)
      .then((data) => {
        if (!alive) return
        const entries = (data.entries ?? [])
          .filter((entry) => {
            const entryTargetId = clean(entry.target_user_id ?? '')
            const entryTargetAccount = clean(entry.target_account ?? '').toLowerCase()
            return (!!targetId && entryTargetId === targetId)
              || (!!targetAccount && entryTargetAccount === targetAccount)
          })
          .slice(0, 5)
        setAuditEntries(entries)
        setAuditMsg('')
      })
      .catch((err) => {
        if (!alive) return
        setAuditEntries([])
        setAuditMsg((err as { message?: string }).message ?? '暂无权限查看成员记录')
      })
      .finally(() => {
        if (alive) setAuditLoading(false)
      })
    return () => {
      alive = false
    }
  }, [projectId, canModerate, member.user_id, member.account])

  useEffect(() => {
    function onDown(e: MouseEvent) {
      if (popRef.current && !popRef.current.contains(e.target as Node)) onClose()
    }
    document.addEventListener('mousedown', onDown)
    return () => document.removeEventListener('mousedown', onDown)
  }, [onClose])

  function copyId() {
    navigator.clipboard.writeText(member.user_id).catch(() => {})
  }

  function openConversations() {
    onOpenConversations?.(member)
  }

  function openRoles() {
    onOpenRoles?.(member)
  }

  function openDetails() {
    onOpenDetails?.(member)
    onClose()
  }

  async function addFriend() {
    if (isFriend || addingFriend) return
    setAddingFriend(true)
    try {
      await api.post('/api/me/friends', { query: member.user_id, search_type: 'user_id' })
      setIsFriend(true)
      setAddMsg('已添加')
    } catch (err) {
      setAddMsg((err as { message?: string }).message ?? '添加失败')
    } finally {
      setAddingFriend(false)
    }
  }

  async function moderate(action: MemberModerationAction, durationMinutes?: number) {
    if (!onModerate || moderating) return
    setModerating(action)
    setModerationMsg('提交中...')
    try {
      await onModerate(member, action, durationMinutes)
      setModerationMsg('已更新')
    } catch (err) {
      setModerationMsg((err as { message?: string }).message ?? '操作失败')
    } finally {
      setModerating('')
    }
  }

  async function removeMember() {
    if (!onRemove || removing) return
    setRemoving(true)
    setModerationMsg('移除中...')
    try {
      const removed = await onRemove(member)
      if (removed !== false) onClose()
    } catch (err) {
      setModerationMsg((err as { message?: string }).message ?? '移除失败')
    } finally {
      setRemoving(false)
    }
  }

  const details: [string, string][] = [
    member.account && ['账号', member.account],
    member.user_id && ['用户 ID', member.user_id.slice(0, 14)],
    member.joined_at && ['加入时间', formatTime(member.joined_at)],
  ].filter(Boolean) as [string, string][]
  const channelPermissions = memberChannelPermissions(member, channel?.id)
  const channelCapabilityLabels = memberChannelCapabilityLabels(channelPermissions)
  const roleChips = profileRoleChips(member)
  const profileVisibleChannels = (channels ?? []).filter((item) => {
    const permissions = memberChannelPermissions(member, item.id)
    return !permissions || memberChannelCanView(permissions)
  })
  const previewChannels = profileVisibleChannels.slice(0, 6)
  const hiddenChannelCount = Math.max(0, (channels ?? []).length - profileVisibleChannels.length)
  const extraChannelCount = Math.max(0, profileVisibleChannels.length - previewChannels.length)
  const presenceDetails = [
    clean(member.custom_status ?? ''),
    clean(member.activity ?? ''),
  ].filter(Boolean)

  const POPOVER_WIDTH = 328
  const POPOVER_HEIGHT = 580
  const viewW = window.innerWidth
  const viewH = window.innerHeight
  const maxTop = Math.max(12, viewH - POPOVER_HEIGHT - 12)
  const popTop = Math.min(Math.max(anchorY - 20, 12), maxTop)
  const popLeft = Math.max(8, viewW - 280 - POPOVER_WIDTH - 8)

  return (
    <div ref={popRef} className={styles.memberPopover}
      style={{ position: 'fixed', left: popLeft, top: popTop, zIndex: 9999, width: POPOVER_WIDTH }}>
      <div className={[styles.memberPopoverHead, roleHeadCls].join(' ')}>
        <div className={[
          styles.memberPopoverAvatar,
          memberPresenceAvatarClass(status),
        ].join(' ')}>
          {member.avatar_data_url
            ? <img src={member.avatar_data_url} alt="" />
            : <span>{name[0]?.toUpperCase() ?? '?'}</span>
          }
        </div>
        <button className={styles.memberPopoverClose} onClick={onClose} type="button">×</button>
      </div>
      <div className={styles.memberPopoverBody}>
        <strong className={styles.memberPopoverName}>{name}</strong>
        <span className={styles.memberPopoverSub}>{presenceLabel(status)}</span>
        <div className={styles.memberPopoverMeta}>
          <em className={styles.memberPopoverPill}>{memberRoleSummary(member)}</em>
          <em className={[styles.memberPopoverPill, status === 'online' ? styles.memberPopoverPillOnline : ''].join(' ')}>
            {presenceLabel(status)}
          </em>
        </div>
        {presenceDetails.length > 0 && (
          <div className={styles.memberPopoverPresence}>
            {presenceDetails.map((item) => (
              <span key={item}>{item}</span>
            ))}
          </div>
        )}
        {channel && channelPermissions && (
          <div className={styles.memberPopoverChannel}>
            <div>
              <span>当前频道</span>
              <strong>{channel.name}</strong>
            </div>
            <div className={styles.memberPopoverChannelPills}>
              {channelCapabilityLabels.map((label) => (
                <em
                  key={label}
                  className={[
                    styles.memberPopoverPill,
                    label === '无频道访问权限' ? styles.memberPopoverPillDanger : '',
                  ].join(' ')}
                >
                  {label}
                </em>
              ))}
            </div>
          </div>
        )}
        {details.length > 0 && (
          <div className={styles.memberPopoverDetails}>
            {details.map(([label, value]) => (
              <div key={label} className={styles.memberPopoverDetail}>
                <span>{label}</span>
                <strong title={value}>{value}</strong>
              </div>
            ))}
          </div>
        )}
        <section className={styles.memberPopoverSection}>
          <div className={styles.memberPopoverSectionHead}>
            <strong>角色</strong>
            <span>{roleChips.length} 个</span>
          </div>
          <div className={styles.memberPopoverRoleList}>
            {roleChips.map((role) => (
              <em
                key={role.id}
                className={styles.memberPopoverRoleChip}
                style={role.color ? { borderColor: role.color, color: role.color } : undefined}
              >
                {role.name || roleLabel(role.id)}
              </em>
            ))}
          </div>
        </section>
        {channels && channels.length > 0 && (
          <section className={styles.memberPopoverSection}>
            <div className={styles.memberPopoverSectionHead}>
              <strong>可见频道</strong>
              <span>
                {profileVisibleChannels.length}/{channels.length}
                {hiddenChannelCount > 0 ? ` · 隐藏 ${hiddenChannelCount}` : ''}
              </span>
            </div>
            <div className={styles.memberPopoverChannelList}>
              {previewChannels.length === 0 && (
                <p className={styles.memberPopoverEmpty}>当前没有可见频道</p>
              )}
              {previewChannels.map((item) => (
                <div key={item.id} className={styles.memberPopoverChannelItem}>
                  <span>{channelKindMark(item.kind)}</span>
                  <div>
                    <strong title={item.name}>{item.name}</strong>
                    <em>{item.category_name || channelKindLabel(item.kind)}</em>
                  </div>
                </div>
              ))}
              {extraChannelCount > 0 && (
                <p className={styles.memberPopoverMore}>还有 {extraChannelCount} 个可见频道</p>
              )}
            </div>
          </section>
        )}
        <div className={styles.memberPopoverActions}>
          {onOpenDetails && (
            <button className={[styles.memberPopoverBtn, styles.memberPopoverBtnPrimary].join(' ')} type="button" onClick={openDetails}>
              完整资料
            </button>
          )}
          {onOpenConversations && (
            <button className={[styles.memberPopoverBtn, styles.memberPopoverBtnPrimary].join(' ')} type="button" onClick={openConversations}>
              打开会话
            </button>
          )}
          <button className={styles.memberPopoverBtn} type="button" onClick={copyId}>复制 ID</button>
          {onOpenRoles && (
            <button className={styles.memberPopoverBtn} type="button" onClick={openRoles}>编辑角色</button>
          )}
          <button className={styles.memberPopoverBtn} type="button"
            onClick={addFriend} disabled={isFriend || addingFriend}
            data-state={isFriend ? 'success' : undefined}>
            {addMsg || (isFriend ? '已是好友' : addingFriend ? '添加中…' : '加好友')}
          </button>
        </div>
        {((canModerate && onModerate) || (canRemove && onRemove)) && (
          <div className={styles.memberPopoverModeration}>
            <div className={styles.memberPopoverModerationHead}>
              <strong>管理操作</strong>
              <span>{moderationMsg || (canModerate ? memberModerationSummary(member) : '成员管理')}</span>
            </div>
            <div className={styles.memberPopoverModerationGrid}>
              {canModerate && onModerate && (
                <>
                  <button className={styles.memberPopoverBtn} type="button" onClick={() => moderate('mute', 60)} disabled={!!moderating || !!member.is_banned || removing}>
                    禁言1小时
                  </button>
                  <button className={styles.memberPopoverBtn} type="button" onClick={() => moderate('mute', 1440)} disabled={!!moderating || !!member.is_banned || removing}>
                    禁言1天
                  </button>
                  <button className={styles.memberPopoverBtn} type="button" onClick={() => moderate('unmute')} disabled={!!moderating || !member.is_muted || removing}>
                    解禁言
                  </button>
                  <button className={[styles.memberPopoverBtn, styles.memberPopoverBtnDanger].join(' ')} type="button" onClick={() => moderate('ban')} disabled={!!moderating || !!member.is_banned || removing}>
                    封禁
                  </button>
                  <button className={styles.memberPopoverBtn} type="button" onClick={() => moderate('unban')} disabled={!!moderating || !member.is_banned || removing}>
                    解封
                  </button>
                </>
              )}
              {canRemove && onRemove && (
                <button className={[styles.memberPopoverBtn, styles.memberPopoverBtnDanger].join(' ')} type="button" onClick={removeMember} disabled={!!moderating || removing}>
                  {removing ? '移除中...' : '移除成员'}
                </button>
              )}
            </div>
          </div>
        )}
        {projectId && canModerate && (
          <section className={styles.memberPopoverSection}>
            <div className={styles.memberPopoverSectionHead}>
              <strong>近期记录</strong>
              <span>{auditLoading ? '同步中' : `${auditEntries.length} 条`}</span>
            </div>
            <div className={styles.memberPopoverTimeline}>
              {auditLoading && auditEntries.length === 0 && (
                <p className={styles.memberPopoverEmpty}>正在读取成员记录...</p>
              )}
              {!auditLoading && auditEntries.length === 0 && (
                <p className={styles.memberPopoverEmpty}>{auditMsg || '暂无近期成员记录'}</p>
              )}
              {auditEntries.map((entry) => (
                <article key={entry.id} className={styles.memberPopoverTimelineItem}>
                  <div>
                    <strong>{profileAuditActionLabel(entry.action)}</strong>
                    <time>{formatTime(entry.created_at)}</time>
                  </div>
                  <span>{profileAuditSummary(entry)}</span>
                </article>
              ))}
            </div>
          </section>
        )}
      </div>
    </div>
  )
}

function profileRoleChips(member: ProjectMember): ProjectRoleRef[] {
  if (member.roles?.length) return member.roles
  const fallbackId = clean(member.role ?? 'member') || 'member'
  return [{ id: fallbackId, name: roleLabel(fallbackId), builtin: true }]
}

function channelKindMark(kind?: string) {
  const normalized = clean(kind ?? '').toLowerCase()
  if (normalized === 'ai_development') return '⚒'
  if (normalized === 'builds') return '◆'
  if (normalized === 'announce' || normalized === 'announcements') return '!'
  if (normalized === 'docs') return '文'
  return '#'
}

function channelKindLabel(kind?: string) {
  const normalized = clean(kind ?? '').toLowerCase()
  const labels: Record<string, string> = {
    ai_development: 'AI 开发频道',
    builds: '构建发布频道',
    announce: '公告频道',
    announcements: '公告频道',
    docs: '文档频道',
    chat: '聊天频道',
  }
  return labels[normalized] ?? '项目频道'
}

const PROFILE_AUDIT_ACTION_LABELS: Record<string, string> = {
  add_member: '添加成员',
  invite_member: '邀请成员',
  join_by_invite_link: '通过邀请加入',
  update_member_role: '调整角色',
  remove_member: '移除成员',
  mute_member: '禁言成员',
  unmute_member: '解除禁言',
  ban_member: '封禁成员',
  unban_member: '解封成员',
}

function profileAuditActionLabel(action: string) {
  return PROFILE_AUDIT_ACTION_LABELS[clean(action)] ?? (clean(action) || '成员操作')
}

function profileAuditSummary(entry: ProjectMemberAuditEntry) {
  const actor = clean(entry.actor_account ?? entry.actor_user_id ?? '') || '系统'
  const oldRole = clean(entry.old_role ?? '')
  const newRole = clean(entry.new_role ?? '')
  const parts = [`操作者 ${actor}`]
  if (oldRole || newRole) {
    if (oldRole && newRole) parts.push(`${roleLabel(oldRole)} -> ${roleLabel(newRole)}`)
    else parts.push(roleLabel(newRole || oldRole))
  }
  const note = clean(entry.note ?? '')
  if (note) parts.push(profileAuditNote(note))
  return parts.join(' · ')
}

function profileAuditNote(note: string) {
  const first = note.split(';').map((part) => part.trim()).filter(Boolean)[0] ?? note
  const index = first.indexOf('=')
  if (index < 0) return first
  const labels: Record<string, string> = {
    reason: '原因',
    duration: '时长',
    duration_minutes: '时长',
    invite_code: '邀请码',
    channel_id: '频道',
  }
  const key = first.slice(0, index)
  const value = first.slice(index + 1)
  return `${labels[key] ?? key}: ${value || '-'}`
}

/* ── MemberContextSummary ── */
export function MemberContextSummary({
  title,
  label,
  members,
  channel,
  projectTotal,
  usingChannelPermissions,
}: {
  title?: string
  label: string
  members: ProjectMember[]
  channel?: Channel
  projectTotal?: number
  usingChannelPermissions?: boolean
}) {
  const stats = memberPanelStats(members)
  const scopeLabel = channel ? '频道范围' : '项目范围'
  const modeLabel = channel
    ? usingChannelPermissions ? '按频道权限' : '继承项目成员'
    : '全项目成员'
  return (
    <section className={styles.memberContextSummary}>
      <div className={styles.memberContextTop}>
        <strong>{title || (channel ? '频道视图' : '项目视图')}</strong>
        <em data-scope={channel ? 'channel' : 'project'}>{scopeLabel}</em>
      </div>
      <div className={styles.memberContextMode}>
        <em>{modeLabel}</em>
        {typeof projectTotal === 'number' && channel && <em>项目 {projectTotal}</em>}
      </div>
      <span>{label}</span>
      {channel && usingChannelPermissions && typeof projectTotal === 'number' && (
        <span>按频道权限过滤，项目成员 {projectTotal} 人</span>
      )}
      {members.length > 0 && (
        <div className={styles.memberContextStats}>
          <em>在线 {stats.online}</em>
          <em>离线 {stats.offline}</em>
          {stats.restricted > 0 && <em>受限 {stats.restricted}</em>}
        </div>
      )}
    </section>
  )
}

function memberPanelStats(members: ProjectMember[]) {
  return members.reduce((stats, member) => {
    if (memberPresenceStatus(member) === 'offline') stats.offline += 1
    else stats.online += 1
    if (member.is_banned || member.is_muted) stats.restricted += 1
    return stats
  }, { online: 0, offline: 0, restricted: 0 })
}

/* ── MemberLoadingRows ── */
export function MemberLoadingRows() {
  return (
    <div className={styles.memberLoadingRows}>
      <span />
      <span />
      <span />
    </div>
  )
}
