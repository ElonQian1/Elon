import { useEffect, useMemo, useState } from 'react'
import { api } from '../../api/client'
import { clean, formatTime } from '../../lib/utils'
import type { Channel, ProjectMember, ProjectMemberAuditEntry, ProjectMemberAuditResponse, ProjectRoleRef } from './types'
import type { MemberModerationAction } from './MemberPanel'
import {
  formatDateTime,
  memberChannelCanView,
  memberChannelCapabilityLabels,
  memberChannelPermissions,
  memberInitial,
  memberModerationSummary,
  memberPresenceStatus,
  memberRoleSummary,
  presenceLabel,
  roleLabel,
} from './memberUtils'
import styles from './ConversationPage.module.css'

type MemberDetailChannelRow = {
  channel: Channel
  canView: boolean
  labels: string[]
  inherited: boolean
}

const DETAIL_AUDIT_LABELS: Record<string, string> = {
  add_member: '添加成员',
  invite_member: '邀请成员',
  join_by_invite_link: '通过邀请加入',
  update_member_role: '调整角色',
  remove_member: '移除成员',
  mute_member: '禁言成员',
  unmute_member: '解除禁言',
  ban_member: '封禁成员',
  unban_member: '解封成员',
  update_channel_member_permission: '更新成员频道权限',
  update_category_member_permission: '更新成员分类权限',
}

