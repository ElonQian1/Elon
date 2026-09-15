import { useCallback, useEffect, useRef, useState, type SetStateAction } from 'react'
import { api, type ApiError } from '../../api/client'
import { cloudBaseUrl, isLocalWorkbench } from '../../api/runtime'
import { useAuthStore } from '../../store/auth'
import { mergeGroupMessages, revisionOf } from './groupMessageRevisions'
import { conversationId, readSocialCache, socialCacheKey, writeSocialCache, type SocialSnapshot } from './socialChatCache'
import { startSocialRefresh } from './socialRefreshLoop'
import type { ActiveConversation, Friend, FriendGroup, SocialMessage } from './socialMessageTypes'

type Status = 'loading' | 'ready' | 'error'
const apply = <T,>(value: SetStateAction<T>, previous: T): T => typeof value === 'function' ? (value as (old: T) => T)(previous) : value
const failed = (signal: AbortSignal) => !signal.aborted || signal.reason?.name === 'TimeoutError'
const denied = (error: unknown) => [401, 403, 404].includes((error as ApiError)?.status)
const foreground = () => !document.hidden && document.hasFocus()

async function read<T>(path: string, signal: AbortSignal): Promise<T> {
  let abort: () => void = () => {}
  const cancelled = new Promise<never>((_, reject) => {
    abort = () => reject(signal.reason)
    if (signal.aborted) abort()
    else signal.addEventListener('abort', abort, { once: true })
  })
  try { return await Promise.race([api.get<T>(path, { signal, cache: 'no-store' }), cancelled]) }
  finally { signal.removeEventListener('abort', abort) }
}

