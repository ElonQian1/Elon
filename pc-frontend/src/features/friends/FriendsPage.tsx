import { useEffect, useRef, useState, useCallback, useMemo } from 'react'
import { ChevronDown, ChevronRight, Layers, List } from 'lucide-react'
import { api } from '../../api/client'
import { useAuthStore } from '../../store/auth'
import { clean, formatTime } from '../../lib/utils'
import MarkdownContent from '../markdown/MarkdownContent'
import styles from './FriendsPage.module.css'

interface Friend {
  id: string
  account: string
  nickname?: string
  avatar_data_url?: string
  last_message?: string
  last_message_at?: string
  unread_count?: number
  is_online?: boolean
  presence_status?: string | null
  custom_status?: string | null
  activity?: string | null
}

interface FriendGroupMemberPreview {
  id: string
  display_name: string
  avatar_data_url?: string
}

interface FriendGroup {
  id: string
  name: string
  member_count?: number
  members?: FriendGroupMemberPreview[]
  created_at?: string
  last_message?: string
  last_message_at?: string
  unread_count?: number
}

interface SocialMessage {
  id: string
  sender_user_id: string
  sender_name?: string
  content: string
  created_at: string
  outgoing: boolean
}

interface SearchResult {
  user: Friend
  already_friend: boolean
  is_self: boolean
}

interface FriendSearchResponse {
  found?: boolean
  user?: Friend
  already_friend?: boolean
  is_self?: boolean
  results?: SearchResult[]
}

type ConversationKind = 'friend' | 'group'
type ConversationDisplayMode = 'grouped' | 'active'
type CollapsibleSection = 'friends' | 'groups'
type PresenceStatus = 'online' | 'idle' | 'dnd' | 'offline'

interface ActiveConversation {
  kind: ConversationKind
  id: string
}

interface ConversationItem {
  kind: ConversationKind
  id: string
  title: string
  subtitle: string
  lastMessage?: string
  lastMessageAt?: string
  unreadCount: number
  isOnline?: boolean
  presenceStatus?: PresenceStatus
  presenceSummary?: string
  friend?: Friend
  group?: FriendGroup
}

interface PresenceEvent extends CustomEvent {
  detail: {
    userId?: string
    isOnline?: boolean
    status?: string
    customStatus?: string | null
    custom_status?: string | null
    activity?: string | null
  }
}

