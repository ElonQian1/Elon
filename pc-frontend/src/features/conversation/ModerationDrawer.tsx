import { useState, useEffect, useMemo, useRef } from 'react'
import { api } from '../../api/client'
import type { ProjectMember, ProjectMemberAuditEntry, ProjectMemberAuditResponse } from './types'
import {
  filterMembers,
  formatDateTime,
  memberInitial,
  memberModerationSummary,
  roleLabel,
} from './memberUtils'
import styles from './ConversationPage.module.css'

type ModerationAction = 'mute' | 'unmute' | 'ban' | 'unban'
type ModerationFilter = 'all' | 'restricted' | 'muted' | 'banned' | 'normal'
type ModerationSort = 'status' | 'expires' | 'name' | 'joined'

const MODERATION_FILTERS: Array<{ id: ModerationFilter; label: string }> = [
  { id: 'all', label: '全部' },
  { id: 'restricted', label: '受限' },
  { id: 'muted', label: '禁言' },
  { id: 'banned', label: '封禁' },
  { id: 'normal', label: '正常' },
]

const MUTE_PRESETS = [
  { id: '10m', label: '10 分钟', minutes: 10 },
  { id: '1h', label: '1 小时', minutes: 60 },
  { id: '1d', label: '1 天', minutes: 1440 },
  { id: '7d', label: '7 天', minutes: 10080 },
  { id: '30d', label: '30 天', minutes: 43200 },
]

const MODERATION_AUDIT_ACTIONS = new Set(['mute_member', 'unmute_member', 'ban_member', 'unban_member'])
const MODERATION_ROW_HEIGHT = 112
const MODERATION_LIST_OVERSCAN = 5
const MODERATION_LIST_MIN_WINDOW = 10

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

function memberRestrictionUntil(member: ProjectMember) {
  if (member.is_banned) return member.banned_until ? Date.parse(member.banned_until) : Number.MAX_SAFE_INTEGER
  if (member.is_muted && member.muted_until) return Date.parse(member.muted_until)
  return Number.MAX_SAFE_INTEGER
}

function matchesModerationFilter(member: ProjectMember, filter: ModerationFilter) {
  if (filter === 'all') return true
  if (filter === 'restricted') return !!(member.is_banned || member.is_muted)
  if (filter === 'muted') return !!member.is_muted
  if (filter === 'banned') return !!member.is_banned
  return !member.is_banned && !member.is_muted
}

function sortModerationMembers(members: ProjectMember[], sortMode: ModerationSort) {
  return [...members].sort((left, right) => {
    if (sortMode === 'name') return memberName(left).localeCompare(memberName(right))
    if (sortMode === 'joined') return memberJoinedTime(right) - memberJoinedTime(left) || memberName(left).localeCompare(memberName(right))
    if (sortMode === 'expires') return memberRestrictionUntil(left) - memberRestrictionUntil(right) || memberName(left).localeCompare(memberName(right))
    return memberRestrictionRank(right) - memberRestrictionRank(left) || memberName(left).localeCompare(memberName(right))
  })
}

