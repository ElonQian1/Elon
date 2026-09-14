import { useEffect, useRef, useState, type Dispatch, type SetStateAction } from 'react'
import { api } from '../../api/client'
import { mergeGroupMessages, revisionOf } from './groupMessageRevisions'
import type { SocialMessage } from './socialMessageTypes'

export default function useGroupMessageRefresh(groupId: string | null, messages: SocialMessage[], setMessages: Dispatch<SetStateAction<SocialMessage[]>>) {
  const current = useRef(messages)
  current.current = messages
  const [notice, setNotice] = useState('')
  useEffect(() => {
    setNotice('')
    if (!groupId) return
    let stopped = false
    let timer: ReturnType<typeof setTimeout>
    async function refresh() {
      try {
        const data = await api.get<{ messages?: SocialMessage[] }>(`/api/me/groups/${encodeURIComponent(groupId!)}/messages?limit=120&preserve_unread=true`)
        if (stopped) return
        const incoming = data.messages ?? []
        const old = new Map(current.current.map(message => [message.id, message]))
        if (incoming.some(message => old.has(message.id) && revisionOf(message) > revisionOf(old.get(message.id)!))) setNotice('群聊文字已更新，可点击“已编辑”查看修改记录')
        setMessages(previous => mergeGroupMessages(previous, incoming))
      } catch {
        // Keep displayed messages and drafts; the next poll retries without a notification sound.
      } finally {
        if (!stopped) timer = setTimeout(refresh, 4000)
      }
    }
    timer = setTimeout(refresh, 4000)
    return () => { stopped = true; clearTimeout(timer) }
  }, [groupId, setMessages])
  return notice
}
