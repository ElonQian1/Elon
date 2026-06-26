import { useEffect, useRef, useState } from 'react'
import { useProjectStore } from './useProjectStore'
import { useAuthStore } from '../../store/auth'
import { useModelStore } from '../models/useModelStore'
import { DevTaskMessage } from '../dev/DevTaskCard'
import { buildContext } from '../dev/devTaskUtils'
import { CreateProjectModal } from '../projects/CreateProjectModal'
import { formatTime, clean } from '../../lib/utils'
import type { Message } from './types'
import styles from './ConversationPage.module.css'

export default function ConversationPage() {
  const user = useAuthStore((s) => s.user)
  const {
    projects, projectsLoaded, activeProjectId, channels, activeChannelId,
    messages, messagesLoading, sendingMessage,
    loadProjects, selectProject, selectChannel, sendMessage, cancelTask, approveTool,
  } = useProjectStore()
  const selectedAgent = useModelStore((s) => s.selectedAgent)
  const [input, setInput] = useState('')
  const [sendError, setSendError] = useState('')
  const [showCreate, setShowCreate] = useState(false)
  const feedRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    loadProjects()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [user?.id])

  useEffect(() => {
    // 自动滚到底
    if (feedRef.current) feedRef.current.scrollTop = feedRef.current.scrollHeight
  }, [messages])

  async function handleSend(e: React.FormEvent) {
    e.preventDefault()
    const text = input.trim()
    if (!text || sendingMessage) return
    setInput('')
    setSendError('')
    try {
      await sendMessage(text, selectedAgent || null)
    } catch (err) {
      setSendError((err as { message?: string }).message ?? '发送失败')
    }
  }

  const activeProject = projects.find((p) => p.id === activeProjectId)
  const activeChannel = channels.find((c) => c.id === activeChannelId)
  const isDevChannel = activeChannel?.kind === 'ai_development'

  // 构建 dev task 上下文（一次性，messages 变化时重算）
  const taskContext = buildContext(messages as Parameters<typeof buildContext>[0])

  return (
    <div className={styles.layout}>
      {/* 项目列表 */}
      <aside className={styles.projectSidebar}>
        <div className={styles.sideHeader}>
          <span>我的项目</span>
          <button className={styles.newProjectBtn} onClick={() => setShowCreate(true)} title="新建项目">+</button>
        </div>
        {!projectsLoaded && <p className={styles.sideHint}>读取中…</p>}
        {projectsLoaded && projects.length === 0 && (
          <p className={styles.sideHint}>暂无项目，点击 + 新建</p>
        )}
        {projects.map((p) => (
          <button
            key={p.id}
            className={[styles.projectBtn, p.id === activeProjectId ? styles.projectActive : ''].join(' ')}
            onClick={() => selectProject(p.id)}
          >
            <span className={styles.projectIcon}>{p.name?.[0]?.toUpperCase() ?? '?'}</span>
            <span className={styles.projectMeta}>
              <strong>{p.name}</strong>
              {p.description && <small>{p.description}</small>}
            </span>
          </button>
        ))}
      </aside>

      {/* 频道列表 */}
      {activeProjectId && (
        <aside className={styles.channelSidebar}>
          <div className={styles.sideHeader}>
            <span>{activeProject?.name ?? '频道'}</span>
          </div>
          {channels.map((c) => (
            <button
              key={c.id}
              className={[styles.channelBtn, c.id === activeChannelId ? styles.channelActive : ''].join(' ')}
              onClick={() => selectChannel(c.id)}
            >
              <span className={styles.channelIcon}>
                {c.kind === 'ai_development' ? '🛠️' : c.kind === 'announce' ? '📢' : '#'}
              </span>
              <span className={styles.channelName}>{c.name}</span>
            </button>
          ))}
        </aside>
      )}

      {/* 消息区域 */}
      <div className={styles.main}>
        {!activeProjectId && (
          <div className={styles.placeholder}>
            <h2>选择一个项目</h2>
            <p>从左侧选择项目，或点击 + 新建</p>
            <button className={styles.bigCreateBtn} onClick={() => setShowCreate(true)}>+ 新建项目</button>
          </div>
        )}

        {activeProjectId && !activeChannelId && (
          <div className={styles.placeholder}>
            <h2>{activeProject?.name}</h2>
            <p>从频道列表选择一个频道开始对话</p>
          </div>
        )}

        {activeChannelId && (
          <>
            <header className={styles.channelHeader}>
              <div>
                <span className={styles.channelHeaderName}>{activeChannel?.name ?? activeChannelId}</span>
                {activeChannel?.description && (
                  <span className={styles.channelHeaderDesc}>{activeChannel.description}</span>
                )}
              </div>
              <span className={styles.channelKindBadge}>
                {activeChannel?.kind === 'ai_development' ? 'AI 开发' : activeChannel?.kind ?? '频道'}
              </span>
            </header>

            <div className={styles.feed} ref={feedRef}>
              {messagesLoading && messages.length === 0 && (
                <p className={styles.feedHint}>正在读取消息…</p>
              )}
              {!messagesLoading && messages.length === 0 && (
                <p className={styles.feedHint}>还没有消息，发送第一条！</p>
              )}
              {messages.map((msg) => (
                <MessageItem
                  key={msg.id}
                  message={msg}
                  isDevChannel={isDevChannel}
                  taskContext={taskContext}
                  user={user}
                  onCancel={cancelTask}
                  onApprove={approveTool}
                />
              ))}
            </div>

            <form className={styles.composer} onSubmit={handleSend}>
              <input
                className={styles.composerInput}
                value={input}
                onChange={(e) => setInput(e.target.value)}
                placeholder={isDevChannel ? '描述开发需求，AI 会帮你实现…' : '发送消息…'}
                disabled={sendingMessage}
              />
              <button
                className={styles.sendBtn}
                type="submit"
                disabled={!input.trim() || sendingMessage}
              >
                {sendingMessage ? '…' : '发送'}
              </button>
            </form>
            {sendError && <p className={styles.sendError}>{sendError}</p>}
          </>
        )}
      </div>

      {showCreate && (
        <CreateProjectModal
          quickMode
          onClose={() => setShowCreate(false)}
          onCreated={async (p) => {
            setShowCreate(false)
            await loadProjects()
            if (p.id) await selectProject(p.id)
          }}
        />
      )}
    </div>
  )
}