export function MemberDetailDrawer({
  projectId,
  member,
  channels,
  currentChannel,
  canModerate,
  canRemove,
  canManageRoles,
  canManagePermissions,
  onClose,
  onOpenConversations,
  onOpenRoles,
  onOpenPermissions,
  onModerate,
  onRemove,
}: {
  projectId: string
  member: ProjectMember
  channels: Channel[]
  currentChannel?: Channel
  canModerate?: boolean
  canRemove?: boolean
  canManageRoles?: boolean
  canManagePermissions?: boolean
  onClose: () => void
  onOpenConversations?: (member: ProjectMember) => void
  onOpenRoles?: (member: ProjectMember) => void
  onOpenPermissions?: (member: ProjectMember) => void
  onModerate?: (member: ProjectMember, action: MemberModerationAction, durationMinutes?: number) => Promise<void>
  onRemove?: (member: ProjectMember) => Promise<boolean | void>
}) {
  const [auditEntries, setAuditEntries] = useState<ProjectMemberAuditEntry[]>([])
  const [auditLoading, setAuditLoading] = useState(false)
  const [statusMsg, setStatusMsg] = useState('')
  const [busyAction, setBusyAction] = useState<MemberModerationAction | ''>('')
  const [removing, setRemoving] = useState(false)
  const name = member.account || member.user_id
  const presence = memberPresenceStatus(member)
  const roleChips = memberDetailRoles(member)
  const channelRows = useMemo(
    () => buildChannelRows(member, channels, currentChannel?.id),
    [member, channels, currentChannel?.id],
  )
  const visibleCount = channelRows.filter((row) => row.canView).length
  const inheritedCount = channelRows.filter((row) => row.inherited).length
  const hiddenCount = channelRows.length - visibleCount
  const relatedAuditEntries = useMemo(() => {
    const targetId = clean(member.user_id)
    const targetAccount = clean(member.account ?? '').toLowerCase()
    return auditEntries
      .filter((entry) => {
        const entryTargetId = clean(entry.target_user_id ?? '')
        const entryTargetAccount = clean(entry.target_account ?? '').toLowerCase()
        return (!!targetId && entryTargetId === targetId)
          || (!!targetAccount && entryTargetAccount === targetAccount)
      })
      .slice(0, 10)
  }, [auditEntries, member.user_id, member.account])

  async function refreshAudit() {
    setAuditLoading(true)
    try {
      const data = await api.get<ProjectMemberAuditResponse>(`/api/projects/${encodeURIComponent(projectId)}/member-audit?limit=120`)
      setAuditEntries(data.entries ?? [])
      setStatusMsg('')
    } catch (err) {
      setAuditEntries([])
      setStatusMsg((err as { message?: string }).message ?? '成员记录读取失败')
    } finally {
      setAuditLoading(false)
    }
  }

  useEffect(() => {
    refreshAudit()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId, member.user_id])

  function copyId() {
    navigator.clipboard.writeText(member.user_id).catch(() => {})
    setStatusMsg('已复制用户 ID')
  }

  function openConversations() {
    onOpenConversations?.(member)
    onClose()
  }

  function openRoles() {
    onOpenRoles?.(member)
    onClose()
  }

  function openPermissions() {
    onOpenPermissions?.(member)
    onClose()
  }

  async function moderate(action: MemberModerationAction, durationMinutes?: number) {
    if (!onModerate || busyAction) return
    setBusyAction(action)
    setStatusMsg('提交中...')
    try {
      await onModerate(member, action, durationMinutes)
      setStatusMsg('成员状态已更新')
      await refreshAudit()
    } catch (err) {
      setStatusMsg((err as { message?: string }).message ?? '操作失败')
    } finally {
      setBusyAction('')
    }
  }

  async function removeMember() {
    if (!onRemove || removing) return
    setRemoving(true)
    setStatusMsg('移除中...')
    try {
      const removed = await onRemove(member)
      if (removed !== false) onClose()
      else setStatusMsg('')
    } catch (err) {
      setStatusMsg((err as { message?: string }).message ?? '移除失败')
    } finally {
      setRemoving(false)
    }
  }

  return (
    <div className={styles.drawerBackdrop}>
      <section className={[styles.permissionDrawer, styles.memberDetailDrawer].join(' ')} role="dialog" aria-modal="true">
        <header className={styles.drawerHeader}>
          <div>
            <strong>成员详情</strong>
            <span>{statusMsg || `${presenceLabel(presence)} · ${memberRoleSummary(member)}`}</span>
          </div>
          <div className={styles.drawerHeaderActions}>
            <button className={styles.drawerCloseBtn} onClick={refreshAudit} disabled={auditLoading}>刷新记录</button>
            <button className={styles.drawerCloseBtn} onClick={onClose}>关闭</button>
          </div>
        </header>

        <div className={styles.memberDetailBody}>
          <section className={styles.memberDetailHero}>
            <div className={styles.memberDetailAvatarWrap}>
              <span className={[styles.memberDetailAvatar, styles[`memberAvatar${capitalizePresence(presence)}`] ?? ''].join(' ')}>
                {member.avatar_data_url
                  ? <img src={member.avatar_data_url} alt="" />
                  : memberInitial(member)
                }
              </span>
            </div>
            <div className={styles.memberDetailHeroCopy}>
              <strong>{name}</strong>
              <span>{presenceLabel(presence)} · {memberModerationSummary(member)}</span>
              <div className={styles.memberDetailBadges}>
                <em>{roleChips.length} 个角色</em>
                <em>{visibleCount}/{channels.length} 个可见频道</em>
                {hiddenCount > 0 && <em data-tone="danger">隐藏 {hiddenCount}</em>}
                {member.custom_status && <em>{member.custom_status}</em>}
                {member.activity && <em>{member.activity}</em>}
              </div>
            </div>
          </section>

          <div className={styles.memberDetailActionBar}>
            {onOpenConversations && <button className={styles.primaryBtn} onClick={openConversations}>打开会话</button>}
            <button className={styles.drawerCloseBtn} onClick={copyId}>复制 ID</button>
            {canManageRoles && onOpenRoles && <button className={styles.drawerCloseBtn} onClick={openRoles}>编辑角色</button>}
            {canManagePermissions && onOpenPermissions && <button className={styles.drawerCloseBtn} onClick={openPermissions}>频道权限</button>}
            {canRemove && onRemove && <button className={styles.dangerBtn} onClick={removeMember} disabled={removing || !!busyAction}>{removing ? '移除中...' : '移除成员'}</button>}
          </div>

          <div className={styles.memberDetailGrid}>
            <section className={styles.memberDetailCard}>
              <div className={styles.memberDetailCardHead}>
                <strong>身份</strong>
                <span>{roleChips.length} 个角色</span>
              </div>
              <div className={styles.memberDetailRoleList}>
                {roleChips.map((role) => (
                  <em
                    key={role.id}
                    style={role.color ? { color: role.color, borderColor: role.color } : undefined}
                  >
                    {role.name || roleLabel(role.id)}
                  </em>
                ))}
              </div>
              <div className={styles.memberDetailKv}>
                <span>账号</span>
                <strong title={member.account || '-'}>{member.account || '-'}</strong>
                <span>用户 ID</span>
                <strong title={member.user_id}>{member.user_id}</strong>
                <span>加入时间</span>
                <strong>{member.joined_at ? formatTime(member.joined_at) : '-'}</strong>
              </div>
            </section>

            <section className={styles.memberDetailCard}>
              <div className={styles.memberDetailCardHead}>
                <strong>管理状态</strong>
                <span>{memberModerationSummary(member)}</span>
              </div>
              <div className={styles.memberDetailKv}>
                <span>禁言至</span>
                <strong>{member.is_muted ? formatDateTime(member.muted_until) : '未禁言'}</strong>
                <span>封禁状态</span>
                <strong>{member.is_banned ? `已封禁 · ${formatDateTime(member.banned_until)}` : '未封禁'}</strong>
              </div>
              {canModerate && onModerate && (
                <div className={styles.memberDetailModerationGrid}>
                  <button className={styles.drawerCloseBtn} onClick={() => moderate('mute', 60)} disabled={!!busyAction || !!member.is_banned || removing}>禁言 1 小时</button>
                  <button className={styles.drawerCloseBtn} onClick={() => moderate('mute', 1440)} disabled={!!busyAction || !!member.is_banned || removing}>禁言 1 天</button>
                  <button className={styles.drawerCloseBtn} onClick={() => moderate('unmute')} disabled={!!busyAction || !member.is_muted || removing}>解禁言</button>
                  <button className={styles.dangerBtn} onClick={() => moderate('ban')} disabled={!!busyAction || !!member.is_banned || removing}>封禁</button>
                  <button className={styles.drawerCloseBtn} onClick={() => moderate('unban')} disabled={!!busyAction || !member.is_banned || removing}>解封</button>
                </div>
              )}
            </section>

            <section className={[styles.memberDetailCard, styles.memberDetailWideCard].join(' ')}>
              <div className={styles.memberDetailCardHead}>
                <strong>频道上下文</strong>
                <span>{visibleCount} 可见 · {hiddenCount} 隐藏 · {inheritedCount} 继承</span>
              </div>
              {currentChannel && (
                <div className={styles.memberDetailCurrentChannel}>
                  <span>当前频道</span>
                  <strong>{currentChannel.name}</strong>
                  <em>{currentChannel.category_name || channelKindLabel(currentChannel.kind)}</em>
                </div>
              )}
              <div className={styles.memberDetailChannelList}>
                {channelRows.map((row) => (
                  <article key={row.channel.id} className={styles.memberDetailChannelRow} data-hidden={!row.canView ? 'true' : undefined}>
                    <span>{channelKindMark(row.channel.kind)}</span>
                    <div>
                      <strong title={row.channel.name}>{row.channel.name}</strong>
                      <em>{row.channel.category_name || channelKindLabel(row.channel.kind)}</em>
                    </div>
                    <p>
                      {row.inherited ? '继承项目角色' : row.labels.join(' / ')}
                    </p>
                  </article>
                ))}
                {channelRows.length === 0 && <p className={styles.sideHint}>暂无频道</p>}
              </div>
            </section>

            <section className={[styles.memberDetailCard, styles.memberDetailWideCard].join(' ')}>
              <div className={styles.memberDetailCardHead}>
                <strong>近期记录</strong>
                <span>{auditLoading ? '同步中' : `${relatedAuditEntries.length} 条`}</span>
              </div>
              <div className={styles.memberDetailAuditList}>
                {auditLoading && relatedAuditEntries.length === 0 && <p className={styles.sideHint}>正在读取成员记录...</p>}
                {!auditLoading && relatedAuditEntries.length === 0 && <p className={styles.sideHint}>{statusMsg || '暂无近期成员记录'}</p>}
                {relatedAuditEntries.map((entry) => (
                  <article key={entry.id} className={styles.memberDetailAuditRow}>
                    <div>
                      <strong>{auditActionLabel(entry.action)}</strong>
                      <time>{formatTime(entry.created_at)}</time>
                    </div>
                    <span>{auditSummary(entry)}</span>
                  </article>
                ))}
              </div>
            </section>
          </div>
        </div>
      </section>
    </div>
  )
}