export default function FriendsPage() {
  const me = useAuthStore((s) => s.user)
  const [friends, setFriends] = useState<Friend[]>([])
  const [groups, setGroups] = useState<FriendGroup[]>([])
  const [activeConversation, setActiveConversation] = useState<ActiveConversation | null>(null)
  const [messages, setMessages] = useState<SocialMessage[]>([])
  const [messagesLoading, setMessagesLoading] = useState(false)
  const [input, setInput] = useState('')
  const [sending, setSending] = useState(false)
  const [error, setError] = useState('')
  const [displayMode, setDisplayMode] = useState<ConversationDisplayMode>(() => readConversationDisplayMode())
  const [collapsedSections, setCollapsedSections] = useState<Record<CollapsibleSection, boolean>>(
    () => readCollapsedSections(),
  )

  // 搜索添加好友
  const [searchQ, setSearchQ] = useState('')
  const [searchResults, setSearchResults] = useState<SearchResult[]>([])
  const [searchLoading, setSearchLoading] = useState(false)
  const [addingId, setAddingId] = useState<string | null>(null)

  const feedRef = useRef<HTMLDivElement>(null)
  const textareaRef = useRef<HTMLTextAreaElement>(null)

  useEffect(() => { loadSocialConversations() }, [me?.id])

  useEffect(() => {
    function onPresence(event: PresenceEvent) {
      const detail = event.detail ?? {}
      const userId = clean(detail.userId)
      if (!userId) return
      setFriends((prev) => {
        let changed = false
        const next = prev.map((friend) => {
          if (friend.id !== userId) return friend
          changed = true
          return applyFriendPresencePatch(friend, detail)
        })
        return changed ? next : prev
      })
    }

    window.addEventListener('elon:presence', onPresence as EventListener)
    return () => window.removeEventListener('elon:presence', onPresence as EventListener)
  }, [])

  useEffect(() => {
    if (feedRef.current) feedRef.current.scrollTop = feedRef.current.scrollHeight
  }, [messages])

  useEffect(() => {
    writeLocalPreference(DISPLAY_MODE_STORAGE_KEY, displayMode)
  }, [displayMode])

  useEffect(() => {
    writeLocalPreference(COLLAPSED_SECTIONS_STORAGE_KEY, JSON.stringify(collapsedSections))
  }, [collapsedSections])

  const friendConversationItems = useMemo(() => {
    return sortConversationItems(friends.map((friend) => {
      const presence = friendPresence(friend)
      return {
        kind: 'friend',
        id: friend.id,
        title: friend.nickname ?? friend.account,
        subtitle: friendPreviewText(friend),
        lastMessage: friend.last_message,
        lastMessageAt: friend.last_message_at,
        unreadCount: friend.unread_count ?? 0,
        isOnline: presence.status !== 'offline',
        presenceStatus: presence.status,
        presenceSummary: presence.summary,
        friend,
      }
    }))
  }, [friends])

  const groupConversationItems = useMemo(() => {
    return sortConversationItems(groups.map((group) => ({
      kind: 'group',
      id: group.id,
      title: group.name || '群聊',
      subtitle: group.last_message || `${group.member_count ?? 0} 位成员`,
      lastMessage: group.last_message,
      lastMessageAt: group.last_message_at ?? group.created_at,
      unreadCount: group.unread_count ?? 0,
      group,
    })))
  }, [groups])

  const allConversationItems = useMemo(
    () => sortConversationItems([...friendConversationItems, ...groupConversationItems]),
    [friendConversationItems, groupConversationItems],
  )

  const activeItem = activeConversation
    ? allConversationItems.find((item) => item.kind === activeConversation.kind && item.id === activeConversation.id)
    : undefined

  async function loadSocialConversations() {
    const [friendResult, groupResult] = await Promise.allSettled([
      api.get<{ friends?: Friend[] }>('/api/me/friends'),
      api.get<{ groups?: FriendGroup[] }>('/api/me/groups'),
    ])
    if (friendResult.status === 'fulfilled') setFriends(friendResult.value.friends ?? [])
    if (groupResult.status === 'fulfilled') setGroups(groupResult.value.groups ?? [])
  }

  async function selectConversation(item: ConversationItem) {
    setActiveConversation({ kind: item.kind, id: item.id })
    setMessages([])
    setMessagesLoading(true)
    setError('')
    try {
      const endpoint = item.kind === 'friend'
        ? `/api/me/friends/${encodeURIComponent(item.id)}/messages?limit=80`
        : `/api/me/groups/${encodeURIComponent(item.id)}/messages?limit=120`
      const data = await api.get<{ messages?: SocialMessage[] }>(endpoint)
      setMessages(data.messages ?? [])
      void loadSocialConversations()
    } catch { /* ignore */ }
    finally { setMessagesLoading(false) }
  }

  function activeTitle() {
    return activeItem?.title ?? '会话'
  }

  const autoResize = useCallback(() => {
    const el = textareaRef.current
    if (!el) return
    el.style.height = '46px'
    el.style.height = Math.min(el.scrollHeight, 120) + 'px'
    el.style.overflowY = el.scrollHeight > 120 ? 'auto' : 'hidden'
  }, [])

  async function handleSend(e: React.FormEvent | React.KeyboardEvent) {
    e.preventDefault()
    const text = input.trim()
    if (!text || sending || !activeConversation) return
    setInput('')
    setError('')
    if (textareaRef.current) textareaRef.current.style.height = '46px'
    setSending(true)
    const current = activeConversation
    const optimistic: SocialMessage = {
      id: `tmp-${Date.now()}`,
      sender_user_id: me?.id ?? '',
      sender_name: me?.nickname ?? me?.account ?? '我',
      content: text,
      created_at: new Date().toISOString(),
      outgoing: true,
    }
    setMessages((prev) => [...prev, optimistic])
    try {
      const endpoint = current.kind === 'friend'
        ? `/api/me/friends/${encodeURIComponent(current.id)}/messages`
        : `/api/me/groups/${encodeURIComponent(current.id)}/messages`
      const res = await api.post<{ message?: SocialMessage }>(
        endpoint,
        { content: text },
      )
      if (res.message) {
        setMessages((prev) => prev.map((m) => m.id === optimistic.id ? res.message! : m))
      }
      void loadSocialConversations()
    } catch (err) {
      setError((err as { message?: string }).message ?? '发送失败')
      setMessages((prev) => prev.filter((m) => m.id !== optimistic.id))
    } finally {
      setSending(false)
    }
  }

  function handleKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSend(e)
    }
  }

  async function handleSearch(e: React.FormEvent) {
    e.preventDefault()
    const q = searchQ.trim()
    if (!q) return
    setSearchLoading(true)
    setSearchResults([])
    try {
      const data = await api.get<FriendSearchResponse>(
        `/api/me/friends/search?q=${encodeURIComponent(q)}`,
      )
      if (data.results) {
        setSearchResults(data.results)
      } else if (data.found && data.user) {
        setSearchResults([{
          user: data.user,
          already_friend: !!data.already_friend,
          is_self: !!data.is_self,
        }])
      } else {
        setSearchResults([])
      }
    } catch { /* ignore */ }
    finally { setSearchLoading(false) }
  }

  async function handleAddFriend(result: SearchResult) {
    setAddingId(result.user.id)
    try {
      await api.post('/api/me/friends', { query: result.user.id, search_type: 'user_id' })
      await loadSocialConversations()
      setSearchResults((prev) => prev.map((r) =>
        r.user.id === result.user.id ? { ...r, already_friend: true } : r,
      ))
    } catch (err) {
      alert((err as { message?: string }).message ?? '添加失败')
    } finally {
      setAddingId(null)
    }
  }

  function toggleSection(section: CollapsibleSection) {
    setCollapsedSections((prev) => ({
      ...prev,
      [section]: !prev[section],
    }))
  }

  function renderConversationItem(item: ConversationItem, showTypeBadge = false) {
    return (
      <button
        key={`${item.kind}:${item.id}`}
        className={[
          styles.friendItem,
          item.kind === activeConversation?.kind && item.id === activeConversation?.id ? styles.friendActive : '',
        ].join(' ')}
        onClick={() => selectConversation(item)}
        type="button"
      >
        <div className={styles.friendAvatarWrap}>
          <div className={[styles.friendAvatar, item.kind === 'group' ? styles.groupAvatar : ''].join(' ')}>
            {avatarInitial(item.title, item.kind === 'group' ? '群' : '友')}
          </div>
          {item.kind === 'friend' && (
            <div className={styles.onlineDot} data-status={item.presenceStatus ?? 'offline'} />
          )}
        </div>
        <div className={styles.friendMeta}>
          <div className={styles.friendNameRow}>
            <strong>{item.title}</strong>
            {showTypeBadge && (
              <span className={styles.conversationType}>{item.kind === 'group' ? '群' : '友'}</span>
            )}
            {item.kind === 'friend' && item.presenceStatus && (
              <span className={styles.presencePill} data-status={item.presenceStatus}>
                {presenceLabel(item.presenceStatus)}
              </span>
            )}
            {item.unreadCount > 0 && (
              <span className={styles.unreadBadge}>{item.unreadCount}</span>
            )}
          </div>
          <span className={styles.lastMsg}>
            {truncateText(item.subtitle, 28)}
          </span>
        </div>
        {item.lastMessageAt && (
          <span className={styles.msgTime}>{formatTime(item.lastMessageAt)}</span>
        )}
      </button>
    )
  }

  function renderConversationSection(
    section: CollapsibleSection,
    title: string,
    items: ConversationItem[],
    emptyText: string,
  ) {
    const collapsed = collapsedSections[section]
    const SectionIcon = collapsed ? ChevronRight : ChevronDown
    return (
      <section className={styles.conversationSection}>
        <button
          className={styles.sectionHeader}
          onClick={() => toggleSection(section)}
          type="button"
          aria-expanded={!collapsed}
        >
          <span className={styles.sectionTitle}>
            <SectionIcon size={14} strokeWidth={2.2} />
            {title}
          </span>
          <small>{items.length}</small>
        </button>
        {!collapsed && (
          items.length === 0
            ? <p className={styles.sectionHint}>{emptyText}</p>
            : items.map((item) => renderConversationItem(item))
        )}
      </section>
    )
  }

  return (
    <div className={styles.layout}>
      <aside className={styles.sidebar}>
        <div className={styles.sideHeader}>
          <span>会话</span>
          <small>{friends.length} 好友 · {groups.length} 群聊</small>
        </div>

        <form onSubmit={handleSearch} className={styles.searchForm}>
          <input
            className={styles.searchInput}
            value={searchQ}
            onChange={(e) => setSearchQ(e.target.value)}
            placeholder="搜索手机号添加好友"
          />
          <button className={styles.searchBtn} type="submit" disabled={searchLoading}>
            {searchLoading ? '…' : '搜'}
          </button>
        </form>

        {searchResults.length > 0 && (
          <div className={styles.searchResults}>
            {searchResults.filter((r) => !r.is_self).map((r) => (
              <div key={r.user.id} className={styles.searchItem}>
                <div className={styles.friendAvatar}>
                  {(r.user.nickname ?? r.user.account)[0]?.toUpperCase()}
                </div>
                <div className={styles.friendInfo}>
                  <strong>{r.user.nickname ?? r.user.account}</strong>
                  <span>{r.user.account}</span>
                </div>
                {r.already_friend
                  ? <span className={styles.alreadyFriend}>已是好友</span>
                  : (
                    <button
                      className={styles.addBtn}
                      disabled={addingId === r.user.id}
                      onClick={() => handleAddFriend(r)}
                      type="button"
                    >
                      {addingId === r.user.id ? '…' : '+'}
                    </button>
                  )}
              </div>
            ))}
          </div>
        )}

        <div className={styles.modeToggle} role="group" aria-label="会话显示模式">
          <button
            className={[styles.modeButton, displayMode === 'grouped' ? styles.modeActive : ''].join(' ')}
            onClick={() => setDisplayMode('grouped')}
            type="button"
            aria-pressed={displayMode === 'grouped'}
            title="好友和群聊分层显示，并支持折叠"
          >
            <Layers size={14} strokeWidth={2.1} />
            <span>分层</span>
          </button>
          <button
            className={[styles.modeButton, displayMode === 'active' ? styles.modeActive : ''].join(' ')}
            onClick={() => setDisplayMode('active')}
            type="button"
            aria-pressed={displayMode === 'active'}
            title="好友和群聊混在一起，最近活跃在前"
          >
            <List size={14} strokeWidth={2.1} />
            <span>活跃</span>
          </button>
        </div>

        <div className={styles.friendList}>
          {allConversationItems.length === 0 && (
            <p className={styles.hint}>暂无好友或群聊，搜索手机号添加好友</p>
          )}
          {displayMode === 'grouped' ? (
            <>
              {renderConversationSection('friends', '好友会话', friendConversationItems, '暂无好友会话')}
              {renderConversationSection('groups', '群聊', groupConversationItems, '暂无群聊')}
            </>
          ) : (
            <section className={styles.conversationSection}>
              <div className={styles.sectionHeader}>
                <span>活跃优先</span>
                <small>{allConversationItems.length}</small>
              </div>
              {allConversationItems.map((item) => renderConversationItem(item, true))}
            </section>
          )}
        </div>
      </aside>

      <div className={styles.chat}>
        <header className={styles.topbar}>
          {activeItem ? (
            <div className={styles.topbarFriend}>
              <div className={[styles.topbarAvatar, activeItem.kind === 'group' ? styles.groupAvatar : ''].join(' ')}>
                {avatarInitial(activeItem.title, activeItem.kind === 'group' ? '群' : '友')}
              </div>
              <div>
                <strong>{activeItem.title}</strong>
                <span
                  className={styles.onlineStatus}
                  data-status={activeItem.kind === 'friend' ? activeItem.presenceStatus ?? 'offline' : undefined}
                >
                  {activeItem.kind === 'group'
                    ? `${activeItem.group?.member_count ?? 0} 位成员`
                    : activeItem.presenceSummary ?? '离线'}
                </span>
              </div>
            </div>
          ) : (
            <span className={styles.topbarTitle}>选择好友或群聊开始聊天</span>
          )}
        </header>

        <div className={styles.feed} ref={feedRef}>
          {!activeConversation && (
            <div className={styles.welcome}>
              <p>从左侧选择一位好友或群聊开始聊天</p>
            </div>
          )}
          {messagesLoading && <p className={styles.hint}>读取消息…</p>}
          {messages.map((m, i) => {
            const isMe = m.outgoing || m.sender_user_id === me?.id
            const hasMarkdown = !isMe && /[#*`\[\]>|]/.test(m.content)
            const senderName = isMe
              ? (me?.nickname ?? me?.account ?? '我')
              : (activeItem?.kind === 'group'
                ? (m.sender_name ?? '群成员')
                : activeItem?.title ?? '对方')
            return (
              <div key={m.id ?? i} className={[styles.msgRow, isMe ? styles.ownRow : ''].join(' ')}>
                <div className={styles.avatar}>
                  {avatarInitial(senderName, isMe ? '我' : activeItem?.kind === 'group' ? '群' : '友')}
                </div>
                <div className={styles.msgBody}>
                  <div className={styles.msgMeta}>
                    <strong>{senderName}</strong>
                    <span>{formatTime(m.created_at)}</span>
                  </div>
                  {hasMarkdown
                    ? <div className={styles.msgContent}><MarkdownContent content={m.content} copy={false} /></div>
                    : <div className={styles.msgContent}>{m.content}</div>}
                </div>
              </div>
            )
          })}
        </div>

        {activeConversation && (
          <form className={styles.composer} onSubmit={handleSend}>
            <textarea
              ref={textareaRef}
              className={styles.composerInput}
              value={input}
              onChange={(e) => { setInput(e.target.value); autoResize() }}
              onKeyDown={handleKeyDown}
              placeholder={`发送消息到 ${activeTitle()}…`}
              disabled={sending}
              rows={1}
            />
            <button className={styles.sendBtn} type="submit" disabled={!input.trim() || sending}>
              {sending ? '…' : '发送'}
            </button>
          </form>
        )}
        {error && <p className={styles.sendError}>{error}</p>}
      </div>
    </div>
  )
}

