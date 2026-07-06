import { useEffect, useMemo, useState } from 'react'
import { Check, Search, UsersRound, X } from 'lucide-react'
import { api } from '../../api/client'
import { useProjectStore } from '../conversation/useProjectStore'
import type { ProjectMember } from '../conversation/types'
import styles from './UserPickerDrawer.module.css'

export interface UserPickerUser {
  id: string
  account: string
  nickname?: string | null
  avatar_data_url?: string | null
  is_online?: boolean
  already_friend?: boolean
  mutual_friend_count?: number
  source?: UserPickerSource
  role_label?: string
}

export type UserPickerSource = 'project' | 'friends' | 'all'

interface FriendLikeUser {
  id: string
  account?: string
  nickname?: string | null
  avatar_data_url?: string | null
  is_online?: boolean
  already_friend?: boolean
  mutual_friend_count?: number
}

interface FriendSearchResponse {
  found?: boolean
  user?: FriendLikeUser
  already_friend?: boolean
  is_self?: boolean
}

const SOURCE_TABS: Array<{ id: UserPickerSource; label: string }> = [
  { id: 'project', label: '项目成员' },
  { id: 'friends', label: '好友' },
  { id: 'all', label: '全站用户' },
]

export default function UserPickerDrawer({
  title,
  subtitle,
  open,
  busy,
  currentUserId,
  disabledUserIds,
  onClose,
  onConfirm,
}: {
  title: string
  subtitle?: string
  open: boolean
  busy?: boolean
  currentUserId?: string
  disabledUserIds?: Set<string>
  onClose: () => void
  onConfirm: (users: UserPickerUser[]) => Promise<void> | void
}) {
  const activeProjectId = useProjectStore((state) => state.activeProjectId)
  const activeProject = useProjectStore((state) => state.projects.find((project) => project.id === state.activeProjectId))
  const projectMembers = useProjectStore((state) => state.members)
  const loadProjects = useProjectStore((state) => state.loadProjects)
  const [source, setSource] = useState<UserPickerSource>('all')
  const [query, setQuery] = useState('')
  const [friends, setFriends] = useState<UserPickerUser[]>([])
  const [globalUsers, setGlobalUsers] = useState<UserPickerUser[]>([])
  const [remoteSearchUser, setRemoteSearchUser] = useState<UserPickerUser | null>(null)
  const [selectedIds, setSelectedIds] = useState<Set<string>>(() => new Set())
  const [loading, setLoading] = useState(false)
  const [searching, setSearching] = useState(false)
  const [error, setError] = useState('')

  useEffect(() => {
    if (!open) return
    setSource('all')
    setQuery('')
    setSelectedIds(new Set())
    setRemoteSearchUser(null)
    setError('')
    void loadProjects().catch(() => {})
    void loadUsers()
  }, [open])

  useEffect(() => {
    if (!open || source !== 'all') {
      setRemoteSearchUser(null)
      return
    }
    const needle = query.trim()
    if (needle.length < 2 && !needle.includes('@')) {
      setRemoteSearchUser(null)
      return
    }
    const timer = window.setTimeout(() => {
      void searchGlobalUser(needle)
    }, 220)
    return () => window.clearTimeout(timer)
  }, [open, source, query])

  const projectUsers = useMemo(() => {
    return projectMembers.map(projectMemberToPickerUser)
  }, [projectMembers])

  const sourceUsers = useMemo(() => {
    if (source === 'project') return projectUsers
    if (source === 'friends') return friends
    return mergeUsers(remoteSearchUser ? [remoteSearchUser, ...globalUsers] : globalUsers)
  }, [friends, globalUsers, projectUsers, remoteSearchUser, source])

  const visibleUsers = useMemo(() => filterUsers(sourceUsers, query), [sourceUsers, query])
  const selectedUsers = useMemo(() => {
    const all = mergeUsers([...projectUsers, ...friends, ...globalUsers, ...(remoteSearchUser ? [remoteSearchUser] : [])])
    return all.filter((user) => selectedIds.has(user.id))
  }, [friends, globalUsers, projectUsers, remoteSearchUser, selectedIds])
  const selectedLine = selectedUsers.length
    ? selectedUsers.map((user) => displayName(user)).join('、')
    : '尚未选择用户'
  const meta = sourceMeta(source, visibleUsers.length, sourceUsers.length, activeProject?.name, searching)

  async function loadUsers() {
    setLoading(true)
    setError('')
    const [friendResult, globalResult] = await Promise.allSettled([
      api.get<{ friends?: FriendLikeUser[] }>('/api/me/friends'),
      api.get<{ recommendations?: FriendLikeUser[]; total_count?: number }>('/api/me/friends/recommendations?limit=200'),
    ])
    if (friendResult.status === 'fulfilled') {
      setFriends((friendResult.value.friends ?? []).map((user) => normalizeUser(user, 'friends')))
    }
    if (globalResult.status === 'fulfilled') {
      setGlobalUsers((globalResult.value.recommendations ?? []).map((user) => normalizeUser(user, 'all')))
    }
    if (friendResult.status === 'rejected' && globalResult.status === 'rejected') {
      setError('用户列表读取失败，请稍后重试。')
    }
    setLoading(false)
  }

  async function searchGlobalUser(needle: string) {
    setSearching(true)
    try {
      const params = new URLSearchParams({ query: needle, search_type: 'auto' })
      const data = await api.get<FriendSearchResponse>(`/api/me/friends/search?${params.toString()}`)
      if (data.found && data.user && !data.is_self) {
        setRemoteSearchUser(normalizeUser({ ...data.user, already_friend: data.already_friend }, 'all'))
      } else {
        setRemoteSearchUser(null)
      }
    } catch {
      setRemoteSearchUser(null)
    } finally {
      setSearching(false)
    }
  }

  function toggleUser(user: UserPickerUser) {
    if (isDisabled(user)) return
    setSelectedIds((current) => {
      const next = new Set(current)
      if (next.has(user.id)) next.delete(user.id)
      else next.add(user.id)
      return next
    })
  }

  function isDisabled(user: UserPickerUser) {
    return user.id === currentUserId || !!disabledUserIds?.has(user.id)
  }

  async function confirmSelection() {
    if (selectedUsers.length === 0 || busy) return
    await onConfirm(selectedUsers)
  }

  if (!open) return null

  return (
    <div className={styles.backdrop}>
      <section className={styles.drawer} role="dialog" aria-modal="true" aria-label={title}>
        <header className={styles.header}>
          <div>
            <strong>{title}</strong>
            <span>{subtitle ?? '从项目成员、好友或全站用户中勾选授权对象。'}</span>
          </div>
          <button className={styles.iconButton} type="button" onClick={onClose} title="关闭">
            <X size={17} aria-hidden="true" />
          </button>
        </header>

        <div className={styles.tabs} role="tablist" aria-label="用户来源">
          {SOURCE_TABS.map((item) => (
            <button
              key={item.id}
              className={styles.tabButton}
              data-active={source === item.id ? 'true' : undefined}
              type="button"
              role="tab"
              aria-selected={source === item.id}
              onClick={() => setSource(item.id)}
            >
              <UsersRound size={14} aria-hidden="true" />
              <span>{item.label}</span>
            </button>
          ))}
        </div>

        <label className={styles.searchBar}>
          <Search size={15} aria-hidden="true" />
          <input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder={source === 'all' ? '搜索昵称、邮箱、手机号或 user id' : '搜索昵称、账号或 user id'}
            autoFocus
          />
        </label>
        <div className={styles.metaLine}>{loading ? '读取中...' : error || meta}</div>

        <div className={styles.list}>
          {!loading && visibleUsers.length === 0 && (
            <p className={styles.empty}>
              {source === 'project' && !activeProjectId
                ? '当前没有打开项目成员列表，可切换到好友或全站用户。'
                : query.trim()
                  ? '没有匹配用户。全站模式支持精确搜索昵称、邮箱、手机号或 user id。'
                  : '暂无可选用户。'}
            </p>
          )}
          {visibleUsers.map((user) => {
            const disabled = isDisabled(user)
            const selected = selectedIds.has(user.id)
            return (
              <label
                key={`${source}:${user.id}`}
                className={styles.row}
                data-selected={selected ? 'true' : undefined}
                data-disabled={disabled || busy ? 'true' : undefined}
                aria-disabled={disabled || busy ? 'true' : undefined}
                title={disabledReason(user, currentUserId, disabledUserIds)}
              >
                <input
                  className={styles.checkbox}
                  type="checkbox"
                  checked={selected}
                  disabled={disabled || busy}
                  onChange={() => toggleUser(user)}
                  aria-label={`选择 ${displayName(user)}`}
                />
                <span className={styles.avatar}>{user.avatar_data_url ? <img src={user.avatar_data_url} alt="" /> : initial(user)}</span>
                <span className={styles.main}>
                  <strong>{displayName(user)}</strong>
                  <span>{user.account || user.id}</span>
                </span>
                <span className={styles.badges}>
                  {user.is_online && <em data-tone="active">在线</em>}
                  {user.role_label && <em>{user.role_label}</em>}
                  {user.already_friend && <em>好友</em>}
                  {disabled && <em data-tone="warn">{user.id === currentUserId ? '自己' : '已授权'}</em>}
                </span>
              </label>
            )
          })}
        </div>

        <footer className={styles.footer}>
          <span className={styles.selectedLine} title={selectedLine}>{selectedLine}</span>
          <button className={styles.button} type="button" onClick={onClose} disabled={busy}>取消</button>
          <button
            className={[styles.button, styles.primary].join(' ')}
            type="button"
            disabled={busy || selectedUsers.length === 0}
            onClick={() => { void confirmSelection() }}
          >
            <Check size={14} aria-hidden="true" />
            <span>{busy ? '处理中' : `授权 ${selectedUsers.length || ''}`}</span>
          </button>
        </footer>
      </section>
    </div>
  )
}

