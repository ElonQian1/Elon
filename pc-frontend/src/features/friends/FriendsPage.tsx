import { useEffect, useState, useMemo } from 'react'
import { ChevronDown, ChevronRight, Layers, List } from 'lucide-react'
import { api } from '../../api/client'
import { useAuthStore } from '../../store/auth'
import { clean, formatTime } from '../../lib/utils'
import { displayMessageContentOrAttachment } from '../../lib/messageDisplay'
import WorkspaceFeatureNav from '../shell/WorkspaceFeatureNav'
import styles from './FriendsPage.module.css'
import SocialAvatar from './SocialAvatar'
import type { Friend, FriendGroup } from './socialMessageTypes'
import useSocialChat from './useSocialChat'
import SocialConversation from './SocialConversation'
import ArticleWorkspace from '../articles/ArticleWorkspace'

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
  const userId = useAuthStore(s => s.user?.id)
  return userId ? <FriendsPageContent key={userId} /> : null
}

function FriendsPageContent() {
  const me = useAuthStore((s) => s.user)
  const { friends, groups, setFriends, activeConversation, messages, setMessages, messagesLoading,
    input, setInput, selectConversation, loadSocialConversations, revisionNotice,
    listStatus, messageError, cacheWarning, retry } = useSocialChat(me!.id)
  const [displayMode, setDisplayMode] = useState<ConversationDisplayMode>(() => readConversationDisplayMode())
  const [collapsedSections, setCollapsedSections] = useState<Record<CollapsibleSection, boolean>>(
    () => readCollapsedSections(),
  )

  // 搜索添加好友
  const [searchQ, setSearchQ] = useState('')
  const [searchResults, setSearchResults] = useState<SearchResult[]>([])
  const [searchLoading, setSearchLoading] = useState(false)
  const [addingId, setAddingId] = useState<string | null>(null)


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
        lastMessage: displayMessageContentOrAttachment(friend.last_message),
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
      subtitle: displayMessageContentOrAttachment(group.last_message) || `${group.member_count ?? 0} 位成员`,
      lastMessage: displayMessageContentOrAttachment(group.last_message),
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

  async function handleSearch(e: React.FormEvent) {
    e.preventDefault()
    const q = searchQ.trim()
    if (!q) return
    setSearchLoading(true)
    setSearchResults([])
    try {
      const data = await api.get<FriendSearchResponse>(
        `/api/me/friends/search?phone=${encodeURIComponent(q)}`,
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
            {item.kind === 'friend'
              ? <SocialAvatar userId={item.id} name={item.title} avatar={item.friend?.avatar_data_url} />
              : avatarInitial(item.title, '群')}
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
          <small>{friends.length || listStatus.friends === 'ready' ? friends.length : listStatus.friends === 'loading' ? '加载中' : '待同步'} 好友 · {groups.length || listStatus.groups === 'ready' ? groups.length : listStatus.groups === 'loading' ? '加载中' : '待同步'} 群聊</small>
        </div>
        <WorkspaceFeatureNav />
        <div className={styles.syncStatus} role="status" aria-live="polite">
          <span>{Object.values(listStatus).includes('error') ? '会话同步失败，已保留现有列表；将自动重试' : Object.values(listStatus).includes('loading') ? '正在同步会话…' : '会话已同步'}</span>
          <button type="button" className={styles.syncRetry} onClick={retry}>重新同步</button>
          {cacheWarning && <p>{cacheWarning}</p>}
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
          {allConversationItems.length === 0 && Object.values(listStatus).every(status => status === 'ready') && (
            <p className={styles.hint}>暂无好友或群聊，搜索手机号添加好友</p>
          )}
          {displayMode === 'grouped' ? (
            <>
              {renderConversationSection('friends', '好友会话', friendConversationItems, listStatus.friends === 'ready' ? '暂无好友会话' : listStatus.friends === 'loading' ? '正在加载好友…' : '好友列表暂不可用')}
              {renderConversationSection('groups', '群聊', groupConversationItems, listStatus.groups === 'ready' ? '暂无群聊' : listStatus.groups === 'loading' ? '正在加载群聊…' : '群聊列表暂不可用')}
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
                {activeItem.kind === 'friend'
                  ? <SocialAvatar userId={activeItem.id} name={activeItem.title} avatar={activeItem.friend?.avatar_data_url} />
                  : avatarInitial(activeItem.title, '群')}
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
          {activeItem?.kind === 'group' && <ArticleWorkspace key={activeItem.id} groupId={activeItem.id} groups={groups} />}
        </header>

        {activeConversation ? <SocialConversation conversation={activeConversation} title={activeItem?.title ?? '会话'} me={me!}
          friend={activeItem?.friend} group={activeItem?.group} messages={messages} setMessages={setMessages} input={input} setInput={setInput}
          targets={allConversationItems} loading={messagesLoading} error={messageError} retry={retry} onSent={() => void loadSocialConversations()} />
          : <div className={styles.welcome}><p>从左侧选择一位好友或群聊开始聊天</p></div>}
        {revisionNotice && <p className={styles.hint} role="status">{revisionNotice}</p>}
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
  return displayMessageContentOrAttachment(friend.last_message) || '暂无消息'
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
