import { clean } from '../../lib/utils'
import type {
  Channel,
  ChannelPermissions,
  PermissionOption,
  PermissionOverride,
  ProjectInvitePreview,
  ProjectMember,
  ProjectRole,
} from './types'

export const CHANNEL_PERMISSION_OPTIONS: PermissionOption[] = [
  { key: 'view_channel', label: '查看频道' },
  { key: 'send_messages', label: '发送消息' },
  { key: 'start_ai_tasks', label: '发起 AI 任务' },
  { key: 'manage_channel', label: '管理频道权限' },
]

export const ROLE_PERMISSION_VIEW_MEMBERS = 'view_members'
export const ROLE_PERMISSION_SEND_MESSAGES = 'send_messages'
export const ROLE_PERMISSION_INVITE_MEMBERS = 'invite_members'
export const ROLE_PERMISSION_MANAGE_MEMBERS = 'manage_members'
export const ROLE_PERMISSION_MODERATE_MEMBERS = 'moderate_members'
export const ROLE_PERMISSION_VIEW_AUDIT_LOG = 'view_audit_log'
export const ROLE_PERMISSION_MANAGE_ROLES = 'manage_roles'
export const ROLE_PERMISSION_MANAGE_PROJECT_SETTINGS = 'manage_project_settings'

export const BUILTIN_ROLE_PERMISSIONS: Record<string, string[]> = {
  owner: [
    ROLE_PERMISSION_VIEW_MEMBERS,
    ROLE_PERMISSION_SEND_MESSAGES,
    ROLE_PERMISSION_INVITE_MEMBERS,
    ROLE_PERMISSION_MANAGE_MEMBERS,
    ROLE_PERMISSION_MODERATE_MEMBERS,
    ROLE_PERMISSION_VIEW_AUDIT_LOG,
    ROLE_PERMISSION_MANAGE_ROLES,
    ROLE_PERMISSION_MANAGE_PROJECT_SETTINGS,
  ],
  admin: [
    ROLE_PERMISSION_VIEW_MEMBERS,
    ROLE_PERMISSION_SEND_MESSAGES,
    ROLE_PERMISSION_INVITE_MEMBERS,
    ROLE_PERMISSION_MANAGE_MEMBERS,
    ROLE_PERMISSION_MODERATE_MEMBERS,
    ROLE_PERMISSION_VIEW_AUDIT_LOG,
    ROLE_PERMISSION_MANAGE_ROLES,
    ROLE_PERMISSION_MANAGE_PROJECT_SETTINGS,
  ],
  editor: [ROLE_PERMISSION_VIEW_MEMBERS, ROLE_PERMISSION_SEND_MESSAGES],
  developer: [ROLE_PERMISSION_VIEW_MEMBERS, ROLE_PERMISSION_SEND_MESSAGES],
  maintainer: [ROLE_PERMISSION_VIEW_MEMBERS, ROLE_PERMISSION_SEND_MESSAGES],
  member: [ROLE_PERMISSION_VIEW_MEMBERS, ROLE_PERMISSION_SEND_MESSAGES],
  observer: [ROLE_PERMISSION_VIEW_MEMBERS],
  viewer: [ROLE_PERMISSION_VIEW_MEMBERS],
}

export const PRESENCE_OPTIONS = [
  { value: 'online', label: '在线' },
  { value: 'idle', label: '离开' },
  { value: 'dnd', label: '勿扰' },
  { value: 'invisible', label: '隐身' },
]

export function channelCanManage(channel: Channel) {
  const permissions = channel.permissions ?? {}
  return !!(permissions.can_manage || permissions.canManage)
}

export function channelPermissionSummary(
  channel: Channel,
  visibleCount?: number,
  totalCount?: number,
  usingChannelPermissions?: boolean,
) {
  const permissions = channel.permissions ?? {}
  const parts = ['可查看']
  if (permissions.can_send || permissions.canSend) parts.push('可发言')
  if (permissions.can_start_ai || permissions.canStartAi) parts.push('可启动 AI')
  if (permissions.can_manage || permissions.canManage) parts.push('可管理权限')
  const scope = usingChannelPermissions && typeof visibleCount === 'number' && typeof totalCount === 'number'
    ? `可见 ${visibleCount}/${totalCount} · `
    : ''
  return `${channel.name} · ${scope}${parts.join(' / ')}`
}

export function channelPermissionValue(permissions: ChannelPermissions | undefined, snakeKey: keyof ChannelPermissions, camelKey: keyof ChannelPermissions) {
  return !!(permissions?.[snakeKey] || permissions?.[camelKey])
}

export function memberChannelPermissions(member: ProjectMember, channelId?: string) {
  if (!channelId) return undefined
  return member.channel_permissions?.[channelId] ?? member.channelPermissions?.[channelId]
}

export function memberCanViewChannel(member: ProjectMember, channelId?: string) {
  const permissions = memberChannelPermissions(member, channelId)
  if (!permissions) return true
  return memberChannelCanView(permissions)
}

