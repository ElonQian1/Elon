import { useCallback, useRef, useState } from 'react'
import { cloudBaseUrl, isLocalWorkbench } from '../../api/runtime'
import { useAuthStore } from '../../store/auth'
import type { ActiveConversation, SocialMessage } from './socialMessageTypes'

const PREFIX = 'elon_social_tools_v1:'
export interface SavedSocialMessage { conversation: ActiveConversation; title: string; message: SocialMessage }
interface LocalState { favorites: SavedSocialMessage[]; hidden: string[] }
export const localMessageKey = (conversation: ActiveConversation, id: string) => `${conversation.kind}:${conversation.id}:${id}`
export function clearSocialLocalState() {
  try {
    Object.keys(localStorage).filter(key => key.startsWith(PREFIX)).forEach(key => localStorage.removeItem(key))
  } catch { /* Logout must remain available. */ }
}
function load(key: string): LocalState {
  try {
    const value = JSON.parse(localStorage.getItem(key) || '{}')
    return { favorites: Array.isArray(value.favorites) ? value.favorites.filter((v: SavedSocialMessage) => v?.message && typeof v.message.content === 'string' && typeof v.message.id === 'string' && v.conversation && ['friend','group'].includes(v.conversation.kind)).slice(0, 100) : [],
      hidden: Array.isArray(value.hidden) ? value.hidden.filter((v: unknown) => typeof v === 'string').slice(-2000) : [] }
  } catch { return { favorites: [], hidden: [] } }
}
export function useSocialLocalState(userId: string) {
  const key = `${PREFIX}${encodeURIComponent(isLocalWorkbench() ? cloudBaseUrl() : location.origin)}:${encodeURIComponent(userId)}`
  const [value, setValue] = useState(() => load(key))
  const [error, setError] = useState('')
  const model = useRef(value)
  const update = useCallback((change: (old: LocalState) => LocalState) => {
    if (useAuthStore.getState().user?.id !== userId) return
    const next = change(model.current)
    try {
      const text = JSON.stringify(next)
      if (text.length > 1_500_000) throw new Error('收藏已满，请先取消部分收藏')
      localStorage.setItem(key, text); model.current = next; setValue(next); setError('')
    } catch { setError('本机存储失败，操作未保存；请减少收藏或检查可用空间') }
  }, [key, userId])
  const save = (items: SavedSocialMessage[]) => update(old => {
    const incoming = new Set(items.map(v => localMessageKey(v.conversation, v.message.id)))
    return { ...old, favorites: [...items, ...old.favorites.filter(v => !incoming.has(localMessageKey(v.conversation, v.message.id)))].slice(0, 100) }
  })
  const remove = (id: string) => update(old => ({ ...old, favorites: old.favorites.filter(v => localMessageKey(v.conversation, v.message.id) !== id) }))
  const hide = (ids: string[]) => update(old => ({ ...old, hidden: [...new Set([...old.hidden, ...ids])].slice(-2000) }))
  const restore = (prefix: string) => update(old => ({ ...old, hidden: old.hidden.filter(id => !id.startsWith(prefix)) }))
  return { ...value, save, remove, hide, restore, error }
}
