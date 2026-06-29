import { useState, useEffect, useMemo, useRef } from 'react'
import { api } from '../../api/client'
import { clean, formatTime } from '../../lib/utils'
import type { Channel, ProjectMember } from './types'
import {
  memberPresenceStatus,
  memberChannelSubtitle,
  memberChannelPermissions,
  memberChannelCapabilityLabels,
  memberRoleSummary,
  presenceLabel,
  memberPrimaryRoleKey,
} from './memberUtils'
import styles from './ConversationPage.module.css'

/* ── 虚拟列表类型和常量 ── */
export type MemberVirtualRow =
  | { kind: 'header'; id: string; label: string; count: number }
  | { kind: 'member'; id: string; member: ProjectMember }

export const MEMBER_VIRTUAL_ROW_HEIGHT = 48
const MEMBER_LIST_OVERSCAN = 6
const MEMBER_LIST_WINDOW = 28

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

function buildMemberRows(members: ProjectMember[]): MemberVirtualRow[] {
  const groups = ROLE_GROUPS.map((group) => [
    group.label,
    sortMembersForPanel(members.filter((member) => memberRoleGroup(member) === group.id)),
  ] as [string, ProjectMember[]])
  return groups.flatMap(([label, list]) => {
    if (!list.length) return []
    return [
      { kind: 'header' as const, id: `header-${label}`, label, count: list.length },
      ...list.map(member => ({ kind: 'member' as const, id: member.user_id, member })),
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

/* ── MemberSearch ── */
export function MemberSearch({
  members,
  onSelect,
  placeholder,
  channelId,
}: {
  members: ProjectMember[]
  onSelect: (member: ProjectMember, y: number) => void
  placeholder: string
  channelId?: string
}) {
  const [query, setQuery] = useState('')
  const [scrollTop, setScrollTop] = useState(0)
  const q = query.trim().toLowerCase()
  const filtered = useMemo(
    () => q ? filterVisibleMembers(members, q) : members,
    [members, q],
  )
  const rows = useMemo(() => buildMemberRows(filtered), [filtered])
  const start = Math.max(0, Math.floor(scrollTop / MEMBER_VIRTUAL_ROW_HEIGHT) - MEMBER_LIST_OVERSCAN)
  const end = Math.min(rows.length, start + MEMBER_LIST_WINDOW)
  const visibleRows = rows.slice(start, end)
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
          <button className={styles.memberSearchClear} type="button" onClick={() => setQuery('')}>×</button>
        )}
      </div>
      <div className={styles.memberVirtualList} onScroll={event => setScrollTop(event.currentTarget.scrollTop)}>
        {rows.length === 0 && <div className={styles.memberSection}>没有匹配成员</div>}
        {rows.length > 0 && (
          <div className={styles.memberVirtualCanvas} style={{ height: rows.length * MEMBER_VIRTUAL_ROW_HEIGHT }}>
            <div style={{ transform: `translateY(${start * MEMBER_VIRTUAL_ROW_HEIGHT}px)` }}>
              {visibleRows.map(row => row.kind === 'header'
                ? <div key={row.id} className={styles.memberVirtualHeader}><div className={styles.memberSection}>{row.label} · {row.count}</div></div>
                : <MemberListItem key={row.id} member={row.member} onSelect={onSelect} channelId={channelId} />
              )}
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
  channelId,
}: {
  member: ProjectMember
  onSelect: (member: ProjectMember, y: number) => void
  channelId?: string
}) {
  const roleKey = memberPrimaryRoleKey(member)
  const roleBadge = memberPrimaryRoleLabel(member)
  const name = member.account ?? member.user_id
  const avatarCls = [
    styles.memberAvatar,
    memberAvatarRoleClass(roleKey),
    member.is_online ? styles.memberAvatarOnline : styles.memberAvatarOffline,
  ].filter(Boolean).join(' ')
  return (
    <button className={styles.memberItem} type="button" onClick={(e) => {
      const rect = e.currentTarget.getBoundingClientRect()
      onSelect(member, rect.top + rect.height / 2)
    }}>
      <div className={avatarCls}>
        {member.avatar_data_url
          ? <img src={member.avatar_data_url} alt="" style={{ width: '100%', height: '100%', borderRadius: '50%', objectFit: 'cover', display: 'block' }} />
          : name[0].toUpperCase()
        }
      </div>
      <div className={styles.memberCopy}>
        <div className={styles.memberLine}>
          <strong className={styles.memberItemName}>{name}</strong>
          {roleBadge && <em className={[styles.memberRolePill, memberRolePillClass(roleKey)].join(' ')}>{roleBadge}</em>}
        </div>
        <span className={styles.memberSub}>{memberChannelSubtitle(member, channelId)}</span>
      </div>
    </button>
  )
}

/* ── MemberProfilePopover ── */
export function MemberProfilePopover({ member, anchorY, channel, onClose }: {
  member: ProjectMember
  anchorY: number
  channel?: Channel
  onClose: () => void
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

  useEffect(() => {
    if (!member.user_id) return
    api.get<{ already_friend?: boolean }>(`/api/me/friends/search?query=${encodeURIComponent(member.user_id)}&search_type=user_id`)
      .then(d => setIsFriend(!!d.already_friend))
      .catch(() => {})
  }, [member.user_id])

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

  const details: [string, string][] = [
    member.account && ['账号', member.account],
    member.user_id && ['用户 ID', member.user_id.slice(0, 14)],
    member.joined_at && ['加入时间', formatTime(member.joined_at)],
  ].filter(Boolean) as [string, string][]
  const channelPermissions = memberChannelPermissions(member, channel?.id)
  const channelCapabilityLabels = memberChannelCapabilityLabels(channelPermissions)

  const POPOVER_WIDTH = 300
  const POPOVER_HEIGHT = 360
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
          status === 'online' ? styles.memberAvatarOnline : styles.memberAvatarOffline,
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
        <div className={styles.memberPopoverActions}>
          <button className={styles.memberPopoverBtn} type="button" onClick={copyId}>复制 ID</button>
          <button className={styles.memberPopoverBtn} type="button"
            onClick={addFriend} disabled={isFriend || addingFriend}
            style={{ background: isFriend ? 'rgba(88,190,106,.1)' : undefined, color: isFriend ? 'var(--green,#58BE6A)' : undefined, cursor: isFriend ? 'default' : 'pointer' }}>
            {addMsg || (isFriend ? '已是好友' : addingFriend ? '添加中…' : '加好友')}
          </button>
        </div>
      </div>
    </div>
  )
}

/* ── MemberContextSummary ── */
export function MemberContextSummary({
  label,
  members,
  channel,
  projectTotal,
  usingChannelPermissions,
}: {
  label: string
  members: ProjectMember[]
  channel?: Channel
  projectTotal?: number
  usingChannelPermissions?: boolean
}) {
  const stats = memberPanelStats(members)
  return (
    <section className={styles.memberContextSummary}>
      <strong>{channel ? '频道视图' : '项目视图'}</strong>
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
