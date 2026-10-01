import type { SocialAttachment, SocialMessage } from '../socialMessageTypes'
import { gridCard } from '../../grid-share/gridShareModel'

export interface SocialQuote {
  message_id: string
  sender_name: string
  content: string
  attachments: SocialAttachment[]
  revision: number
  unavailable: boolean
}
export interface QuoteSource { message_id: string; revision: number }

/** Only historical app quote prefixes; normal Markdown elsewhere remains untouched. */
export function splitSocialQuote(message: Pick<SocialMessage, 'content' | 'quote'>): { body: string; quote: SocialQuote | null } {
  if (message.quote) return { body: message.content, quote: message.quote }
  const match = /^(>[^\n]*(?:\r?\n>[^\n]*)*)\r?\n(?:[ \t]*\r?\n)?([^>][\s\S]*)$/.exec(message.content)
  if (!match) return { body: message.content, quote: null }
  const lines = match[1].split(/\r?\n/).map(line => line.replace(/^(?:>\s*)+/, ''))
  const header = /^引用 (.+?)(?: · 第 (\d+) 版)?$/.exec(lines[0])
  if (header) lines.shift()
  return { body: match[2], quote: { message_id: '', sender_name: header?.[1] || '',
    content: lines.join('\n'), attachments: [], revision: Number(header?.[2] || 1), unavailable: false } }
}

export function quoteFromMessage(message: SocialMessage, author: string): SocialQuote {
  return { message_id: message.id, sender_name: author, content: splitSocialQuote(message).body,
    attachments: message.attachments || [], revision: message.revision || 1, unavailable: !!(message.recalled_at || message.recalledAt) }
}

export function quoteSummary(quote: SocialQuote): string {
  if (quote.unavailable) return '原消息已撤回或不可用'
  let text = splitSocialQuote({ content: quote.content }).body.trim()
  const grid = gridCard(text)
  if (grid) return `[网格快照] ${grid.title} · ${new Date(grid.grid.observed_at_ms).toLocaleString()}`
  if (/^【一龙(?:聊天记录|.*卡片)】/.test(text)) {
    const start = text.indexOf('{')
    try { const card = JSON.parse(text.slice(start)); text = `[聊天记录] ${card.title || card.name || '聊天记录'}` } catch { text = '[聊天记录]' }
  }
  const attachment = quote.attachments[0]
  return text || (attachment ? `[${attachment.kind === 'image' ? '图片' : attachment.kind === 'audio' ? '语音' : attachment.kind === 'video' ? '视频' : '文件'}] ${attachment.display_name || attachment.file_name || ''}` : '[消息]')
}
