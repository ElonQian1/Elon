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
}
