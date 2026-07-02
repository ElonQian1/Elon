import { useState, useMemo } from 'react'
import { api } from '../../api/client'
import type { ProjectMember } from './types'
import { filterMembers, memberInitial, memberModerationSummary } from './memberUtils'
import styles from './ConversationPage.module.css'

type ModerationAction = 'mute' | 'unmute' | 'ban' | 'unban'
type ModerationFilter = 'all' | 'restricted' | 'muted' | 'banned'
type ModerationSort = 'status' | 'name' | 'joined'

const MODERATION_FILTERS: Array<{ id: ModerationFilter; label: string }> = [
  { id: 'all', label: '全部' },
  { id: 'restricted', label: '受限' },
  { id: 'muted', label: '禁言' },
  { id: 'banned', label: '封禁' },
]

function memberName(member: ProjectMember) {
  return member.account || member.user_id
}

function memberJoinedTime(member: ProjectMember) {
  const timestamp = Date.parse(member.joined_at ?? '')
  return Number.isNaN(timestamp) ? 0 : timestamp
}

function memberRestrictionRank(member: ProjectMember) {
  if (member.is_banned) return 3
  if (member.is_muted) return 2
  return 1
}

function matchesModerationFilter(member: ProjectMember, filter: ModerationFilter) {
  if (filter === 'all') return true
  if (filter === 'restricted') return !!(member.is_banned || member.is_muted)
  if (filter === 'muted') return !!member.is_muted
  return !!member.is_banned
}

function sortModerationMembers(members: ProjectMember[], sortMode: ModerationSort) {
  return [...members].sort((left, right) => {
    if (sortMode === 'name') return memberName(left).localeCompare(memberName(right))
    if (sortMode === 'joined') return memberJoinedTime(right) - memberJoinedTime(left) || memberName(left).localeCompare(memberName(right))
    return memberRestrictionRank(right) - memberRestrictionRank(left) || memberName(left).localeCompare(memberName(right))
  })
}

export function ModerationDrawer({
  projectId,
  members,
  onClose,
  onSaved,
}: {
  projectId: string
  members: ProjectMember[]
  onClose: () => void
  onSaved: () => Promise<void>
}) {
  const [query, setQuery] = useState('')
  const [filter, setFilter] = useState<ModerationFilter>('all')
  const [sortMode, setSortMode] = useState<ModerationSort>('status')
  const [message, setMessage] = useState('')
  const [busyMemberId, setBusyMemberId] = useState('')
  const stats = useMemo(() => ({
    total: members.length,
    restricted: members.filter((member) => member.is_banned || member.is_muted).length,
    muted: members.filter((member) => member.is_muted).length,
    banned: members.filter((member) => member.is_banned).length,
  }), [members])
  const visibleMembers = useMemo(() => {
    const searched = filterMembers(members, query)
    return sortModerationMembers(
      searched.filter((member) => matchesModerationFilter(member, filter)),
      sortMode,
    )
  }, [members, query, filter, sortMode])

  async function moderate(member: ProjectMember, action: ModerationAction, durationMinutes?: number) {
    setMessage('提交中…')
    setBusyMemberId(member.user_id)
    try {
      await api.patch(`/api/projects/${encodeURIComponent(projectId)}/members/${encodeURIComponent(member.user_id)}/moderation`, {
        action,
        duration_minutes: durationMinutes,
        note: 'PC 成员管理页操作',
      })
      setMessage('已更新')
      await onSaved()
    } catch (err) {
      setMessage((err as { message?: string }).message ?? '操作失败')
    } finally {
      setBusyMemberId('')
    }
  }

  return (
    <div className={styles.drawerBackdrop}>
      <section className={[styles.permissionDrawer, styles.moderationDrawer].join(' ')} role="dialog" aria-modal="true">
        <header className={styles.drawerHeader}>
          <div>
            <strong>禁言与封禁</strong>
            <span>{message || `${stats.total} 位成员 · ${stats.restricted} 位受限`}</span>
          </div>
          <button className={styles.drawerCloseBtn} onClick={onClose}>关闭</button>
        </header>
        <div className={styles.drawerBody}>
          <div className={styles.moderationStats}>
            {MODERATION_FILTERS.map((item) => {
              const count = item.id === 'all' ? stats.total : item.id === 'restricted' ? stats.restricted : item.id === 'muted' ? stats.muted : stats.banned
              return (
                <button
                  key={item.id}
                  type="button"
                  data-active={filter === item.id ? 'true' : undefined}
                  onClick={() => setFilter(item.id)}
                >
                  <strong>{count}</strong>
                  <span>{item.label}</span>
                </button>
              )
            })}
          </div>
          <div className={styles.moderationToolbar}>
            <input className={styles.drawerSearchInput} value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索成员" />
            <select value={sortMode} onChange={(event) => setSortMode(event.target.value as ModerationSort)} aria-label="成员排序">
              <option value="status">按状态</option>
              <option value="name">按名称</option>
              <option value="joined">按加入</option>
            </select>
          </div>
          <div className={styles.moderationList}>
            {visibleMembers.map((member) => {
              const isBusy = busyMemberId === member.user_id
              const restriction = member.is_banned ? '已封禁' : member.is_muted ? '已禁言' : '正常'
              return (
                <article key={member.user_id} className={styles.moderationRow}>
                  <span className={[styles.memberAvatar, member.is_banned ? styles.moderationAvatarBanned : member.is_muted ? styles.moderationAvatarMuted : ''].join(' ')}>
                    {member.avatar_data_url
                      ? <img src={member.avatar_data_url} alt="" />
                      : memberInitial(member)
                    }
                  </span>
                  <div className={styles.moderationInfo}>
                    <strong>{memberName(member)}</strong>
                    <span>{memberModerationSummary(member)}</span>
                    <div className={styles.moderationBadges}>
                      <em data-tone={member.is_banned ? 'danger' : member.is_muted ? 'warning' : 'normal'}>{restriction}</em>
                      {member.joined_at && <em>加入 {new Date(member.joined_at).toLocaleDateString()}</em>}
                    </div>
                  </div>
                  <div className={styles.moderationActions}>
                    <button type="button" className={styles.drawerCloseBtn} onClick={() => moderate(member, 'mute', 60)} disabled={isBusy || !!member.is_banned}>禁言1小时</button>
                    <button type="button" className={styles.drawerCloseBtn} onClick={() => moderate(member, 'mute', 1440)} disabled={isBusy || !!member.is_banned}>禁言1天</button>
                    <button type="button" className={styles.drawerCloseBtn} onClick={() => moderate(member, 'unmute')} disabled={isBusy || !member.is_muted}>解禁言</button>
                    <button type="button" className={styles.dangerBtn} onClick={() => moderate(member, 'ban')} disabled={isBusy || !!member.is_banned}>封禁</button>
                    <button type="button" className={styles.drawerCloseBtn} onClick={() => moderate(member, 'unban')} disabled={isBusy || !member.is_banned}>解封</button>
                  </div>
                </article>
              )
            })}
            {visibleMembers.length === 0 && <p className={styles.sideHint}>没有匹配成员</p>}
          </div>
        </div>
      </section>
    </div>
  )
}