export function membersHaveChannelPermissionMap(members: ProjectMember[], channelId?: string) {
  if (!channelId) return false
  return members.some(member => !!memberChannelPermissions(member, channelId))
}

export function membersForChannel(members: ProjectMember[], channelId?: string) {
  if (!channelId || !membersHaveChannelPermissionMap(members, channelId)) return members
  return members.filter(member => memberCanViewChannel(member, channelId))
}

export function memberRoleIds(member: ProjectMember) {
  const ids = [
    member.role,
    ...(member.roles ?? []).map((role) => role.id),
  ]
  return Array.from(new Set(ids.map((id) => clean(id ?? '').toLowerCase()).filter(Boolean)))
}

export function memberPrimaryRoleKey(member: ProjectMember) {
  return clean(member.roles?.[0]?.id ?? member.role ?? 'member').toLowerCase()
}

export function projectMemberHasRolePermission(member: ProjectMember, roles: ProjectRole[], permission: string) {
  return memberRoleIds(member).some((roleId) => {
    if (roleId === 'owner') return true
    const role = roles.find((item) => clean(item.id).toLowerCase() === roleId)
    const permissions = role?.permissions?.length ? role.permissions : BUILTIN_ROLE_PERMISSIONS[roleId]
    return !!permissions?.includes(permission)
  })
}

export function projectedMemberChannelPermissions({
  member,
  channel,
  categoryId,
  roles,
  categoryRoleOverrides,
  categoryMemberOverrides,
  channelRoleOverrides,
  channelMemberOverrides,
}: {
  member: ProjectMember
  channel?: Channel
  categoryId?: string
  roles: ProjectRole[]
  categoryRoleOverrides: PermissionOverride[]
  categoryMemberOverrides: PermissionOverride[]
  channelRoleOverrides: PermissionOverride[]
  channelMemberOverrides: PermissionOverride[]
}): ChannelPermissions {
  if (!channel) {
    return memberChannelPermissions(member) ?? {
      can_view: true,
      can_send: false,
      can_start_ai: false,
      can_manage: false,
    }
  }
  const roleIds = memberRoleIds(member)
  const channelKind = clean(channel.kind ?? '').toLowerCase()
  if (roleIds.includes('owner')) {
    return {
      can_view: true,
      can_send: channelKind !== 'docs',
      can_start_ai: channelKind === 'ai_development',
      can_manage: true,
    }
  }

  const applyDraft = (permission: string, base: boolean) => {
    let next = base
    if (channel.category_id && channel.category_id === categoryId && channel.permission_sync !== false) {
      next = applyRolePermissionOverrides(next, roleIds, categoryRoleOverrides, permission)
      next = applyMemberPermissionOverride(next, member.user_id, categoryMemberOverrides, permission)
    }
    next = applyRolePermissionOverrides(next, roleIds, channelRoleOverrides, permission)
    return applyMemberPermissionOverride(next, member.user_id, channelMemberOverrides, permission)
  }

  const canViewBase = projectMemberHasRolePermission(member, roles, ROLE_PERMISSION_VIEW_MEMBERS)
  const canSendBase = projectMemberHasRolePermission(member, roles, ROLE_PERMISSION_SEND_MESSAGES)
    && channelKind !== 'docs'
    && channelKind !== 'announcements'
  const canStartAiBase = ['owner', 'admin', 'editor', 'developer', 'maintainer']
    .includes(memberPrimaryRoleKey(member)) && channelKind === 'ai_development'
  const canManageBase = projectMemberHasRolePermission(member, roles, ROLE_PERMISSION_MANAGE_PROJECT_SETTINGS)

  return {
    can_view: applyDraft('view_channel', canViewBase),
    can_send: channelKind === 'docs' ? false : applyDraft('send_messages', canSendBase),
    can_start_ai: channelKind === 'ai_development' && applyDraft('start_ai_tasks', canStartAiBase),
    can_manage: applyDraft('manage_channel', canManageBase),
  }
}

export function applyRolePermissionOverrides(base: boolean, roleIds: string[], overrides: PermissionOverride[], permission: string) {
  if (roleIds.length === 0) return false
  let denied = false
  let allowed = false
  for (const override of overrides) {
    const roleId = clean(String(override.role_id ?? override.roleId ?? '')).toLowerCase()
    if (!roleId || !roleIds.includes(roleId)) continue
    if ((override.deny ?? []).includes(permission)) denied = true
    if ((override.allow ?? []).includes(permission)) allowed = true
  }
  if (denied) return false
  if (allowed) return true
  return base
}

export function applyMemberPermissionOverride(base: boolean, memberId: string, overrides: PermissionOverride[], permission: string) {
  const targetId = clean(memberId)
  const effects = overrides.filter((override) =>
    clean(String(override.user_id ?? override.userId ?? '')) === targetId
  )
  if (effects.some((override) => (override.deny ?? []).includes(permission))) return false
  if (effects.some((override) => (override.allow ?? []).includes(permission))) return true
  return base
}