export default function useSocialChat(userId: string) {
  const token = useAuthStore(s => s.token)
  const cacheKey = socialCacheKey(isLocalWorkbench() ? cloudBaseUrl() : location.origin, userId)
  const [snapshot, setSnapshot] = useState(() => readSocialCache(cacheKey))
  const current = useRef(snapshot)
  const [cacheWarning, setCacheWarning] = useState('')
  const [listStatus, setListStatus] = useState<Record<'friends' | 'groups', Status>>({ friends: 'loading', groups: 'loading' })
  const [messageStatus, setMessageStatus] = useState<Status>('loading')
  const [messageError, setMessageError] = useState('')
  const [revisionNotice, setRevisionNotice] = useState('')
  const listLoops = useRef<ReturnType<typeof startSocialRefresh>[]>([])
  const messageLoop = useRef<ReturnType<typeof startSocialRefresh> | null>(null)

  const commit = useCallback((update: (old: SocialSnapshot) => SocialSnapshot) => {
    // A late request or send receipt must not recreate caches after logout.
    const auth = useAuthStore.getState()
    if (auth.user?.id !== userId || !auth.token) return
    const next = update(current.current)
    current.current = next
    setSnapshot(next)
    setCacheWarning(writeSocialCache(cacheKey, next) ? '' : '本地缓存暂不可用，当前消息仍可使用')
  }, [cacheKey, userId])

  const activeConversation = snapshot.active
  const key = activeConversation ? conversationId(activeConversation) : ''
  const messages = snapshot.messages[key] ?? []
  const input = snapshot.drafts[key] ?? ''
  const setMessages = useCallback((value: SetStateAction<SocialMessage[]>) => {
    if (key) commit(old => ({ ...old, messages: { ...old.messages, [key]: apply(value, old.messages[key] ?? []) } }))
  }, [commit, key])
  const setInput = useCallback((value: SetStateAction<string>) => {
    if (key) commit(old => ({ ...old, drafts: { ...old.drafts, [key]: apply(value, old.drafts[key] ?? '') } }))
  }, [commit, key])
  const setFriends = useCallback((value: SetStateAction<Friend[]>) => {
    commit(old => ({ ...old, friends: apply(value, old.friends ?? []) }))
  }, [commit])
  const selectConversation = useCallback((item: ActiveConversation) => {
    commit(old => ({ ...old, active: { kind: item.kind, id: item.id } }))
  }, [commit])
  const loadSocialConversations = useCallback(() => { listLoops.current.forEach(loop => void loop.refresh()) }, [])
  const retry = useCallback(() => {
    loadSocialConversations()
    void messageLoop.current?.refresh()
  }, [loadSocialConversations])

  useEffect(() => {
    if (!token || !userId) return
    listLoops.current = (['friends', 'groups'] as const).map(field => startSocialRefresh(async signal => {
      try {
        const data = await read<{ friends?: Friend[]; groups?: FriendGroup[] }>(`/api/me/${field}`, signal)
        if (signal.aborted) return
        if (!Array.isArray(data[field])) throw new Error('无效的会话列表')
        const rows = data[field]!
        commit(old => {
          const kind = field === 'friends' ? 'friend' : 'group'
          const allowed = new Set(rows.map(row => `${kind}:${row.id}`))
          const keep = (id: string) => !id.startsWith(`${kind}:`) || allowed.has(id)
          let active = old.active && !keep(conversationId(old.active)) ? null : old.active
          if (!active && rows.length) {
            const latest = [...rows].sort((a, b) => (b.last_message_at ?? '').localeCompare(a.last_message_at ?? ''))[0]
            active = { kind, id: latest.id }
          }
          return { ...old, [field]: rows, active, messages: Object.fromEntries(Object.entries(old.messages).filter(([id]) => keep(id))), drafts: Object.fromEntries(Object.entries(old.drafts).filter(([id]) => keep(id))) }
        })
        setListStatus(old => ({ ...old, [field]: 'ready' }))
      } catch (error) {
        if (!failed(signal)) return
        if (denied(error)) commit(old => {
          const kind = field === 'friends' ? 'friend:' : 'group:'
          return { ...old, [field]: undefined, active: old.active && conversationId(old.active).startsWith(kind) ? null : old.active,
            messages: Object.fromEntries(Object.entries(old.messages).filter(([id]) => !id.startsWith(kind))),
            drafts: Object.fromEntries(Object.entries(old.drafts).filter(([id]) => !id.startsWith(kind))) }
        })
        setListStatus(old => ({ ...old, [field]: 'error' }))
      }
    }, { interval: 15000 }))
    return () => { listLoops.current.forEach(loop => loop.stop()); listLoops.current = [] }
  }, [commit, token, userId])

  useEffect(() => {
    setMessageError('')
    setRevisionNotice('')
    setMessageStatus('loading')
    if (!key || !token) return
    const conversation = current.current.active!
    const endpoint = `/api/me/${conversation.kind === 'friend' ? 'friends' : 'groups'}/${encodeURIComponent(conversation.id)}/messages?limit=120`
    const loop = startSocialRefresh(async signal => {
      try {
        const data = await read<{ messages?: SocialMessage[] }>(`${endpoint}&preserve_unread=${!foreground()}`, signal)
        if (signal.aborted) return
        if (!Array.isArray(data.messages)) throw new Error('无效的消息列表')
        const incoming = data.messages
        const prior = current.current.messages[key] ?? []
        const byId = new Map(prior.map(m => [m.id, m]))
        if (incoming.some(m => byId.has(m.id) && revisionOf(m) > revisionOf(byId.get(m.id)!))) setRevisionNotice('群聊文字已更新，可点击“已编辑”查看修改记录')
        commit(old => ({ ...old, messages: { ...old.messages, [key]: mergeGroupMessages(old.messages[key] ?? [], incoming) } }))
        setMessageError('')
        setMessageStatus('ready')
        // The foreground read marks this conversation read; reflect that locally.
        if (foreground()) commit(old => ({ ...old, [conversation.kind === 'friend' ? 'friends' : 'groups']:
          (conversation.kind === 'friend' ? old.friends : old.groups)?.map(row => row.id === conversation.id ? { ...row, unread_count: 0 } : row) }))
      } catch (error) {
        if (!failed(signal)) return
        if (denied(error)) commit(old => ({ ...old, messages: { ...old.messages, [key]: [] } }))
        setMessageStatus('error')
        setMessageError(denied(error) ? '无法访问此会话，请检查登录状态或成员权限' : '消息同步失败，已保留现有内容；将自动重试')
      }
    })
    messageLoop.current = loop
    return () => { loop.stop(); if (messageLoop.current === loop) messageLoop.current = null }
  }, [commit, key, token])

  return { friends: snapshot.friends ?? [], groups: snapshot.groups ?? [], setFriends, activeConversation,
    messages, setMessages, input, setInput, selectConversation, loadSocialConversations, retry,
    listStatus, messageError, cacheWarning, revisionNotice,
    messagesLoading: !!key && messageStatus === 'loading' && messages.length === 0,
  }
}
