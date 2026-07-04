import { clean } from '../../lib/utils'
import type { Message } from './types'

const FINGERPRINT_FIELDS = [
  'id',
  'kind',
  'role',
  'message_kind',
  'task_id',
  'taskId',
  'task_status',
  'taskStatus',
  'task_error',
  'taskError',
  'task_apk_url',
  'taskApkUrl',
  'conversation_id',
  'conversationId',
  'content',
  'text',
  'created_at',
  'updated_at',
  'sender_name',
  'senderName',
  'sender_avatar_data_url',
  'senderAvatarDataUrl',
  'outgoing',
  'model_used',
  'node_id',
  'stream_id',
] as const

export function sameMessageList(left: Message[], right: Message[]): boolean {
  if (left === right) return true
  if (left.length !== right.length) return false
  for (let index = 0; index < left.length; index += 1) {
    if (messageFingerprint(left[index]) !== messageFingerprint(right[index])) return false
  }
  return true
}

export function messageFingerprint(message: Message | undefined): string {
  if (!message) return ''
  const source = message as Record<string, unknown>
  return FINGERPRINT_FIELDS.map((field) => normalizeFingerprintValue(source[field])).join('|')
}

function normalizeFingerprintValue(value: unknown): string {
  if (value === null || value === undefined) return ''
  if (typeof value === 'string') return clean(value)
  if (typeof value === 'number' || typeof value === 'boolean') return String(value)
  return JSON.stringify(value)
}
