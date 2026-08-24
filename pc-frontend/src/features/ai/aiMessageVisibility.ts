const INVISIBLE_MESSAGE_PLACEHOLDERS = /[\u00ad\u034f\u061c\u180e\u200b-\u200f\u202a-\u202e\u2060-\u206f\u2022\u2026\u22ef\u25cf\u25cb\u2580-\u259f\ue000-\uf8ff\ufeff]/gu
const SPEAKER_SHELL_ONLY = /^(?:#{1,6}\s*)?(?:chatgpt(?: said| 说)|assistant said|助理说|you said|你说|您说)\s*[:：]?\s*$/i

export function hasVisibleAiMessageContent(value: string | null | undefined): boolean {
  const visible = String(value ?? '').replace(INVISIBLE_MESSAGE_PLACEHOLDERS, '').trim()
  return visible.length > 0 && !SPEAKER_SHELL_ONLY.test(visible)
}

export function shouldKeepAiWebMessage({
  content,
  state,
  sourceCount = 0,
  structuredCount = 0,
}: {
  content: string | null | undefined
  state: 'streaming' | 'completed'
  sourceCount?: number
  structuredCount?: number
}): boolean {
  return state === 'streaming'
    || hasVisibleAiMessageContent(content)
    || sourceCount > 0
    || structuredCount > 0
}