/* ── 单条消息渲染 ── */
function MessageItem({ message, isDevChannel, taskContext, user, onCancel, onApprove }: {
  message: Message
  isDevChannel: boolean
  taskContext: ReturnType<typeof buildContext>
  user: { nickname?: string; account?: string } | null
  onCancel: (taskId: string) => Promise<void>
  onApprove: (taskId: string, approvalId: string, decision: 'approve' | 'deny') => Promise<void>
}) {
  const kind = clean(message.kind ?? message.role ?? '').toLowerCase()

  // Dev task 消息用 DevTaskCard 渲染
  if (isDevChannel && ['ai_task', 'ai_progress', 'ai_result'].includes(kind)) {
    return (
      <div className={styles.msgRow}>
        <DevTaskMessage
          message={message}
          context={taskContext}
          onCancel={onCancel}
          onApprove={onApprove}
        />
      </div>
    )
  }

  // 普通聊天消息
  const isUser = kind === 'user' || kind === 'human'
  const content = clean(message.content ?? message.text ?? '')
  const time = message.created_at ? formatTime(message.created_at) : ''
  const displayName = isUser ? (user?.nickname ?? user?.account ?? '我') : 'AI'

  return (
    <div className={[styles.msgRow, isUser ? styles.userMsg : styles.aiMsg].join(' ')}>
      <div className={styles.msgAvatar}>
        {isUser ? ((user?.nickname ?? user?.account)?.[0]?.toUpperCase() ?? '我') : 'AI'}
      </div>
      <div className={styles.msgBody}>
        <div className={styles.msgMeta}>
          <strong>{displayName}</strong>
          {time && <span>{time}</span>}
        </div>
        <div className={styles.msgContent}>{content}</div>
      </div>
    </div>
  )
}