function normalizeUser(user: FriendLikeUser, source: UserPickerSource): UserPickerUser {
  return {
    id: user.id,
    account: user.account || user.id,
    nickname: user.nickname,
    avatar_data_url: user.avatar_data_url,
    is_online: user.is_online,
    already_friend: user.already_friend,
    mutual_friend_count: user.mutual_friend_count,
    source,
  }
}

function projectMemberToPickerUser(member: ProjectMember): UserPickerUser {
  const role = member.roles?.[0]?.name || member.roles?.[0]?.id || member.role
  return {
    id: member.user_id,
    account: member.account || member.global_account || member.user_id,
    nickname: member.member_display_name,
    avatar_data_url: member.avatar_data_url,
    is_online: member.is_online,
    role_label: role,
    source: 'project',
  }
}

function filterUsers(users: UserPickerUser[], query: string) {
  const needle = query.trim().toLowerCase()
  if (!needle) return users
  return users.filter((user) => {
    const haystack = [
      user.id,
      user.account,
      user.nickname,
      user.role_label,
    ].filter(Boolean).join(' ').toLowerCase()
    return haystack.includes(needle)
  })
}

function mergeUsers(users: UserPickerUser[]) {
  const seen = new Set<string>()
  const out: UserPickerUser[] = []
  users.forEach((user) => {
    if (!user.id || seen.has(user.id)) return
    seen.add(user.id)
    out.push(user)
  })
  return out
}

function sourceMeta(source: UserPickerSource, visible: number, total: number, projectName?: string, searching?: boolean) {
  const prefix = source === 'project'
    ? projectName ? `项目 ${projectName}` : '项目成员'
    : source === 'friends'
      ? '好友列表'
      : '全站用户'
  return `${prefix} · 显示 ${visible}/${total}${searching ? ' · 搜索中' : ''}`
}

function displayName(user: UserPickerUser) {
  return user.nickname || user.account || user.id
}

function initial(user: UserPickerUser) {
  const chars = Array.from(displayName(user).trim())
  return (chars[0] || '用').toUpperCase()
}

function disabledReason(user: UserPickerUser, currentUserId?: string, disabledUserIds?: Set<string>) {
  if (user.id === currentUserId) return '不能授权给自己'
  if (disabledUserIds?.has(user.id)) return '该用户已在共享授权列表中'
  return `选择 ${displayName(user)}`
}