export function channelPermissionsChanged(next: ChannelPermissions, current?: ChannelPermissions) {
  if (!current) return false
  return memberChannelCanView(next) !== memberChannelCanView(current)
    || channelPermissionValue(next, 'can_send', 'canSend') !== channelPermissionValue(current, 'can_send', 'canSend')
    || channelPermissionValue(next, 'can_start_ai', 'canStartAi') !== channelPermissionValue(current, 'can_start_ai', 'canStartAi')
    || channelPermissionValue(next, 'can_manage', 'canManage') !== channelPermissionValue(current, 'can_manage', 'canManage')
}

export function filterMembers(members: ProjectMember[], query: string) {
  const needle = clean(query).toLowerCase()
  if (!needle) return members
  return members.filter((member) => {
    const haystack = [
      member.account,
      member.global_account,
      member.member_display_name,
      member.admin_note,
      member.user_id,
      member.role,
      member.custom_status,
      member.activity,
      ...(member.roles ?? []).map((role) => role.name || role.id),
    ].join(' ').toLowerCase()
    return haystack.includes(needle)
  })
}

export function memberInitial(member: ProjectMember) {
  return clean(member.account ?? member.user_id).slice(0, 1).toUpperCase() || '员'
}

export function memberSubtitle(member: ProjectMember) {
  if (member.is_banned) return '已封禁'
  if (member.is_muted) return `禁言至 ${formatDateTime(member.muted_until)}`
  const activity = clean(member.activity ?? '')
  const customStatus = clean(member.custom_status ?? '')
  if (activity) return activity
  if (customStatus) return customStatus
  return memberRoleSummary(member)
}

export function memberChannelSubtitle(member: ProjectMember, channelId?: string) {
  const permissions = memberChannelPermissions(member, channelId)
  return memberChannelSubtitleForPermissions(member, permissions)
}

export function memberChannelSubtitleForPermissions(member: ProjectMember, permissions?: ChannelPermissions) {
  if (!permissions) return memberSubtitle(member)
  const parts = memberChannelCapabilityLabels(permissions)
  if (!memberChannelCanView(permissions)) return parts[0] ?? '无频道访问权限'
  return `${parts.join(' / ')} · ${memberSubtitle(member)}`
}

export function memberChannelCanView(permissions?: ChannelPermissions) {
  return channelPermissionValue(permissions, 'can_view', 'canView')
}

export function memberChannelCapabilityLabels(permissions?: ChannelPermissions) {
  if (!permissions) return []
  if (!memberChannelCanView(permissions)) return ['无频道访问权限']
  const parts = [
    channelPermissionValue(permissions, 'can_send', 'canSend') ? '可发言' : '只读',
  ]
  if (channelPermissionValue(permissions, 'can_start_ai', 'canStartAi')) parts.push('可启动 AI')
  if (channelPermissionValue(permissions, 'can_manage', 'canManage')) parts.push('可管理')
  return parts
}

export function memberPresenceStatus(member: ProjectMember) {
  const status = clean(member.presence_status ?? '').toLowerCase()
  if (!member.is_online || status === 'offline' || status === 'invisible') return 'offline'
  if (status === 'idle' || status === 'dnd') return status
  return 'online'
}

export function presenceLabel(status: string) {
  const labels: Record<string, string> = {
    online: '在线',
    idle: '离开',
    dnd: '勿扰',
    invisible: '隐身',
    offline: '离线',
  }
  return labels[status] ?? status
}

export function inviteTitle(invite: ProjectInvitePreview) {
  return invite.display_name || invite.project_name || '项目邀请'
}

export function formatDateTime(value?: string | null) {
  if (!value) return '无限期'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return date.toLocaleString()
}

export function memberRoleSummary(member: ProjectMember) {
  const roles = member.roles ?? []
  if (roles.length) return roles.map((role) => role.name || role.id).join(' / ')
  return roleLabel(member.role ?? 'member')
}

export function roleLabel(role: string) {
  const labels: Record<string, string> = {
    owner: '拥有者',
    admin: '管理员',
    editor: '协作者',
    developer: '开发者',
    maintainer: '维护者',
    member: '成员',
    observer: '只读成员',
  }
  return labels[role] ?? role
}

export function memberModerationSummary(member: ProjectMember) {
  if (member.is_banned) return `已封禁${member.banned_until ? `至 ${formatDateTime(member.banned_until)}` : ''}`
  if (member.is_muted) return `禁言至 ${formatDateTime(member.muted_until)}`
  return `${memberRoleSummary(member)} · ${presenceLabel(memberPresenceStatus(member))}`
}

export function numberOrUndefined(value: string) {
  const trimmed = clean(value)
  if (!trimmed) return undefined
  const parsed = Number(trimmed)
  return Number.isFinite(parsed) && parsed > 0 ? Math.floor(parsed) : undefined
}

export function inviteUrl(code: string) {
  const url = new URL('/pc', window.location.origin)
  url.searchParams.set('invite', code)
  return url.toString()
}
