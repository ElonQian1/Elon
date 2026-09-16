import type { SocialAttachment } from './socialMessageTypes'
import { cloudResourceUrl } from '../../lib/cloudResourceUrl'

export interface SocialMenuRequest {
  id: string; x: number; y: number; nonce: number
  origin: HTMLElement; selection: string; media: HTMLElement | null; link: HTMLAnchorElement | null
}

export function messageMenuRequest(id: string, origin: HTMLElement, x: number, y: number, target: HTMLElement): SocialMenuRequest {
  const body = origin.closest('[data-message-id]')?.querySelector('[data-social-content]')
  const selection = window.getSelection()
  // Copy only a selection wholly inside the message that was invoked.
  const selected = selection && !selection.isCollapsed && body?.contains(selection.anchorNode) && body.contains(selection.focusNode)
    ? selection.toString() : ''
  const directLink = target.closest<HTMLAnchorElement>('a.social-link-card[href]')
  const links = body?.querySelectorAll<HTMLAnchorElement>('a.social-link-card[href]')
  // Keyboard/ellipsis on a message with one card targets that card, too.
  const link = directLink || (links?.length === 1 ? links[0] : null)
  return { id, x, y, nonce: performance.now(), origin, selection: selected, link,
    media: target.closest<HTMLElement>('[data-social-attachment]') }
}

export function attachmentKind(attachment: SocialAttachment): 'image' | 'audio' | 'file' {
  const kind = attachment.kind?.toLowerCase(), mime = attachment.mime_type?.toLowerCase() || ''
  const extension = (attachment.file_name || attachment.url?.split('?')[0] || '').toLowerCase()
  if (kind === 'image' || mime.startsWith('image/') || /\.(png|jpe?g|gif|webp|bmp|avif)$/.test(extension)) return 'image'
  if (kind === 'audio' || kind === 'voice' || mime.startsWith('audio/') || /\.(mp3|m4a|aac|ogg|opus|wav|amr)$/.test(extension)) return 'audio'
  return 'file'
}

async function mediaBlob(attachment: SocialAttachment) {
  const url = cloudResourceUrl(attachment.url)
  if (!url) throw new Error('附件地址不可用')
  const controller = new AbortController(), timer = setTimeout(() => controller.abort(), 15000)
  try {
    const response = await fetch(url, { signal: controller.signal })
    if (!response.ok) throw new Error('附件读取失败，请稍后重试')
    return await response.blob()
  } finally { clearTimeout(timer) }
}

export async function downloadAttachment(attachment: SocialAttachment) {
  const blob = await mediaBlob(attachment), url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url; link.download = attachment.display_name || attachment.file_name || '聊天附件'
  document.body.append(link); link.click(); link.remove()
  setTimeout(() => URL.revokeObjectURL(url), 60000)
}

export async function copyImageAttachment(attachment: SocialAttachment) {
  if (!navigator.clipboard?.write || typeof ClipboardItem === 'undefined') throw new Error('当前环境不支持复制图片，请使用下载图片')
  // Chromium clipboard accepts PNG. Decode other image formats before writing.
  const png = mediaBlob(attachment).then(async blob => {
    if (blob.type === 'image/png') return blob
    const bitmap = await createImageBitmap(blob)
    try {
      const canvas = document.createElement('canvas'); canvas.width = bitmap.width; canvas.height = bitmap.height
      const context = canvas.getContext('2d'); if (!context) throw new Error('图片转换失败')
      context.drawImage(bitmap, 0, 0)
      return await new Promise<Blob>((resolve, reject) => canvas.toBlob(value => value ? resolve(value) : reject(new Error('图片转换失败')), 'image/png'))
    } finally { bitmap.close() }
  })
  await navigator.clipboard.write([new ClipboardItem({ 'image/png': png })])
}
