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
  const taskContext = buildContext(messages as Parameters<typeof buildContext>[0])

  return (
    <div className={styles.layout}>
      {/* 频道面板（左 304px）*/}
      <aside className={styles.channelPanel}>
        <div className={styles.workspaceTitle}>
          <strong>{activeProject?.name ?? '选择项目'}</strong>
          <button className={styles.newProjectBtn} onClick={() => setShowCreate(true)} title="新建项目">+</button>
        </div>
        <div className={styles.projectList}>
          <div className={styles.sectionLabel}>我的项目</div>
          {!projectsLoaded && <div className={styles.feedHint}>读取中…</div>}
          {projects.map((p) => (
            <button
              key={p.id}
              className={[styles.projectBtn, p.id === activeProjectId ? styles.projectActive : ''].join(' ')}
              onClick={() => selectProject(p.id)}
            >
              <span className={styles.projectIcon}>{p.name?.[0]?.toUpperCase() ?? '?'}</span>
              <span className={styles.projectName}>{p.name}</span>
            </button>
          ))}
          {projectsLoaded && projects.length === 0 && (
            <div className={styles.feedHint} style={{ textAlign: 'left', marginTop: 4 }}>暂无项目</div>
          )}
        </div>
        <div className={styles.channelList}>
          {activeProjectId && <div className={styles.sectionLabel}>频道</div>}
          {channels.map((c) => (
            <button
              key={c.id}
              className={[styles.channelBtn, c.id === activeChannelId ? styles.channelActive : ''].join(' ')}
              onClick={() => selectChannel(c.id)}
            >
              <span className={styles.channelGlyph}>{c.kind === 'ai_development' ? '🛠' : '#'}</span>
              <span className={styles.channelName}>{c.name}</span>
            </button>
          ))}
        </div>
        <div className={styles.userStrip}>
          <div className={styles.userDot}>
            {(user?.nickname ?? user?.account)?.[0]?.toUpperCase() ?? '?'}
          </div>
          <div className={styles.userInfo}>
            <strong>{user?.nickname ?? user?.account ?? '未登录'}</strong>
            <span>{user?.account}</span>
          </div>
        </div>
      </aside>

      {/* 聊天区（中 1fr）*/}
      <div className={styles.chatColumn}>
        <header className={styles.topbar}>
          <span className={styles.topbarGlyph}>{activeChannel?.kind === 'ai_development' ? '🛠' : '#'}</span>
          <span className={styles.topbarTitle}>{activeChannel?.name ?? (activeProject?.name ?? '选择项目开始对话')}</span>
          {activeChannel?.description && <span className={styles.topbarSub}>{activeChannel.description}</span>}
          {activeChannel && (
            <span className={styles.topbarKind}>{activeChannel.kind === 'ai_development' ? 'AI 开发' : '频道'}</span>
          )}
        </header>

        {!activeChannelId ? (
          <div className={styles.placeholder}>
            {!activeProjectId ? (
              <>
                <h2>欢迎回来</h2>
                <p>从左侧选择一个项目，或新建一个</p>
                <button className={styles.bigCreateBtn} onClick={() => setShowCreate(true)}>+ 新建项目</button>
              </>
            ) : (
              <>
                <h2>{activeProject?.name}</h2>
                <p>从左侧频道列表选择一个频道开始对话</p>
              </>
            )}
          </div>
        ) : (
          <div className={styles.feed} ref={feedRef}>
            {messagesLoading && messages.length === 0 && <p className={styles.feedHint}>正在读取消息…</p>}
            {!messagesLoading && messages.length === 0 && <p className={styles.feedHint}>还没有消息，发送第一条！</p>}
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
        )}

        {activeChannelId && (
          <form className={styles.composer} onSubmit={handleSend}>
            <div className={styles.composerWrap}>
              <input
                className={styles.composerInput}
                value={input}
                onChange={(e) => setInput(e.target.value)}
                placeholder={isDevChannel ? `向 ${activeChannel?.name ?? 'AI'} 描述开发需求…` : `在 #${activeChannel?.name ?? ''} 发送消息`}
                disabled={sendingMessage}
              />
            </div>
            <button className={styles.sendBtn} type="submit" disabled={!input.trim() || sendingMessage}>
              {sendingMessage ? '…' : '发送'}
            </button>
          </form>
        )}
        {sendError && <p className={styles.sendError}>{sendError}</p>}
      </div>

      {/* 成员面板（右 272px）*/}
      <aside className={styles.memberPanel}>
        <div className={styles.memberSection}>成员</div>
        {user && (
          <div className={styles.memberItem}>
            <div className={styles.memberDot}>{(user.nickname ?? user.account)?.[0]?.toUpperCase() ?? '?'}</div>
            <span>{user.nickname ?? user.account}</span>
          </div>
        )}
      </aside>

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

function MessageItem({ message, isDevChannel, taskContext, user, onCancel, onApprove }: {
  message: Message
  isDevChannel: boolean
  taskContext: ReturnType<typeof buildContext>
  user: { nickname?: string; account?: string } | null
  onCancel: (id: string) => Promise<void>
  onApprove: (taskId: string, approvalId: string, decision: 'approve' | 'deny') => Promise<void>
}) {
  const kind = clean(message.kind ?? message.role ?? '').toLowerCase()
  if (isDevChannel && ['ai_task', 'ai_progress', 'ai_result'].includes(kind)) {
    return (
      <div className={styles.msgRow}>
        <DevTaskMessage message={message} context={taskContext} onCancel={onCancel} onApprove={onApprove} />
      </div>
    )
  }
  const isUser = kind === 'user' || kind === 'human'
  const content = clean(message.content ?? message.text ?? '')
  const time = message.created_at ? formatTime(message.created_at) : ''
  const displayName = isUser ? (user?.nickname ?? user?.account ?? '我') : 'AI'
  return (
    <div className={styles.msgRow}>
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
