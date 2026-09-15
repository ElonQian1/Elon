import { useEffect, useRef, useState } from 'react'
import { socialLocalId } from './socialLocalId'
import type { ActiveConversation, SocialAttachment } from './socialMessageTypes'
import { conversationId } from './socialChatCache'
import { socialRequest } from './socialChatOperations'

export interface PendingSocialFile { id: string; file: File; status: 'uploading' | 'ready' | 'error'; attachment?: SocialAttachment; error?: string }
export function useSocialAttachments(userId: string) {
  const [byConversation, setByConversation] = useState<Record<string, PendingSocialFile[]>>({})
  const current = useRef(byConversation)
  const controllers = useRef(new Map<string, AbortController>())
  const mounted = useRef(true)
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; controllers.current.forEach(c => c.abort()) } }, [])
  const update = (key: string, change: (files: PendingSocialFile[]) => PendingSocialFile[]) => {
    if (!mounted.current) return
    current.current = { ...current.current, [key]: change(current.current[key] ?? []) }
    setByConversation(current.current)
  }
  async function upload(conversation: ActiveConversation, item: PendingSocialFile) {
    const key = conversationId(conversation), controller = new AbortController()
    controllers.current.set(item.id, controller)
    update(key, files => files.map(f => f.id === item.id ? { ...f, status: 'uploading', error: '' } : f))
    try {
      const mime = item.file.type || 'application/octet-stream'
      const params = new URLSearchParams({ file_name: item.file.name, display_name: item.file.name, mime_type: mime,
        kind: mime.startsWith('image/') ? 'image' : mime.startsWith('audio/') ? 'audio' : 'attachment', conversation_id: `${conversation.kind}-${conversation.id}` })
      const result = await socialRequest<{ attachment: SocialAttachment }>(`/api/user/${encodeURIComponent(userId)}/chat-attachments?${params}`, {
        method: 'POST', body: item.file, headers: { 'Content-Type': mime }, signal: controller.signal,
      }, 60000)
      if (!result.attachment?.attachment_id || !result.attachment.url) throw new Error('上传响应缺少附件信息，请重试')
      update(key, files => files.map(f => f.id === item.id ? { ...f, status: 'ready', attachment: result.attachment } : f))
    } catch (failure) {
      if (!controller.signal.aborted) update(key, files => files.map(f => f.id === item.id ? { ...f, status: 'error', error: (failure as Error).message } : f))
    } finally { controllers.current.delete(item.id) }
  }
  function add(conversation: ActiveConversation, files: File[]): string {
    const key = conversationId(conversation)
    if ((current.current[key]?.length ?? 0) + files.length > 6) return '每条消息最多添加 6 个附件'
    if (files.some(file => file.size > 12 * 1024 * 1024)) return '每个附件不能超过 12 MB'
    const items: PendingSocialFile[] = files.map(file => ({ id: socialLocalId(), file, status: 'uploading' }))
    update(key, previous => [...previous, ...items])
    items.forEach(item => void upload(conversation, item))
    return ''
  }
  function remove(conversation: ActiveConversation, ids: string[]) {
    ids.forEach(id => controllers.current.get(id)?.abort())
    update(conversationId(conversation), files => files.filter(f => !ids.includes(f.id)))
  }
  return { byConversation, add, remove, retry: upload }
}
