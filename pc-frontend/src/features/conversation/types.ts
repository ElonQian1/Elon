export interface Project {
  id: string
  name: string
  description?: string
  template?: string
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
  is_muted?: boolean
  is_banned?: boolean
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

export interface ProjectSpace {
  project?: Project
  categories?: ChannelCategory[]
  channels?: Channel[]
  members?: ProjectMember[]
  my_role?: string
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
}

export interface StartAiTaskPayload {
  content: string
  agent?: string | null
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
