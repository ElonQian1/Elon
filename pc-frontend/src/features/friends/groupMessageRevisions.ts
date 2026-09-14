import type { SocialMessage } from './socialMessageTypes'

export interface MessageRevision {
  revision: number
  content: string
  edited_by: string
  created_at: string
}
export interface MessageHistory {
  message_id: string
  current_revision: number
  revisions: MessageRevision[]
  next_before_revision: number | null
}
export type MessageEdit = Pick<SocialMessage, 'id' | 'content' | 'revision' | 'edited_at'>

export const revisionOf = (message: SocialMessage) => message.revision ?? 1
export const revisionsPath = (group: string, message: string) =>
  `/api/me/groups/${encodeURIComponent(group)}/messages/${encodeURIComponent(message)}`

// A bounded, Unicode-safe changed span. Full versions remain visible even for large edits.
export function changedText(before: string, after: string) {
  const a = Array.from(before), b = Array.from(after)
  let start = 0, end = 0
  while (start < a.length && start < b.length && a[start] === b[start]) start++
  while (end < a.length - start && end < b.length - start && a[a.length - end - 1] === b[b.length - end - 1]) end++
  return { removed: a.slice(start, a.length - end).join(''), added: b.slice(start, b.length - end).join('') }
}

export function mergeGroupMessages(previous: SocialMessage[], incoming: SocialMessage[]) {
  const old = new Map(previous.map(message => [message.id, message]))
  const merged = incoming.map(message => {
    const prior = old.get(message.id)
    // A poll started before a successful save must not roll the message back.
    if (prior && !message.recalled_at && (prior.recalled_at || revisionOf(prior) > revisionOf(message))) return prior
    return message
  })
  return [...merged, ...previous.filter(message => message.id.startsWith('tmp-') && !incoming.some(m => m.id === message.id))]
}