export function ModerationDrawer({
  projectId,
  members,
  initialMemberId,
  onClose,
  onSaved,
}: {
  projectId: string
  members: ProjectMember[]
  initialMemberId?: string
  onClose: () => void
  onSaved: () => Promise<void>
}) {
  const [query, setQuery] = useState('')
  const [filter, setFilter] = useState<ModerationFilter>('all')
  const [sortMode, setSortMode] = useState<ModerationSort>('status')
  const [muteMinutes, setMuteMinutes] = useState(60)
  const [customMinutes, setCustomMinutes] = useState('')
  const [note, setNote] = useState('')
  const [auditEntries, setAuditEntries] = useState<ProjectMemberAuditEntry[]>([])
  const [listScrollTop, setListScrollTop] = useState(0)
  const [listHeight, setListHeight] = useState(0)
  const [message, setMessage] = useState('')
  const [busyMemberId, setBusyMemberId] = useState('')
  const listRef = useRef<HTMLDivElement | null>(null)
  const activeMuteMinutes = useMemo(() => moderationDurationMinutes(customMinutes, muteMinutes), [customMinutes, muteMinutes])
  const stats = useMemo(() => ({
    total: members.length,
    restricted: members.filter((member) => member.is_banned || member.is_muted).length,
    muted: members.filter((member) => member.is_muted).length,
    banned: members.filter((member) => member.is_banned).length,
    normal: members.filter((member) => !member.is_banned && !member.is_muted).length,
  }), [members])
  const focusedMember = useMemo(() => {
    if (!initialMemberId) return undefined
    return members.find((member) => member.user_id === initialMemberId)
  }, [initialMemberId, members])
  const activeCases = useMemo(
    () => sortModerationMembers(members.filter((member) => member.is_banned || member.is_muted), 'expires'),
    [members],
  )
  const visibleMembers = useMemo(() => {
    const searched = filterMembers(members, query)
    return sortModerationMembers(
      searched.filter((member) => matchesModerationFilter(member, filter)),
      sortMode,
    )
  }, [members, query, filter, sortMode])
  const visibleAuditEntries = useMemo(
    () => auditEntries.filter((entry) => MODERATION_AUDIT_ACTIONS.has(entry.action)).slice(0, 6),
    [auditEntries],
  )
  const listWindowSize = Math.max(
    MODERATION_LIST_MIN_WINDOW,
    Math.ceil((listHeight || MODERATION_ROW_HEIGHT * MODERATION_LIST_MIN_WINDOW) / MODERATION_ROW_HEIGHT) + MODERATION_LIST_OVERSCAN * 2,
  )
  const listStart = Math.max(0, Math.floor(listScrollTop / MODERATION_ROW_HEIGHT) - MODERATION_LIST_OVERSCAN)
  const listEnd = Math.min(visibleMembers.length, listStart + listWindowSize)
  const virtualMembers = visibleMembers.slice(listStart, listEnd)

  async function refreshAudit() {
    try {
      const data = await api.get<ProjectMemberAuditResponse>(`/api/projects/${encodeURIComponent(projectId)}/member-audit?limit=60`)
      setAuditEntries(data.entries ?? [])
    } catch {
      setAuditEntries([])
    }
  }

  useEffect(() => {
    refreshAudit()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId])
  useEffect(() => {
    if (!initialMemberId) return
    const member = members.find((item) => item.user_id === initialMemberId)
    if (!member) return
    setQuery(memberName(member))
    setFilter(member.is_banned ? 'banned' : member.is_muted ? 'muted' : 'all')
    setSortMode('status')
    setListScrollTop(0)
    if (listRef.current) listRef.current.scrollTop = 0
  }, [initialMemberId, members])
  useEffect(() => {
    setListScrollTop(0)
    if (listRef.current) listRef.current.scrollTop = 0
  }, [filter, query, sortMode, visibleMembers.length])
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

  async function moderate(member: ProjectMember, action: ModerationAction, durationMinutes?: number) {
    setMessage('提交中…')
    setBusyMemberId(member.user_id)
    try {
      await api.patch(`/api/projects/${encodeURIComponent(projectId)}/members/${encodeURIComponent(member.user_id)}/moderation`, {
        action,
        duration_minutes: durationMinutes,
        note: note.trim() || moderationActionNote(action, durationMinutes),
      })
      setMessage('已更新')
      await onSaved()
      await refreshAudit()
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
            <strong>成员限制中心</strong>
            <span>
              {message || (focusedMember
                ? `已定位 ${memberName(focusedMember)} · ${memberModerationSummary(focusedMember)}`
                : `${stats.restricted} 个当前案件 · ${stats.muted} 个禁言 · ${stats.banned} 个封禁`)}
            </span>
          </div>
          <div className={styles.drawerHeaderActions}>
            <button className={styles.drawerCloseBtn} onClick={refreshAudit}>刷新记录</button>
            <button className={styles.drawerCloseBtn} onClick={onClose}>关闭</button>
          </div>
        </header>
        <div className={styles.drawerBody}>
          <div className={styles.moderationStats}>
            {MODERATION_FILTERS.map((item) => {
              const count = item.id === 'all' ? stats.total : item.id === 'restricted' ? stats.restricted : item.id === 'muted' ? stats.muted : item.id === 'banned' ? stats.banned : stats.normal
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

          {focusedMember && (
            <section
              className={styles.moderationFocusPanel}
              data-state={focusedMember.is_banned ? 'banned' : focusedMember.is_muted ? 'muted' : 'normal'}
            >
              <span className={[styles.memberAvatar, focusedMember.is_banned ? styles.moderationAvatarBanned : focusedMember.is_muted ? styles.moderationAvatarMuted : ''].join(' ')}>
                {focusedMember.avatar_data_url
                  ? <img src={focusedMember.avatar_data_url} alt="" />
                  : memberInitial(focusedMember)
                }
              </span>
              <div className={styles.moderationFocusInfo}>
                <strong>已定位成员 · {memberName(focusedMember)}</strong>
                <span>{memberModerationSummary(focusedMember)} · {memberRoleLabel(focusedMember)} · {restrictionUntilLabel(focusedMember)}</span>
              </div>
              <button
                type="button"
                className={styles.drawerCloseBtn}
                onClick={() => {
                  setQuery(memberName(focusedMember))
                  setFilter(focusedMember.is_banned ? 'banned' : focusedMember.is_muted ? 'muted' : 'all')
                  setSortMode('status')
                  setListScrollTop(0)
                  if (listRef.current) listRef.current.scrollTop = 0
                }}
              >
                回到该成员
              </button>
            </section>
          )}

          <section className={styles.moderationControlPanel}>
            <div className={styles.moderationControlHead}>
              <strong>处理策略</strong>
              <span>禁言最长 30 天；封禁为永久封禁，可在案件列表中解封。</span>
            </div>
            <div className={styles.moderationPresetGrid}>
              {MUTE_PRESETS.map((preset) => (
                <button
                  key={preset.id}
                  type="button"
                  data-active={!customMinutes && muteMinutes === preset.minutes ? 'true' : undefined}
                  onClick={() => {
                    setMuteMinutes(preset.minutes)
                    setCustomMinutes('')
                  }}
                >
                  {preset.label}
                </button>
              ))}
            </div>
            <div className={styles.moderationPolicyGrid}>
              <label className={styles.field}>
                <span>自定义禁言分钟</span>
                <input
                  value={customMinutes}
                  onChange={(event) => setCustomMinutes(event.target.value)}
                  inputMode="numeric"
                  placeholder={`当前 ${durationLabel(activeMuteMinutes)}`}
                />
              </label>
              <label className={styles.field}>
                <span>操作原因</span>
                <input
                  value={note}
                  onChange={(event) => setNote(event.target.value)}
                  maxLength={120}
                  placeholder="例如：刷屏、辱骂、违规广告"
                />
              </label>
            </div>
          </section>

          <section className={styles.moderationCaseSummary}>
            <div className={styles.moderationControlHead}>
              <strong>当前限制案件</strong>
              <span>{activeCases.length ? '优先处理即将到期或仍受限的成员' : '暂无禁言或封禁案件'}</span>
            </div>
            <div className={styles.moderationCaseStrip}>
              {activeCases.slice(0, 4).map((member) => (
                <button
                  key={member.user_id}
                  type="button"
                  onClick={() => {
                    setQuery(memberName(member))
                    setFilter(member.is_banned ? 'banned' : 'muted')
                  }}
                >
                  <strong>{memberName(member)}</strong>
                  <span>{restrictionStatusLabel(member)} · {restrictionUntilLabel(member)}</span>
                </button>
              ))}
              {activeCases.length === 0 && <p className={styles.sideHint}>成员状态正常</p>}
            </div>
          </section>

          <div className={styles.moderationToolbar}>
            <input className={styles.drawerSearchInput} value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索成员" />
            <select value={sortMode} onChange={(event) => setSortMode(event.target.value as ModerationSort)} aria-label="成员排序">
              <option value="status">按状态</option>
              <option value="expires">按到期</option>
              <option value="name">按名称</option>
              <option value="joined">按加入</option>
            </select>
          </div>
          <div
            ref={listRef}
            className={styles.moderationVirtualList}
            onScroll={(event) => setListScrollTop(event.currentTarget.scrollTop)}
          >
            {visibleMembers.length === 0 && <p className={styles.sideHint}>没有匹配成员</p>}
            {visibleMembers.length > 0 && (
              <div className={styles.moderationVirtualCanvas} style={{ height: visibleMembers.length * MODERATION_ROW_HEIGHT }}>
                <div style={{ transform: `translateY(${listStart * MODERATION_ROW_HEIGHT}px)` }}>
                  {virtualMembers.map((member) => {
                    const isBusy = busyMemberId === member.user_id
                    const restriction = restrictionStatusLabel(member)
                    return (
                      <div key={member.user_id} className={styles.moderationVirtualSlot}>
                        <article
                          className={styles.moderationRow}
                          data-state={member.is_banned ? 'banned' : member.is_muted ? 'muted' : 'normal'}
                          data-focus={member.user_id === focusedMember?.user_id ? 'true' : undefined}
                        >
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
                              <em>{memberRoleLabel(member)}</em>
                              <em>{restrictionUntilLabel(member)}</em>
                              {member.joined_at && <em>加入 {new Date(member.joined_at).toLocaleDateString()}</em>}
                            </div>
                          </div>
                          <div className={styles.moderationActions}>
                            <button type="button" className={styles.drawerCloseBtn} onClick={() => moderate(member, 'mute', activeMuteMinutes)} disabled={isBusy || !!member.is_banned}>禁言 {durationLabel(activeMuteMinutes)}</button>
                            <button type="button" className={styles.drawerCloseBtn} onClick={() => moderate(member, 'unmute')} disabled={isBusy || !member.is_muted}>解禁言</button>
                            <button type="button" className={styles.dangerBtn} onClick={() => moderate(member, 'ban')} disabled={isBusy || !!member.is_banned}>永久封禁</button>
                            <button type="button" className={styles.drawerCloseBtn} onClick={() => moderate(member, 'unban')} disabled={isBusy || !member.is_banned}>解封</button>
                          </div>
                        </article>
                      </div>
                    )
                  })}
                </div>
              </div>
            )}
          </div>
          <section className={styles.moderationAuditPanel}>
            <div className={styles.moderationControlHead}>
              <strong>最近处理记录</strong>
              <span>来自成员日志的禁言、解禁言、封禁和解封动作。</span>
            </div>
            <div className={styles.moderationAuditList}>
              {visibleAuditEntries.map((entry) => (
                <article key={entry.id}>
                  <strong>{moderationAuditLabel(entry.action)}</strong>
                  <span>{entry.target_account || entry.target_user_id || '成员'} · {new Date(entry.created_at).toLocaleString()}</span>
                  {entry.note && <em>{entry.note}</em>}
                </article>
              ))}
              {visibleAuditEntries.length === 0 && <p className={styles.sideHint}>暂无处理记录</p>}
            </div>
          </section>
        </div>
      </section>
    </div>
  )
}

function moderationDurationMinutes(customMinutes: string, fallback: number) {
  const parsed = Number(customMinutes.trim())
  if (!Number.isFinite(parsed) || parsed <= 0) return fallback
  return Math.min(43200, Math.max(1, Math.floor(parsed)))
}

function durationLabel(minutes: number) {
  if (minutes >= 1440 && minutes % 1440 === 0) return `${minutes / 1440} 天`
  if (minutes >= 60 && minutes % 60 === 0) return `${minutes / 60} 小时`
  return `${minutes} 分钟`
}

function moderationActionNote(action: ModerationAction, durationMinutes?: number) {
  if (action === 'mute') return `PC 成员限制中心 · 禁言 ${durationLabel(durationMinutes ?? 60)}`
  if (action === 'ban') return 'PC 成员限制中心 · 永久封禁'
  if (action === 'unmute') return 'PC 成员限制中心 · 解除禁言'
  return 'PC 成员限制中心 · 解封'
}

function restrictionStatusLabel(member: ProjectMember) {
  if (member.is_banned) return '已封禁'
  if (member.is_muted) return '已禁言'
  return '正常'
}

function restrictionUntilLabel(member: ProjectMember) {
  if (member.is_banned) return member.banned_until ? `封禁至 ${formatDateTime(member.banned_until)}` : '永久封禁'
  if (member.is_muted) return member.muted_until ? `禁言至 ${formatDateTime(member.muted_until)}` : '禁言中'
  return '无限制'
}

function memberRoleLabel(member: ProjectMember) {
  const role = member.roles?.[0]
  if (role?.name) return role.name
  return roleLabel(member.role ?? 'member')
}

function moderationAuditLabel(action: string) {
  const labels: Record<string, string> = {
    mute_member: '禁言',
    unmute_member: '解除禁言',
    ban_member: '封禁',
    unban_member: '解封',
  }
  return labels[action] ?? action
}
