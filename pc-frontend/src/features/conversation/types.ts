export interface Project {
  id: string
  name: string
  description?: string
  template?: string
  created_at?: string
  updated_at?: string
  icon?: string
  member_count?: number
  unread_count?: number
}

export interface Channel {
  id: string
  name: string
  kind?: string          // 'ai_development' | 'chat' | 'announce' | ...
  description?: string
  unread_count?: number
  last_message_at?: string
}

export interface ProjectSpace {
  project?: Project
  channels?: Channel[]
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
