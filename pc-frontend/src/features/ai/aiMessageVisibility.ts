const INVISIBLE_MESSAGE_PLACEHOLDERS = /[\u00ad\u034f\u061c\u180e\u200b-\u200f\u202a-\u202e\u2060-\u206f\ufeff]/gu

export function hasVisibleAiMessageContent(value: string | null | undefined): boolean {
  return String(value ?? '').replace(INVISIBLE_MESSAGE_PLACEHOLDERS, '').trim().length > 0
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
