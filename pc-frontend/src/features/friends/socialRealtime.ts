export const SOCIAL_REFRESH_EVENT = 'elon:social-refresh'
const messageTypes = new Set(['friend_message', 'group_message', 'group_message_edited'])

/** Invalidate from push; fetch the authoritative message including media and recall state. */
export function dispatchSocialMessage(data: Record<string, unknown>): boolean {
  if (!messageTypes.has(String(data.type))) return false
  window.dispatchEvent(new Event(SOCIAL_REFRESH_EVENT))
  return true
}

export function notifySocialReconnect() {
  window.dispatchEvent(new Event(SOCIAL_REFRESH_EVENT))
}
