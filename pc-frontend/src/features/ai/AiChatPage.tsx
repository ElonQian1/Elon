import { useEffect, useRef, useState, useCallback } from 'react'
import { api } from '../../api/client'
import { useAuthStore } from '../../store/auth'
import { useModelStore } from '../models/useModelStore'
import { shortButtonLabel } from '../models/modelUtils'
import { ModelPickerPopover } from '../models/ModelPicker'
import MarkdownContent from '../markdown/MarkdownContent'
import { formatTime } from '../../lib/utils'
import NodeStatusBanner from './NodeStatusBanner'
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
  // 节点本机执行输出扩展字段
  node_exec?: boolean
  node_display_name?: string
  exit_ok?: boolean
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
  avatar_data_url?: string | null
  is_online?: boolean
}

export default function AiChatPage() {
  const user = useAuthStore((s) => s.user)
  const selectedAgent = useModelStore((s) => s.selectedAgent)
  const modelLabel = useModelStore((s) => s.label)

  const [conversations, setConversations] = useState<AiConversation[]>([])
  // 初始即创建新会话 ID，保证输入框始终可见（与旧版一致）
  const [activeConvId, setActiveConvId] = useState<string>(() => uuidv4())
  const [messages, setMessages] = useState<AiMessage[]>([])
  const [messagesLoading, setMessagesLoading] = useState(false)
  const [input, setInput] = useState('')
  const [sending, setSending] = useState(false)
  const [error, setError] = useState('')
  const [showModelPicker, setShowModelPicker] = useState(false)
  const [friends, setFriends] = useState<Friend[]>([])
  const [totalUserCount, setTotalUserCount] = useState(0)
  const [userQuery, setUserQuery] = useState('')
  // 节点在线状态（由本页面轮询，同时传给 NodeStatusBanner 避免重复请求）
  const [onlineNodeId, setOnlineNodeId] = useState<string | null>(null)
  const [onlineNodeName, setOnlineNodeName] = useState<string>('')

  const feedRef = useRef<HTMLDivElement>(null)
  const textareaRef = useRef<HTMLTextAreaElement>(null)
  const modelBtnRef = useRef<HTMLButtonElement>(null)
  const atBottomRef = useRef(true)

  useEffect(() => {
    loadConversations()
    api.get<{ recommendations?: Friend[]; total_count?: number }>('/api/me/friends/recommendations?limit=50')
      .then(d => {
        setFriends(d.recommendations ?? [])
        setTotalUserCount(d.total_count ?? d.recommendations?.length ?? 0)
      })
      .catch(() => {})
  }, [user?.id]) // eslint-disable-line

  // ── 节点状态轮询（每 6s）──────────────────────────────────────────────
  useEffect(() => {
    function checkNode() {
      api.get<{ nodes?: Array<{ node_id: string; online: boolean; ai_cli_ready: boolean; display_name: string; device_name?: string }> }>('/api/me/nodes')
        .then(d => {
          const on = (d.nodes ?? []).find(n => n.online && (n.ai_cli_ready || (d.nodes ?? []).some(x => x.online)))
          if (on) {
            setOnlineNodeId(on.node_id)
            setOnlineNodeName(on.display_name || on.device_name || on.node_id.slice(0, 8))
          } else {
            setOnlineNodeId(null)
            setOnlineNodeName('')
          }
        })
        .catch(() => {})
    }
    checkNode()
    const t = setInterval(checkNode, 6000)
    return () => clearInterval(t)
  }, [user?.id]) // eslint-disable-line

  // 客户端搜索过滤
  const filteredFriends = userQuery.trim()
    ? friends.filter(f =>
        (f.nickname ?? f.account).toLowerCase().includes(userQuery.toLowerCase())
      )
    : friends

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

    const convId = activeConvId

    // 乐观更新：先显示用户消息
    const userMsg: AiMessage = { role: 'user', content: text, created_at: new Date().toISOString() }
    setMessages((prev) => [...prev, userMsg])
    atBottomRef.current = true

    setSending(true)
    try {
      if (onlineNodeId) {
        // ── 节点在线：直接在用户电脑上执行 ────────────────────────────────
        const res = await api.post<{ output: string; req_id: string; node_id: string; node_display_name: string; exit_ok: boolean; error?: string }>(
          '/api/me/node/exec',
          { prompt: text, node_id: onlineNodeId },
        )
        const nodeMsg: AiMessage = {
          role: 'assistant',
          content: res.output || (res.error ? `执行失败：${res.error}` : '（无输出）'),
          created_at: new Date().toISOString(),
          node_exec: true,
          node_display_name: res.node_display_name || onlineNodeName,
          exit_ok: res.exit_ok,
        }
        setMessages((prev) => [...prev, nodeMsg])
      } else {
        // ── 无节点：走云端 AI 对话 ───────────────────────────────────────
        const res = await api.post<LmChatResponse>('/api/llm/chat', {
          messages: [{ role: 'user', content: text }],
          agent: selectedAgent || null,
          conversation_id: convId,
          scope: 'chat_memory',
        })
        const reply = res.reply ?? res.content ?? ''
        const aiMsg: AiMessage = { role: 'assistant', content: reply, created_at: new Date().toISOString() }
        setMessages((prev) => [...prev, aiMsg])
        loadConversations()
      }
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
            {conversations.find((c) => c.id === activeConvId)?.title ?? '新对话'}
          </span>
          <div className={styles.topbarRight}>
            <span className={styles.modelBadge}>{shortButtonLabel(modelLabel)}</span>
            <button className={styles.topbarBtn} type="button"
              title="分享这台电脑的算力" onClick={() => { window.location.href = '/pc/node' }}>
              分享算力
            </button>
            <button className={styles.topbarBtn} type="button"
              title="打开移动端入口" onClick={() => window.open('/app/download', '_blank', 'noopener')}>
              打开移动端
            </button>
            <button className={styles.topbarBtn} type="button"
              title="切换到旧版" onClick={() => {
                try {
                  const raw = localStorage.getItem('elon_auth')
                  if (raw) {
                    const tok = JSON.parse(raw)?.state?.token
                    if (tok) {
                      localStorage.setItem('lodex_token', tok)
                      localStorage.setItem('elon_token', tok)
                    }
                  }
                } catch {}
                window.open('/pc-legacy', '_blank', 'noopener')
              }}>
              旧版
            </button>
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
          {/* 本机节点状态横幅 */}
          <NodeStatusBanner onlineNodeId={onlineNodeId} onlineNodeName={onlineNodeName} />
          {messages.length === 0 && !messagesLoading && (
            <div className={styles.welcome}>
              <h2>你好，我是一龙 AI</h2>
              <p>{onlineNodeId ? `本机「${onlineNodeName}」已就绪，直接输入需求或命令。` : '随时可以开始对话，我会记住我们聊过的内容。'}</p>
            </div>
          )}
          {messagesLoading && <p className={styles.hint}>读取消息…</p>}
          {messages.filter((m) => m.role !== 'system').map((m, i) => {
            const isUser = m.role === 'user'
            const isNode = !isUser && m.node_exec === true
            const hasMarkdown = !isUser && !isNode && /[#*`\[\]>|]/.test(m.content)
            const avatarLabel = isUser ? (user?.account?.[0]?.toUpperCase() ?? '我') : (isNode ? '🖥' : 'AI')
            const nameLabel = isUser ? (user?.nickname ?? user?.account ?? '我') : (isNode ? `本机 · ${m.node_display_name ?? ''}` : 'AI')
            return (
              <div key={i} className={[styles.msgRow, isUser ? styles.ownRow : ''].join(' ')}>
                <div className={[styles.avatar, isNode ? styles.nodeAvatar : ''].join(' ')}>{avatarLabel}</div>
                <div className={styles.msgBody}>
                  <div className={styles.msgMeta}>
                    <strong className={isNode ? styles.nodeLabel : ''}>{nameLabel}</strong>
                    {m.created_at && <span>{formatTime(m.created_at)}</span>}
                    {isNode && m.exit_ok === false && <span className={styles.exitFail}>执行失败</span>}
                  </div>
                  {isNode
                    ? <pre className={styles.nodeOutput}>{m.content}</pre>
                    : hasMarkdown
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
        {error && <p className={styles.sendError}>{error}</p>}
      </div>

      {/* ══ 右侧用户栏 ══ */}
      <aside className={styles.userPanel}>
        <div className={styles.userPanelTitle}>
          <span>用户{friends.length > 0 ? ` — ${friends.length}` : ''}</span>
          {totalUserCount > friends.length && (
            <small className={styles.userPanelMore}>共{totalUserCount}位</small>
          )}
        </div>
        <div className={styles.userPanelList}>
          {/* 搜索框 */}
          {friends.length > 0 && (
            <div className={styles.userPanelSearch}>
              <input
                className={styles.userPanelSearchInput}
                value={userQuery}
                onChange={e => setUserQuery(e.target.value)}
                placeholder="搜索用户"
                autoComplete="off"
              />
              {userQuery && (
                <button className={styles.userPanelSearchClear} type="button" onClick={() => setUserQuery('')}>×</button>
              )}
            </div>
          )}
          {friends.length === 0 && (
            <p className={styles.userPanelHint}>暂无用户</p>
          )}
          {filteredFriends.length === 0 && userQuery && (
            <p className={styles.userPanelHint}>没有匹配的用户</p>
          )}
          {/* 在线 */}
          {filteredFriends.filter(f => f.is_online).length > 0 && (
            <>
              <div className={styles.userPanelSection}>
                在线 · {filteredFriends.filter(f => f.is_online).length}
              </div>
              {filteredFriends.filter(f => f.is_online).map(f => (
                <div key={f.id} className={styles.userPanelItem}>
                  <div className={[styles.userPanelAvatar, styles.userPanelAvatarOnline].join(' ')}>
                    {f.avatar_data_url
                      ? <img src={f.avatar_data_url} alt="" style={{ width: '100%', height: '100%', borderRadius: '50%', objectFit: 'cover', display: 'block' }} />
                      : (f.nickname ?? f.account)[0].toUpperCase()
                    }
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
          {filteredFriends.filter(f => !f.is_online).length > 0 && (
            <>
              <div className={styles.userPanelSection}>
                离线 · {filteredFriends.filter(f => !f.is_online).length}
              </div>
              {filteredFriends.filter(f => !f.is_online).map(f => (
                <div key={f.id} className={styles.userPanelItem}>
                  <div className={[styles.userPanelAvatar, styles.userPanelAvatarOffline].join(' ')}>
                    {f.avatar_data_url
                      ? <img src={f.avatar_data_url} alt="" style={{ width: '100%', height: '100%', borderRadius: '50%', objectFit: 'cover', display: 'block' }} />
                      : (f.nickname ?? f.account)[0].toUpperCase()
                    }
                  </div>
                  <div className={styles.userPanelCopy}>
                    <strong className={styles.userPanelName}>{f.nickname ?? f.account}</strong>
                    <span className={styles.userPanelSub}>离线</span>
                  </div>
                </div>
              ))}
            </>
          )}
          {/* 提示条 */}
          {totalUserCount > friends.length && !userQuery && (
            <p className={styles.userPanelHint}>已显示 {friends.length} 位，可搜索查找其他用户</p>
          )}
        </div>
      </aside>

      {showModelPicker && (
        <ModelPickerPopover anchorRef={modelBtnRef} onClose={() => setShowModelPicker(false)} />
      )}
    </div>
  )
}
