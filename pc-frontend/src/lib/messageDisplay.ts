import { clean } from './utils'

const INTERNAL_ATTACHMENT_CONTEXT_MARKER =
  'User uploaded real chat attachments for this project conversation'

const INTERNAL_ATTACHMENT_CONTEXT_PATTERN =
  /(?:\r?\n){0,2}\s*User uploaded real chat attachments for this project conversation\s*(?:\([^)]*\))?:[\s\S]*?(?:These attachments are part of the current message context, like images\/files in a normal chat app\.[\s\S]*?before answering\.|$)/g

const UPLOADED_ATTACHMENT_FALLBACK = '\u5df2\u4e0a\u4f20\u9644\u4ef6'

export function stripInternalAttachmentContext(value: unknown): string {
  const text = clean(value)
  if (!text.includes(INTERNAL_ATTACHMENT_CONTEXT_MARKER)) return text
  return text
    .replace(INTERNAL_ATTACHMENT_CONTEXT_PATTERN, '')
    .replace(/[ \t]+\n/g, '\n')
    .replace(/\n{3,}/g, '\n\n')
    .trim()
}

export function displayMessageContent(value: unknown, emptyFallback = ''): string {
  const text = stripInternalAttachmentContext(value)
  return text || emptyFallback
}

export function displayMessageContentOrAttachment(value: unknown): string {
  const text = stripInternalAttachmentContext(value)
  if (text) return text
  return clean(value).includes(INTERNAL_ATTACHMENT_CONTEXT_MARKER)
    ? UPLOADED_ATTACHMENT_FALLBACK
    : ''
}

export function compactDisplayMessageContent(value: unknown, maxLength = 28): string {
  const normalized = stripInternalAttachmentContext(value).replace(/\s+/g, ' ').trim()
  return normalized.length > maxLength ? `${normalized.slice(0, maxLength)}...` : normalized
}
