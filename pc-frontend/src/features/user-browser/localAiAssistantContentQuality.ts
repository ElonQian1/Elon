import type { LocalAiVisibleMessage } from './localAiBrowserApi'

const ACTION_ONLY_TEXT = new Set([
  '提供反馈', '反馈', '复制', '复制回答', '重新生成', '重试', '点赞', '点踩', '分享',
  '朗读', '编辑', '更多', 'good response', 'bad response', 'provide feedback',
  'copy', 'copy response', 'regenerate', 'retry', 'share', 'read aloud', 'more',
])

export function isLocalAiActionOnlyText(value: string): boolean {
  const normalized = value.trim().toLowerCase().replace(/[：:。.!！?？]+$/g, '')
  if (!normalized) return false
  if (ACTION_ONLY_TEXT.has(normalized)) return true
  const tokens = normalized.split(/[\s·|/]+/).filter(Boolean)
  return tokens.length > 0 && tokens.length <= 8
    && tokens.every((token) => ACTION_ONLY_TEXT.has(token))
}

export function localAiAssistantExtractionIncomplete(message: LocalAiVisibleMessage): boolean {
  if (message.role !== 'assistant' || message.state !== 'completed') return false
  const text = message.content
    .filter((part) => part.type === 'text' || part.type === 'markdown')
    .map((part) => part.text)
    .filter(Boolean)
    .join(' ')
  const structuredCount = message.content.filter((part) => (
    !['text', 'markdown', 'citation'].includes(part.type)
  )).length
  return structuredCount > 0 && (!text.trim() || isLocalAiActionOnlyText(text))
}

export function localAiAssistantHasRendererPlaceholder(message: LocalAiVisibleMessage): boolean {
  return message.role === 'assistant' && message.content.some((part) => (
    part.type === 'interactive'
    && ['interactive', 'renderer_upgrade_required'].includes(String(part.kind || ''))
  ))
}
