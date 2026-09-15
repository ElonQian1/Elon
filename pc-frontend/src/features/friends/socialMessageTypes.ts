export interface SocialAttachment {
  attachment_id?: string | null
  kind?: string | null
  display_name?: string | null
  file_name?: string | null
  mime_type?: string | null
  url?: string | null
  size_bytes?: number | null
  duration_seconds?: number | null
  transcription?: string | null
}

export interface SocialMessage {
  id: string
  sender_user_id: string
  sender_name?: string
  content: string
  attachments?: SocialAttachment[] | null
  created_at: string
  outgoing: boolean
  recalled_at?: string | null
  recalled_by?: string | null
  recalledAt?: string | null
  recalledBy?: string | null
  revision?: number
  edited_at?: string | null
}

export interface Friend {
  id: string
  account: string
  nickname?: string
  avatar_data_url?: string
  last_message?: string
  last_message_at?: string
  unread_count?: number
  is_online?: boolean
  presence_status?: string | null
  custom_status?: string | null
  activity?: string | null
}

export interface FriendGroup {
  id: string
  name: string
  member_count?: number
  members?: { id: string; display_name: string; avatar_data_url?: string }[]
  created_at?: string
  last_message?: string
  last_message_at?: string
  unread_count?: number
}

export interface ActiveConversation { kind: 'friend' | 'group'; id: string }