const DISPLAY_MODE_STORAGE_KEY = 'elon_pc_friends_display_mode'
const COLLAPSED_SECTIONS_STORAGE_KEY = 'elon_pc_friends_collapsed_sections'

function readConversationDisplayMode(): ConversationDisplayMode {
  try {
    const stored = window.localStorage.getItem(DISPLAY_MODE_STORAGE_KEY)
    return stored === 'active' ? 'active' : 'grouped'
  } catch {
    return 'grouped'
  }
}

function readCollapsedSections(): Record<CollapsibleSection, boolean> {
  const fallback = { friends: false, groups: false }
  try {
    const stored = window.localStorage.getItem(COLLAPSED_SECTIONS_STORAGE_KEY)
    if (!stored) return fallback
    const parsed = JSON.parse(stored) as Partial<Record<CollapsibleSection, unknown>>
    return {
      friends: parsed.friends === true,
      groups: parsed.groups === true,
    }
  } catch {
    return fallback
  }
}

function writeLocalPreference(key: string, value: string) {
  try {
    window.localStorage.setItem(key, value)
  } catch {
    // localStorage can be blocked by browser privacy settings.
  }
}

function avatarInitial(value: string | undefined, fallback: string) {
  const chars = Array.from((value || fallback).trim())
  return (chars[0] || fallback).toUpperCase()
}

