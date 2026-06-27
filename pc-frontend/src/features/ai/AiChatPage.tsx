import { useEffect, useRef, useState, useCallback } from 'react'
import { api } from '../../api/client'
import { useAuthStore } from '../../store/auth'
import { useModelStore } from '../models/useModelStore'
import { shortButtonLabel } from '../models/modelUtils'
import { ModelPickerPopover } from '../models/ModelPicker'
import MarkdownContent from '../markdown/MarkdownContent'
import { formatTime } from '../../lib/utils'
import styles from './AiChatPage.module.css'
import { v4 as uuidv4 } from 'uuid'

interface AiConversation {
  id: string
  title?: string
  updated_at?: string
  message_count?: number
}

interface AiMessage {
  id?: string
  role: 'user' | 'assistant' | 'system'
  content: string
  created_at?: string
}

interface LmChatResponse {
  reply?: string
  content?: string
  conversation_id?: string
}

interface Friend {
  id: string
  account: string
  nickname?: string
  is_online?: boolean
}

export default function AiChatPage() {
  const user = useAuthStore((s) => s.user)
  const selectedAgent = useModelStore((s) => s.selectedAgent)
  const modelLabel = useModelStore((s) => s.label)

  const [conversations, setConversations] = useState<AiConversation[]>([])
  const [activeConvId, setActiveConvId] = useState<string | null>(null)
  const [messages, setMessages] = useState<AiMessage[]>([])
  const [messagesLoading, setMessagesLoading] = useState(false)
  const [input, setInput] = useState('')
  const [sending, setSending] = useState(false)
  const [error, setError] = useState('')
  const [showModelPicker, setShowModelPicker] = useState(false)
  const [friends, setFriends] = useState<Friend[]>([])

  const feedRef = useRef<HTMLDivElement>(null)
  const textareaRef = useRef<HTMLTextAreaElement>(null)
  const modelBtnRef = useRef<HTMLButtonElement>(null)
  const atBottomRef = useRef(true)

  useEffect(() => {
    loadConversations()
    api.get<{ recommendations?: Friend[] }>('/api/me/friends/recommendations')
      .then(d => setFriends(d.recommendations ?? []))
      .catch(() => {})
  }, [user?.id]) // eslint-disable-line

  useEffect(() => {
    if (atBottomRef.current && feedRef.current) {
      feedRef.current.scrollTop = feedRef.current.scrollHeight
    }
  }, [messages])

  async function loadConversations() {
    try {
      const data = await api.get<{ conversations?: AiConversation[] }>(
        '/api/me/ai/conversations?limit=50',
      )
      setConversations(data.conversations ?? [])
    } catch { /* ignore */ }
  }

  async function selectConversation(convId: string) {
    setActiveConvId(convId)
    setMessages([])
    setMessagesLoading(true)
    try {
      const data = await api.get<{ messages?: AiMessage[] }>(
        `/api/me/ai/conversations/${encodeURIComponent(convId)}/messages?limit=100`,
      )
      setMessages(data.messages ?? [])
    } catch { /* ignore */ }
    finally { setMessagesLoading(false) }
  }

  function newConversation() {
    const id = uuidv4()
    setActiveConvId(id)
    setMessages([])
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
    if (!text || sending) return
    setInput('')
    setError('')
    if (textareaRef.current) textareaRef.current.style.height = '46px'

    const convId = activeConvId ?? uuidv4()
    if (!activeConvId) setActiveConvId(convId)

    // 乐观更新：先显示用户消息
    const userMsg: AiMessage = { role: 'user', content: text, created_at: new Date().toISOString() }
    setMessages((prev) => [...prev, userMsg])
    atBottomRef.current = true

    setSending(true)
    try {
      const res = await api.post<LmChatResponse>('/api/llm/chat', {
        messages: [{ role: 'user', content: text }],
        agent: selectedAgent || null,
        conversation_id: convId,
        scope: 'chat_memory',
      })
      const reply = res.reply ?? res.content ?? ''
      const aiMsg: AiMessage = { role: 'assistant', content: reply, created_at: new Date().toISOString() }
      setMessages((prev) => [...prev, aiMsg])
      // 刷新会话列表（标题可能从服务端更新）
      loadConversations()
    } catch (err) {
      setError((err as { message?: string }).message ?? '发送失败')
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

  return (
    <div className={styles.layout}>
      {/* 会话列表（左栏）*/}
      <aside className={styles.sidebar}>
        <div className={styles.sideHeader}>
          <span>一龙 AI</span>
          <button className={styles.newBtn} onClick={newConversation} title="新对话" type="button">+</button>
        </div>
        <div className={styles.convList}>
          {conversations.length === 0 && (
            <p className={styles.hint}>还没有对话记录</p>
          )}
          {conversations.map((c) => (
            <button
              key={c.id}
              className={[styles.convItem, c.id === activeConvId ? styles.convActive : ''].join(' ')}
              onClick={() => selectConversation(c.id)}
              type="button"
            >
              <strong className={styles.convTitle}>{c.title ?? '新对话'}</strong>
              <span className={styles.convMeta}>
                {c.message_count ? `${c.message_count} 条` : ''}
                {c.updated_at ? ` · ${formatTime(c.updated_at)}` : ''}
              </span>
            </button>
          ))}
        </div>
        <div className={styles.userStrip}>
          <div className={styles.userDot}>
            {(user?.nickname ?? user?.account ?? '?')[0]?.toUpperCase()}
          </div>
          <div className={styles.userInfo}>
            <strong>{user?.nickname ?? user?.account}</strong>
            <span>{user?.account}</span>
          </div>
        </div>
      </aside>

      {/* 聊天区 */}
      <div className={styles.chat}>
        <header className={styles.topbar}>
          <span className={styles.topbarTitle}>
            {activeConvId
              ? (conversations.find((c) => c.id === activeConvId)?.title ?? '新对话')
              : '一龙 AI'}
          </span>
          <div className={styles.topbarRight}>
            <span className={styles.modelBadge}>{shortButtonLabel(modelLabel)}</span>
          </div>
        </header>

        <div
          className={styles.feed}
          ref={feedRef}
          onScroll={() => {
            const el = feedRef.current
            if (el) atBottomRef.current = el.scrollHeight - el.scrollTop - el.clientHeight < 80
          }}
        >
          {!activeConvId && (
            <div className={styles.welcome}>
              <h2>你好，我是一龙 AI</h2>
              <p>随时可以开始对话，我会记住我们聊过的内容。</p>
              <button className={styles.startBtn} onClick={newConversation} type="button">
                + 开始新对话
              </button>
            </div>
          )}
          {messagesLoading && <p className={styles.hint}>读取消息…</p>}
          {messages.filter((m) => m.role !== 'system').map((m, i) => {
            const isUser = m.role === 'user'
            const hasMarkdown = !isUser && /[#*`\[\]>|]/.test(m.content)
            return (
              <div key={i} className={[styles.msgRow, isUser ? styles.ownRow : ''].join(' ')}>
                <div className={styles.avatar}>{isUser ? (user?.account?.[0]?.toUpperCase() ?? '我') : 'AI'}</div>
                <div className={styles.msgBody}>
                  <div className={styles.msgMeta}>
                    <strong>{isUser ? (user?.nickname ?? user?.account ?? '我') : 'AI'}</strong>
                    {m.created_at && <span>{formatTime(m.created_at)}</span>}
                  </div>
                  {hasMarkdown
                    ? <div className={styles.msgContent}><MarkdownContent content={m.content} copy /></div>
                    : <div className={styles.msgContent}>{m.content}</div>}
                </div>
              </div>
            )
          })}
          {sending && (
            <div className={styles.msgRow}>
              <div className={styles.avatar}>AI</div>
              <div className={styles.msgBody}>
                <div className={styles.typing}>
                  <span /><span /><span />
                </div>
              </div>
            </div>
          )}
        </div>

        {activeConvId !== null && (
          <form className={styles.composer} onSubmit={handleSend}>
            <button
              ref={modelBtnRef}
              className={styles.modelBtn}
              type="button"
              title={`AI 模型：${modelLabel || '服务器默认'}`}
              onClick={() => setShowModelPicker((v) => !v)}
            >
              {shortButtonLabel(modelLabel)}
            </button>
            <textarea
              ref={textareaRef}
              className={styles.composerInput}
              value={input}
              onChange={(e) => { setInput(e.target.value); autoResize() }}
              onKeyDown={handleKeyDown}
              placeholder="输入消息，Enter 发送，Shift+Enter 换行"
              disabled={sending}
              rows={1}
            />
            <button
              className={styles.sendBtn}
              type="submit"
              disabled={!input.trim() || sending}
            >
              {sending ? '…' : '发送'}
            </button>
          </form>
        )}
        {error && <p className={styles.sendError}>{error}</p>}
      </div>

      {/* ══ 右侧好友栏 ══ */}
      <aside className={styles.userPanel}>
        <div className={styles.userPanelTitle}>
          <span>用户{friends.length > 0 ? ` — ${friends.length}` : ''}</span>
        </div>
        <div className={styles.userPanelList}>
          {friends.length === 0 && (
            <p className={styles.userPanelHint}>暂无联系人</p>
          )}
          {/* 在线 */}
          {friends.filter(f => f.is_online).length > 0 && (
            <>
              <div className={styles.userPanelSection}>
                在线 · {friends.filter(f => f.is_online).length}
              </div>
              {friends.filter(f => f.is_online).map(f => (
                <div key={f.id} className={styles.userPanelItem}>
                  <div className={[styles.userPanelAvatar, styles.userPanelAvatarOnline].join(' ')}>
                    {(f.nickname ?? f.account)[0].toUpperCase()}
                  </div>
                  <div className={styles.userPanelCopy}>
                    <strong className={styles.userPanelName}>{f.nickname ?? f.account}</strong>
                    <span className={styles.userPanelSub}>在线</span>
                  </div>
                </div>
              ))}
            </>
          )}
          {/* 离线 */}
          {friends.filter(f => !f.is_online).length > 0 && (
            <>
              <div className={styles.userPanelSection}>
                离线 · {friends.filter(f => !f.is_online).length}
              </div>
              {friends.filter(f => !f.is_online).map(f => (
                <div key={f.id} className={styles.userPanelItem}>
                  <div className={[styles.userPanelAvatar, styles.userPanelAvatarOffline].join(' ')}>
                    {(f.nickname ?? f.account)[0].toUpperCase()}
                  </div>
                  <div className={styles.userPanelCopy}>
                    <strong className={styles.userPanelName}>{f.nickname ?? f.account}</strong>
                    <span className={styles.userPanelSub}>离线</span>
                  </div>
                </div>
              ))}
            </>
          )}
        </div>
      </aside>

      {showModelPicker && (
        <ModelPickerPopover anchorRef={modelBtnRef} onClose={() => setShowModelPicker(false)} />
      )}
    </div>
  )
}
