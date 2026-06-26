import { useEffect, useRef, useState, useCallback } from 'react'
import { api } from '../../api/client'
import { useAuthStore } from '../../store/auth'
import { formatTime } from '../../lib/utils'
import MarkdownContent from '../markdown/MarkdownContent'
import styles from './FriendsPage.module.css'

interface Friend {
  id: string
  account: string
  nickname?: string
  last_message?: string
  last_message_at?: string
  unread_count?: number
  is_online?: boolean
}

interface FriendMessage {
  id: string
  sender_user_id: string
  content: string
  created_at: string
  outgoing: boolean
}

interface SearchResult {
  user: Friend
  already_friend: boolean
  is_self: boolean
}

export default function FriendsPage() {
  const me = useAuthStore((s) => s.user)
  const [friends, setFriends] = useState<Friend[]>([])
  const [activeFriendId, setActiveFriendId] = useState<string | null>(null)
  const [messages, setMessages] = useState<FriendMessage[]>([])
  const [messagesLoading, setMessagesLoading] = useState(false)
  const [input, setInput] = useState('')
  const [sending, setSending] = useState(false)
  const [error, setError] = useState('')

  // 搜索添加好友
  const [searchQ, setSearchQ] = useState('')
  const [searchResults, setSearchResults] = useState<SearchResult[]>([])
  const [searchLoading, setSearchLoading] = useState(false)
  const [addingId, setAddingId] = useState<string | null>(null)

  const feedRef = useRef<HTMLDivElement>(null)
  const textareaRef = useRef<HTMLTextAreaElement>(null)

  useEffect(() => { loadFriends() }, [me?.id]) // eslint-disable-line

  useEffect(() => {
    if (feedRef.current) feedRef.current.scrollTop = feedRef.current.scrollHeight
  }, [messages])

  async function loadFriends() {
    try {
      const data = await api.get<{ friends?: Friend[] }>('/api/me/friends')
      setFriends(data.friends ?? [])
    } catch { /* ignore */ }
  }

  async function selectFriend(friendId: string) {
    setActiveFriendId(friendId)
    setMessages([])
    setMessagesLoading(true)
    setError('')
    try {
      const data = await api.get<{ messages?: FriendMessage[] }>(
        `/api/me/friends/${encodeURIComponent(friendId)}/messages?limit=80`,
      )
      setMessages(data.messages ?? [])
    } catch { /* ignore */ }
    finally { setMessagesLoading(false) }
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
    if (!text || sending || !activeFriendId) return
    setInput('')
    setError('')
    if (textareaRef.current) textareaRef.current.style.height = '46px'
    setSending(true)
    // 乐观更新
    const optimistic: FriendMessage = {
      id: `tmp-${Date.now()}`,
      sender_user_id: me?.id ?? '',
      content: text,
      created_at: new Date().toISOString(),
      outgoing: true,
    }
    setMessages((prev) => [...prev, optimistic])
    try {
      const res = await api.post<{ message?: FriendMessage }>(
        `/api/me/friends/${encodeURIComponent(activeFriendId)}/messages`,
        { content: text },
      )
      // 用服务端返回替换乐观消息
      if (res.message) {
        setMessages((prev) => prev.map((m) => m.id === optimistic.id ? res.message! : m))
      }
      // 刷新好友列表（更新最后消息）
      loadFriends()
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
      const data = await api.get<{ results?: SearchResult[] }>(
        `/api/me/friends/search?q=${encodeURIComponent(q)}`,
      )
      setSearchResults(data.results ?? [])
    } catch { /* ignore */ }
    finally { setSearchLoading(false) }
  }

  async function handleAddFriend(userId: string) {
    setAddingId(userId)
    try {
      await api.post('/api/me/friends', { phone: userId })
      await loadFriends()
      setSearchResults((prev) => prev.map((r) =>
        r.user.id === userId ? { ...r, already_friend: true } : r,
      ))
    } catch (err) {
      alert((err as { message?: string }).message ?? '添加失败')
    } finally {
      setAddingId(null)
    }
  }

  const activeFriend = friends.find((f) => f.id === activeFriendId)

  return (
    <div className={styles.layout}>
      {/* 好友列表（左栏）*/}
      <aside className={styles.sidebar}>
        <div className={styles.sideHeader}>
          <span>好友</span>
        </div>

        {/* 搜索栏 */}
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

        {/* 搜索结果 */}
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
                      onClick={() => handleAddFriend(r.user.account)}
                      type="button"
                    >
                      {addingId === r.user.id ? '…' : '+'}
                    </button>
                  )}
              </div>
            ))}
          </div>
        )}

        {/* 好友列表 */}
        <div className={styles.friendList}>
          {friends.length === 0 && (
            <p className={styles.hint}>暂无好友，搜索手机号添加</p>
          )}
          {friends.map((f) => (
            <button
              key={f.id}
              className={[styles.friendItem, f.id === activeFriendId ? styles.friendActive : ''].join(' ')}
              onClick={() => selectFriend(f.id)}
              type="button"
            >
              <div className={styles.friendAvatarWrap}>
                <div className={styles.friendAvatar}>
                  {(f.nickname ?? f.account)[0]?.toUpperCase()}
                </div>
                {f.is_online && <div className={styles.onlineDot} />}
              </div>
              <div className={styles.friendMeta}>
                <div className={styles.friendNameRow}>
                  <strong>{f.nickname ?? f.account}</strong>
                  {(f.unread_count ?? 0) > 0 && (
                    <span className={styles.unreadBadge}>{f.unread_count}</span>
                  )}
                </div>
                <span className={styles.lastMsg}>
                  {f.last_message
                    ? f.last_message.slice(0, 28) + (f.last_message.length > 28 ? '…' : '')
                    : ''}
                </span>
              </div>
              {f.last_message_at && (
                <span className={styles.msgTime}>{formatTime(f.last_message_at)}</span>
              )}
            </button>
          ))}
        </div>
      </aside>

      {/* 聊天区 */}
      <div className={styles.chat}>
        <header className={styles.topbar}>
          {activeFriend ? (
            <div className={styles.topbarFriend}>
              <div className={styles.topbarAvatar}>
                {(activeFriend.nickname ?? activeFriend.account)[0]?.toUpperCase()}
              </div>
              <div>
                <strong>{activeFriend.nickname ?? activeFriend.account}</strong>
                <span className={[styles.onlineStatus, activeFriend.is_online ? styles.onlineTrue : ''].join(' ')}>
                  {activeFriend.is_online ? '在线' : '离线'}
                </span>
              </div>
            </div>
          ) : (
            <span className={styles.topbarTitle}>选择好友开始聊天</span>
          )}
        </header>

        <div className={styles.feed} ref={feedRef}>
          {!activeFriendId && (
            <div className={styles.welcome}>
              <p>从左侧选择一位好友开始私聊</p>
            </div>
          )}
          {messagesLoading && <p className={styles.hint}>读取消息…</p>}
          {messages.map((m, i) => {
            const isMe = m.outgoing || m.sender_user_id === me?.id
            const hasMarkdown = !isMe && /[#*`\[\]>|]/.test(m.content)
            return (
              <div key={m.id ?? i} className={[styles.msgRow, isMe ? styles.ownRow : ''].join(' ')}>
                <div className={styles.avatar}>
                  {isMe
                    ? (me?.account?.[0]?.toUpperCase() ?? '我')
                    : (activeFriend?.nickname ?? activeFriend?.account ?? '?')[0]?.toUpperCase()}
                </div>
                <div className={styles.msgBody}>
                  <div className={styles.msgMeta}>
                    <strong>{isMe ? (me?.nickname ?? me?.account ?? '我') : (activeFriend?.nickname ?? activeFriend?.account ?? '对方')}</strong>
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

        {activeFriendId && (
          <form className={styles.composer} onSubmit={handleSend}>
            <textarea
              ref={textareaRef}
              className={styles.composerInput}
              value={input}
              onChange={(e) => { setInput(e.target.value); autoResize() }}
              onKeyDown={handleKeyDown}
              placeholder={`发送消息给 ${activeFriend?.nickname ?? activeFriend?.account ?? '好友'}…`}
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
