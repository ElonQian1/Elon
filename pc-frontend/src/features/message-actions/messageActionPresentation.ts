export const messageActionsHostClassName = 'message-actions-host'

export function messageCopySourceId(storageScope: string, messageKey: string): string {
  return `message-copy-${normalizeMessageActionStorageSegment(storageScope)}-${normalizeMessageActionStorageSegment(messageKey)}`
}

export function normalizeMessageActionStorageSegment(value: string): string {
  return value
    .trim()
    .replace(/[^a-zA-Z0-9._:-]+/g, '_')
    .slice(0, 120) || 'message'
}