function buildChannelRows(member: ProjectMember, channels: Channel[], currentChannelId?: string): MemberDetailChannelRow[] {
  return channels
    .map((channel) => {
      const permissions = memberChannelPermissions(member, channel.id)
      const inherited = !permissions
      const canView = inherited || memberChannelCanView(permissions)
      return {
        channel,
        canView,
        inherited,
        labels: inherited ? [] : memberChannelCapabilityLabels(permissions),
      }
    })
    .sort((left, right) => {
      if (left.channel.id === currentChannelId) return -1
      if (right.channel.id === currentChannelId) return 1
      if (left.canView !== right.canView) return left.canView ? -1 : 1
      return (left.channel.category_position ?? 0) - (right.channel.category_position ?? 0)
        || left.channel.name.localeCompare(right.channel.name)
    })
}

function memberDetailRoles(member: ProjectMember): ProjectRoleRef[] {
  if (member.roles?.length) return member.roles
  const fallbackId = clean(member.role ?? 'member') || 'member'
  return [{ id: fallbackId, name: roleLabel(fallbackId), builtin: true }]
}

function capitalizePresence(status: string) {
  if (status === 'dnd') return 'Dnd'
  return `${status.slice(0, 1).toUpperCase()}${status.slice(1)}`
}

function channelKindMark(kind?: string) {
  const normalized = clean(kind ?? '').toLowerCase()
  if (normalized === 'ai_development') return 'AI'
  if (normalized === 'builds') return '包'
  if (normalized === 'announce' || normalized === 'announcements') return '告'
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

function auditActionLabel(action: string) {
  return DETAIL_AUDIT_LABELS[clean(action)] ?? (clean(action) || '成员操作')
}

function auditSummary(entry: ProjectMemberAuditEntry) {
  const actor = clean(entry.actor_account ?? entry.actor_user_id ?? '') || '系统'
  const oldRole = clean(entry.old_role ?? '')
  const newRole = clean(entry.new_role ?? '')
  const parts = [`操作者 ${actor}`]
  if (oldRole || newRole) {
    if (oldRole && newRole) parts.push(`${roleLabel(oldRole)} -> ${roleLabel(newRole)}`)
    else parts.push(roleLabel(newRole || oldRole))
  }
  const note = clean(entry.note ?? '')
  if (note) parts.push(formatAuditNote(note))
  return parts.join(' · ')
}

function formatAuditNote(note: string) {
  const first = note.split(';').map((part) => part.trim()).filter(Boolean)[0] ?? note
  const index = first.indexOf('=')
  if (index < 0) return first
  const labels: Record<string, string> = {
    channel_id: '频道',
    category_id: '分类',
    reason: '原因',
    duration_minutes: '时长',
    invite_code: '邀请码',
  }
  const key = first.slice(0, index)
  const value = first.slice(index + 1)
  return `${labels[key] ?? key}: ${value || '-'}`
}
