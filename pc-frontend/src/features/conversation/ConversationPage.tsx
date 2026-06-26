import { useEffect, useRef, useState, useCallback } from 'react'
import { useNavigate } from 'react-router-dom'
import { useProjectStore } from './useProjectStore'
import { useChannelAutoRefresh } from './useChannelAutoRefresh'
import { AttachmentButton, AttachmentChip, attachmentsToMarkdown } from './AttachmentButton'
import type { UploadedAttachment } from './AttachmentButton'
import { useAuthStore } from '../../store/auth'
import { useModelStore } from '../models/useModelStore'
import { ModelPickerPopover } from '../models/ModelPicker'
import { DevTaskMessage } from '../dev/DevTaskCard'
import { buildContext } from '../dev/devTaskUtils'
import { CreateProjectModal } from '../projects/CreateProjectModal'
import MarkdownContent from '../markdown/MarkdownContent'
import { formatTime, clean } from '../../lib/utils'
import { shortButtonLabel } from '../models/modelUtils'
import type { Message } from './types'
import styles from './ConversationPage.module.css'

export default function ConversationPage() {
  useChannelAutoRefresh()
  const navigate = useNavigate()
  const user = useAuthStore((s) => s.user)
  const {
    projects, projectsLoaded, activeProjectId, channels, activeChannelId,
    messages, messagesLoading, sendingMessage,
    loadProjects, selectProject, selectChannel, sendMessage, cancelTask, approveTool,
  } = useProjectStore()
  const selectedAgent = useModelStore((s) => s.selectedAgent)
  const modelLabel = useModelStore((s) => s.label)
  const [input, setInput] = useState('')
  const [sendError, setSendError] = useState('')
  const [showCreate, setShowCreate] = useState(false)
  const [showModelPicker, setShowModelPicker] = useState(false)
  const [channelSearch, setChannelSearch] = useState('')
  const [showNewMsg, setShowNewMsg] = useState(false)
  const [attachments, setAttachments] = useState<UploadedAttachment[]>([])   // P1.4   // P1.3：新消息提示
  const feedRef = useRef<HTMLDivElement>(null)
  const textareaRef = useRef<HTMLTextAreaElement>(null)
  const modelBtnRef = useRef<HTMLButtonElement>(null)
  const atBottomRef = useRef(true)   // P1.3：用户是否在底部

  useEffect(() => { loadProjects() }, [user?.id]) // eslint-disable-line

  // P1.3：智能滚动——只有用户在底部时才自动跟随；否则显示"新消息"按钮
  useEffect(() => {
    const el = feedRef.current
    if (!el) return
    if (atBottomRef.current) {
      el.scrollTop = el.scrollHeight
      setShowNewMsg(false)
    } else {
      setShowNewMsg(true)
    }
  }, [messages])

  // P1.3：检测用户是否滚到底部
  function handleFeedScroll() {
    const el = feedRef.current
    if (!el) return
    atBottomRef.current = el.scrollHeight - el.scrollTop - el.clientHeight < 80
    if (atBottomRef.current) setShowNewMsg(false)
  }

  function scrollToBottom() {
    const el = feedRef.current
    if (!el) return
    el.scrollTop = el.scrollHeight
    atBottomRef.current = true
    setShowNewMsg(false)
  }

  // P1.3：判断是否有运行中任务（用于打字指示器）
  const taskContext = buildContext(messages as Parameters<typeof buildContext>[0])
  const hasRunningTask = (() => {
    const taskIds = new Set<string>()
    const doneIds = new Set<string>()
    for (const m of messages) {
      const kind = ((m.kind ?? m.role ?? '') as string).toLowerCase()
      const id = (m.task_id ?? m.taskId ?? '') as string
      if (!id) continue
      if (kind === 'ai_task') taskIds.add(id)
      if (kind === 'ai_result') doneIds.add(id)
    }
    for (const id of taskIds) if (!doneIds.has(id)) return true
    return false
  })()
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
    if (!text || sendingMessage) return
    setInput('')
    setSendError('')
    setAttachments([])   // P1.4：发送后清空附件
    if (textareaRef.current) { textareaRef.current.style.height = '46px' }
    try {
      // P1.4：附件转为 markdown 追加到消息末尾
      const fullContent = attachments.length > 0
        ? text + attachmentsToMarkdown(attachments)
        : text
      await sendMessage(fullContent, selectedAgent || null)
    } catch (err) {
      setSendError((err as { message?: string }).message ?? '发送失败')
    }
  }

  function handleKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSend(e)
    }
  }

  const activeProject = projects.find((p) => p.id === activeProjectId)
  const activeChannel = channels.find((c) => c.id === activeChannelId)
  const isDevChannel = activeChannel?.kind === 'ai_development'
  // taskContext 和 hasRunningTask 已在上方 P1.3 代码块中定义

  const filteredChannels = channelSearch
    ? channels.filter((c) => c.name.toLowerCase().includes(channelSearch.toLowerCase()))
    : channels

  return (
    <div className={styles.layout}>

      {/* ══ 频道面板（左 304px）══ */}
      <aside className={styles.channelPanel}>
        {/* 工作区标题（58px）*/}
        <div className={styles.workspaceTitle}>
          <div style={{ minWidth: 0 }}>
            <strong className={styles.workspaceTitleText}>
              {activeProject?.name ?? '选择项目'}
            </strong>
            {activeProject?.description && (
              <span className={styles.workspaceTitleMeta}>{activeProject.description}</span>
            )}
          </div>
          <div style={{ display: 'flex', gap: 4, flexShrink: 0 }}>
            {activeProjectId && (
              <button
                className={styles.iconBtn}
                onClick={() => navigate(`/projects/${activeProjectId}`)}
                title="项目设置"
                type="button"
                style={{ fontSize: 14 }}
              >⚙</button>
            )}
            <button className={styles.iconBtn} onClick={() => setShowCreate(true)} title="新建项目" type="button">+</button>
          </div>
        </div>

        {/* 搜索栏（48px）*/}
        <div className={styles.channelSearch}>
          <input
            value={channelSearch}
            onChange={(e) => setChannelSearch(e.target.value)}
            placeholder="搜索频道或项目"
          />
        </div>

        {/* 频道 + 项目列表（1fr）*/}
        <div className={styles.channelList}>
          {/* 我的项目 */}
          <div className={styles.channelSection}>我的项目</div>
          {!projectsLoaded && (
            <div style={{ padding: '6px 9px', color: 'var(--text-muted)', fontSize: 13 }}>读取中…</div>
          )}
          {projects.map((p) => (
            <button
              key={p.id}
              className={[styles.channelItem, p.id === activeProjectId ? styles.channelActive : ''].join(' ')}
              onClick={() => selectProject(p.id)}
              type="button"
            >
              <span className={styles.channelGlyph} style={{ fontSize: 14 }}>
                {p.id === activeProjectId ? '▶' : '▷'}
              </span>
              <span className={styles.channelMain}>
                <strong>{p.name}</strong>
                {p.description && <span>{p.description}</span>}
              </span>
            </button>
          ))}
          {projectsLoaded && projects.length === 0 && (
            <div style={{ padding: '6px 9px', color: 'var(--text-muted)', fontSize: 12 }}>
              暂无项目，点击 + 新建
            </div>
          )}

          {/* 频道列表（选中项目后显示）*/}
          {activeProjectId && filteredChannels.length > 0 && (
            <>
              <div className={styles.channelSection}>频道</div>
              {filteredChannels.map((c) => {
                const isDev = c.kind === 'ai_development'
                return (
                  <button
                    key={c.id}
                    className={[
                      styles.channelItem,
                      isDev ? styles.devChannel : '',
                      c.id === activeChannelId ? styles.channelActive : '',
                    ].join(' ')}
                    onClick={() => selectChannel(c.id)}
                    type="button"
                  >
                    <span className={styles.channelGlyph}>{isDev ? '🛠' : '#'}</span>
                    <span className={styles.channelMain}>
                      <strong>{c.name}</strong>
                      {c.description && <span>{c.description}</span>}
                    </span>
                  </button>
                )
              })}
            </>
          )}
        </div>

        {/* 用户条（64px）*/}
        <div className={styles.userStrip}>
          <button className={styles.userProfileBtn} type="button" title="账号">
            <div className={styles.userDot}>
              {(user?.nickname ?? user?.account)?.[0]?.toUpperCase() ?? '?'}
            </div>
            <div className={styles.userInfo}>
              <strong>{user?.nickname ?? user?.account ?? '未登录'}</strong>
              <span>{user?.account}</span>
            </div>
          </button>
          <div className={styles.userActions}>
            <button
              className={styles.iconBtn}
              onClick={() => useAuthStore.getState().logout()}
              title="退出登录"
              type="button"
            >↩</button>
          </div>
        </div>
      </aside>

      {/* ══ 聊天区（中 1fr）══ */}
      <div className={styles.chatColumn}>
        {/* 顶栏（58px）*/}
        <header className={styles.chatTopbar}>
          <div className={styles.chatTitle}>
            <span className={styles.chatTitleGlyph}>
              {activeChannel?.kind === 'ai_development' ? '🛠' : (activeChannel ? '#' : '💬')}
            </span>
            <div>
              <strong className={styles.chatTitleText}>
                {activeChannel?.name ?? activeProject?.name ?? '选择项目开始对话'}
              </strong>
              {activeChannel?.description && (
                <span className={styles.chatTitleSub}>{activeChannel.description}</span>
              )}
            </div>
          </div>
          <div className={styles.topbarActions}>
            {activeChannelId && (
              <button className={styles.textBtn} type="button" onClick={() => useProjectStore.getState().loadMessages(activeProjectId, activeChannelId)}>
                刷新
              </button>
            )}
          </div>
        </header>

        {/* 消息列表（1fr）*/}
        {!activeChannelId ? (
          <div className={styles.messageList}>
            <div className={styles.emptyState}>
              {!activeProjectId ? (
                <>
                  <strong>欢迎使用一龙工作台</strong>
                  <p>从左侧选择一个项目，或新建一个开始开发。</p>
                  <button className={styles.bigCreateBtn} onClick={() => setShowCreate(true)}>+ 新建项目</button>
                </>
              ) : (
                <>
                  <strong>{activeProject?.name}</strong>
                  <p>从左侧频道列表选择一个频道开始对话。</p>
                </>
              )}
            </div>
          </div>
        ) : (
          <div className={styles.messageList} ref={feedRef} onScroll={handleFeedScroll}>
            {messagesLoading && messages.length === 0 && (
              <div className={styles.emptyState} style={{ marginTop: '4vh' }}>
                <p>正在读取消息…</p>
              </div>
            )}
            {!messagesLoading && messages.length === 0 && (
              <div className={styles.emptyState} style={{ marginTop: '4vh' }}>
                <p>还没有消息，发送第一条吧！</p>
              </div>
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
            {/* P1.3：AI 打字指示器 */}
            {(hasRunningTask || sendingMessage) && (
              <div className={styles.typingRow}>
                <div className={styles.typingAvatar}>AI</div>
                <div className={styles.typingBubble}>
                  <span>AI 正在处理</span>
                  <div className={styles.typingDots}>
                    <span /><span /><span />
                  </div>
                </div>
              </div>
            )}
          </div>
        )}
        {/* P1.3：新消息跳转按钮 */}
        {showNewMsg && activeChannelId && (
          <button className={styles.newMsgBtn} onClick={scrollToBottom} type="button">
            ↓ 新消息
          </button>
        )}

        {/* 输入框（composer）*/}
        {activeChannelId && (
          <form onSubmit={handleSend}>
            {/* P1.4：附件预览条 */}
            {attachments.length > 0 && (
              <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6, padding: '6px 16px 0' }}>
                {attachments.map((att) => (
                  <AttachmentChip
                    key={att.attachment_id}
                    attachment={att}
                    onRemove={() => setAttachments((prev) => prev.filter((a) => a.attachment_id !== att.attachment_id))}
                  />
                ))}
              </div>
            )}
            <div className={styles.composer}>
              {/* 模型选择按钮 */}
              <button
                ref={modelBtnRef}
                className={styles.composerModelBtn}
                type="button"
                title={`AI 模型：${modelLabel || '服务器默认'}`}
                onClick={() => setShowModelPicker((v) => !v)}
              >
                {shortButtonLabel(modelLabel)}
              </button>

              {/* Textarea */}
              <textarea
                ref={textareaRef}
                className={styles.composerTextarea}
                value={input}
                onChange={(e) => { setInput(e.target.value); autoResize() }}
                onKeyDown={handleKeyDown}
                placeholder={
                  isDevChannel
                    ? `向 ${activeChannel?.name ?? 'AI'} 描述开发需求… (Enter 发送，Shift+Enter 换行)`
                    : `在 #${activeChannel?.name ?? ''} 发送消息`
                }
                disabled={sendingMessage}
                rows={1}
              />

              {/* P1.4：附件按钮 */}
              {activeProjectId && (
                <AttachmentButton
                  projectId={activeProjectId}
                  disabled={sendingMessage}
                  onAttached={(att) => setAttachments((prev) => [...prev, att])}
                />
              )}

              {/* 发送按钮 */}
              <button
                className={styles.sendBtn}
                type="submit"
                disabled={(!input.trim() && attachments.length === 0) || sendingMessage}
              >
                {sendingMessage ? '…' : '发送'}
              </button>
            </div>
            {sendError && <p className={styles.sendError}>{sendError}</p>}
          </form>
        )}
      </div>

      {/* ══ 成员面板（右 272px）══ */}
      <aside className={styles.memberPanel}>
        <div className={styles.memberTitle}>
          <span>成员</span>
        </div>
        <div className={styles.memberList}>
          {user && (
            <>
              <div className={styles.memberSection}>在线 · 1</div>
              <div className={styles.memberItem}>
                <div className={styles.memberAvatar}>
                  {(user.nickname ?? user.account)?.[0]?.toUpperCase() ?? '?'}
                </div>
                <span className={styles.memberName}>{user.nickname ?? user.account}</span>
                <span className={styles.presenceDot} />
              </div>
            </>
          )}
        </div>
      </aside>

      {/* 模型选择弹窗 */}
      {showModelPicker && (
        <ModelPickerPopover anchorRef={modelBtnRef} onClose={() => setShowModelPicker(false)} />
      )}

      {/* 新建项目弹窗 */}
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

/* ── 单条消息组件 ── */
function MessageItem({ message, isDevChannel, taskContext, user, onCancel, onApprove }: {
  message: Message
  isDevChannel: boolean
  taskContext: ReturnType<typeof buildContext>
  user: { nickname?: string; account?: string } | null
  onCancel: (id: string) => Promise<void>
  onApprove: (taskId: string, approvalId: string, decision: 'approve' | 'deny') => Promise<void>
}) {
  const kind = clean(message.kind ?? message.role ?? '').toLowerCase()

  // Dev 任务消息用 DevTaskCard 渲染
  if (isDevChannel && ['ai_task', 'ai_progress', 'ai_result'].includes(kind)) {
    return (
      <div className={[styles.messageRow, styles.devTaskWrap].join(' ')}>
        <DevTaskMessage message={message} context={taskContext} onCancel={onCancel} onApprove={onApprove} />
      </div>
    )
  }

  const isUser = kind === 'user' || kind === 'human'
  const isAi = !isUser
  const content = clean(message.content ?? message.text ?? '')
  const time = message.created_at ? formatTime(message.created_at) : ''
  const displayName = isUser ? (user?.nickname ?? user?.account ?? '我') : 'AI'

  // AI 消息：检测是否含 Markdown 特征，有则渲染 Markdown
  const hasMarkdown = isAi && /[#*`\[\]>|]/.test(content)

  return (
    <div className={[styles.messageRow, isUser ? styles.ownRow : ''].join(' ')}>
      <div className={styles.messageAvatar}>
        {isUser
          ? ((user?.nickname ?? user?.account)?.[0]?.toUpperCase() ?? '我')
          : 'AI'}
      </div>
      <div className={styles.messageBody}>
        <div className={styles.messageMeta}>
          <strong>{displayName}</strong>
          {time && <span>{time}</span>}
        </div>
        {hasMarkdown ? (
          <div className={[styles.messageContent, styles.aiContent, styles.markdownMsg].join(' ')}>
            <MarkdownContent content={content} copy />
          </div>
        ) : (
          <div className={[styles.messageContent, isAi ? styles.aiContent : ''].join(' ')}>
            {content}
          </div>
        )}
      </div>
    </div>
  )
}
