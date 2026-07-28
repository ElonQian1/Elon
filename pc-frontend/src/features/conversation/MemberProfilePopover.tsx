import { useEffect, useRef, useState } from 'react'
import { api } from '../../api/client'
import { clean, formatTime } from '../../lib/utils'
import type { Channel, ProjectMember, ProjectMemberAuditEntry, ProjectMemberAuditResponse, ProjectRoleRef } from './types'
import {
  memberChannelCapabilityLabels,
  memberChannelCanView,
  memberChannelPermissions,
  memberModerationSummary,
  memberPresenceStatus,
  memberPrimaryRoleKey,
  memberRoleSummary,
  presenceLabel,
  roleLabel,
} from './memberUtils'
import { styles as sharedStyles } from './memberPanelStyles'
import type { MemberModerationAction } from './memberPanelTypes'
import profileStyles from './MemberProfilePopover.module.css'

const styles = { ...sharedStyles, ...profileStyles }

function memberPopoverAvatarStatusClass(status: string) {
  if (status === 'idle') return styles.memberPopoverAvatarIdle
  if (status === 'dnd') return styles.memberPopoverAvatarDnd
  if (status === 'offline') return styles.memberPopoverAvatarOffline
  return styles.memberPopoverAvatarOnline
}

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
          memberPopoverAvatarStatusClass(status),
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
  update_member_profile: '更新成员资料',
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