function timestampOf(value: string | undefined) {
  if (!value) return 0
  const ms = Date.parse(value)
  return Number.isFinite(ms) ? ms : 0
}

function sortConversationItems(items: ConversationItem[]) {
  return [...items].sort((a, b) => {
    const byTime = timestampOf(b.lastMessageAt) - timestampOf(a.lastMessageAt)
    if (byTime !== 0) return byTime
    return a.title.localeCompare(b.title, 'zh-Hans-CN')
  })
}

function friendPreviewText(friend: Friend) {
  return clean(friend.last_message) || '暂无消息'
}

function truncateText(value: string, length: number) {
  return value.length > length ? `${value.slice(0, length)}…` : value
}

function applyFriendPresencePatch(friend: Friend, detail: PresenceEvent['detail']): Friend {
  const status = normalizePresenceStatus(
    typeof detail.status === 'string' ? detail.status : friend.presence_status,
    typeof detail.isOnline === 'boolean' ? detail.isOnline : friend.is_online,
  )
  const isVisible = status !== 'offline'
  const hasCustomStatus = Object.prototype.hasOwnProperty.call(detail, 'customStatus')
    || Object.prototype.hasOwnProperty.call(detail, 'custom_status')
  const hasActivity = Object.prototype.hasOwnProperty.call(detail, 'activity')
  return {
    ...friend,
    is_online: isVisible,
    presence_status: status,
    custom_status: isVisible
      ? (hasCustomStatus ? detail.customStatus ?? detail.custom_status ?? null : friend.custom_status ?? null)
      : null,
    activity: isVisible
      ? (hasActivity ? detail.activity ?? null : friend.activity ?? null)
      : null,
  }
}

function friendPresence(friend: Friend) {
  const status = normalizePresenceStatus(friend.presence_status, friend.is_online)
  const details = status === 'offline'
    ? []
    : [clean(friend.activity), clean(friend.custom_status)].filter(Boolean)
  const label = presenceLabel(status)
  return {
    status,
    label,
    summary: [label, ...details].join(' · '),
  }
}

function normalizePresenceStatus(status: unknown, isOnline: unknown): PresenceStatus {
  const value = clean(status).toLowerCase()
  if (isOnline === false || value === 'offline' || value === 'invisible') return 'offline'
  if (value === 'idle' || value === 'dnd') return value
  return 'online'
}

function presenceLabel(status: PresenceStatus) {
  const labels: Record<PresenceStatus, string> = {
    online: '在线',
    idle: '离开',
    dnd: '勿扰',
    offline: '离线',
  }
  return labels[status]
}
