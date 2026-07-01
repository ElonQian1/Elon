import type { RuntimeRoute } from './runtimeRoutes'

export interface Project {
  id: string
  name: string
  description?: string
  template?: string
  source_type?: string
  workspace_key?: string
  workspace_path?: string | null
  node_id?: string | null
  storage_worktree_path?: string | null
  runtime_permission?: string
  created_at?: string
  updated_at?: string
  icon?: string           // 已废弃字段，保留兼容
  icon_data_url?: string  // 服务端实际字段
  member_count?: number
  unread_count?: number
}

export interface Channel {
  id: string
  name: string
  kind?: string          // 'ai_development' | 'chat' | 'announce' | ...
  description?: string
  category_id?: string | null
  category_name?: string | null
  category_kind?: string | null
  category_position?: number
  permission_sync?: boolean
  permissions?: ChannelPermissions
  unread_count?: number
  last_message_at?: string
}

export interface ChannelCategory {
  id: string
  name: string
  kind?: string
  position?: number
}

export interface ChannelPermissions {
  can_view?: boolean
  canView?: boolean
  can_send?: boolean
  canSend?: boolean
  can_start_ai?: boolean
  canStartAi?: boolean
  can_manage?: boolean
  canManage?: boolean
}

export interface ProjectRoleRef {
  id: string
  name: string
  color?: string | null
  position?: number
  builtin?: boolean
}

export interface ProjectMember {
  user_id: string
  account?: string
  avatar_data_url?: string | null
  role?: string
  roles?: ProjectRoleRef[]
  joined_at?: string
  is_online?: boolean
  presence_status?: 'online' | 'idle' | 'dnd' | 'invisible' | 'offline' | string
  custom_status?: string | null
  activity?: string | null
  muted_until?: string | null
  banned_at?: string | null
  banned_until?: string | null
  is_muted?: boolean
  is_banned?: boolean
  channel_permissions?: Record<string, ChannelPermissions>
  channelPermissions?: Record<string, ChannelPermissions>
}

export interface ProjectMemberAuditEntry {
  id: string
  project_id: string
  actor_user_id?: string | null
  actor_account?: string | null
  target_user_id?: string | null
  target_account?: string | null
  action: string
  old_role?: string | null
  new_role?: string | null
  note?: string | null
  created_at: string
}

export interface ProjectMemberAuditResponse {
  entries?: ProjectMemberAuditEntry[]
  total?: number
  project_id?: string
}

export interface UserPresenceSettings {
  user_id: string
  status: 'online' | 'idle' | 'dnd' | 'invisible' | string
  custom_status?: string | null
  activity?: string | null
  updated_at?: string
}

export interface ProjectInviteLink {
  id: string
  project_id: string
  code: string
  role: string
  max_uses?: number | null
  use_count: number
  expires_at?: string | null
  temporary?: boolean
  revoked_at?: string | null
  created_by?: string
  created_at?: string
  updated_at?: string
}

export interface ProjectInvitePreview {
  project_id: string
  project_name: string
  display_name?: string | null
  role: string
  max_uses?: number | null
  use_count?: number
  expires_at?: string | null
  temporary?: boolean
}

export interface ProjectRole {
  id: string
  name: string
  color?: string | null
  position?: number
  permissions?: string[]
  builtin?: boolean
  member_count?: number
}

export interface PermissionOption {
  key: string
  label: string
}

export interface PermissionOverride {
  role_id?: string
  roleId?: string
  user_id?: string
  userId?: string
  allow?: string[]
  deny?: string[]
}

export interface ChannelPermissionResponse {
  permissions?: PermissionOption[]
  overrides?: PermissionOverride[]
  member_overrides?: PermissionOverride[]
  memberOverrides?: PermissionOverride[]
}

export interface ProjectLandingDownload {
  platform?: string
  label?: string
  short?: string
  url?: string
  version?: string
  size?: string
  status?: string
  note?: string
  variants?: Array<{
    label?: string
    arch?: string
    url?: string
    version?: string
    size?: string
    status?: string
    note?: string
  }>
}

export interface ProjectLandingResource {
  label?: string
  url?: string
}

export interface ProjectLanding {
  title?: string
  tagline?: string
  summary?: string
  description?: string
  highlights?: string[]
  target_users?: string[]
  downloads?: ProjectLandingDownload[]
  resources?: ProjectLandingResource[]
  custom_landing_url?: string
  web_url?: string
  source?: { mode?: string; status?: string }
}

export interface ProjectSpace {
  project?: Project
  categories?: ChannelCategory[]
  channels?: Channel[]
  members?: ProjectMember[]
  my_role?: string
  landing?: ProjectLanding
}

export interface Message {
  id: string
  kind?: string          // 'user' | 'ai_task' | 'ai_progress' | 'ai_result' | ...
  role?: string
  content?: string
  text?: string
  created_at?: string
  user_id?: string
  task_id?: string
  taskId?: string
  task_status?: string
  taskStatus?: string
  task_error?: string
  taskError?: string
  task_apk_url?: string
  taskApkUrl?: string
  [key: string]: unknown
}

export interface SendMessagePayload {
  content: string
  agent?: string | null
  runtimeRoute?: RuntimeRoute
}

export interface StartAiTaskPayload {
  content: string
  agent?: string | null
  runtimeRoute?: RuntimeRoute
}

export interface ProjectListResponse {
  projects?: Project[]
}

export interface ChannelMessagesResponse {
  messages?: Message[]
}

export interface ProjectRolesResponse {
  roles?: ProjectRole[]
  permissions?: PermissionOption[]
}

export interface ProjectInviteLinksResponse {
  invites?: ProjectInviteLink[]
}

export interface ProjectInviteResponse {
  invite?: ProjectInviteLink
}

export interface ProjectInvitePreviewResponse {
  invite?: ProjectInvitePreview
}
