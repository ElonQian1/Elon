import { useEffect, useMemo, useRef, useState, useCallback } from 'react'
import { createPortal } from 'react-dom'
import { useNavigate } from 'react-router-dom'
import { useProjectStore } from './useProjectStore'
import { useChannelAutoRefresh } from './useChannelAutoRefresh'
import { AttachmentButton, AttachmentChip, attachmentsToMarkdown } from './AttachmentButton'
import type { UploadedAttachment } from './AttachmentButton'
import { useAuthStore } from '../../store/auth'
import { useModelStore } from '../models/useModelStore'
import { ModelPickerPopover } from '../models/ModelPicker'
import { DevTaskMessage } from '../dev/DevTaskCard'
import DevTaskGroup from '../dev/DevTaskGroup'
import { buildContext } from '../dev/devTaskUtils'
import { CreateProjectModal } from '../projects/CreateProjectModal'
import ProjectLanding from './ProjectLanding'
import NodeOfflineBanner from './NodeOfflineBanner'
import { api } from '../../api/client'
import MarkdownContent from '../markdown/MarkdownContent'
import { formatTime, clean } from '../../lib/utils'
import { shortButtonLabel } from '../models/modelUtils'
import type {
  Channel,
  ChannelCategory,
  ChannelPermissionResponse,
  Message,
  PermissionOption,
  PermissionOverride,
  ProjectInviteLink,
  ProjectInviteLinksResponse,
  ProjectInvitePreview,
  ProjectInvitePreviewResponse,
  ProjectInviteResponse,
  ProjectMember,
  ProjectRole,
  ProjectRolesResponse,
  UserPresenceSettings,
} from './types'
import styles from './ConversationPage.module.css'

export default function ConversationPage() {
  useChannelAutoRefresh()
  const navigate = useNavigate()
  const user = useAuthStore((s) => s.user)
  const token = useAuthStore((s) => s.token)
  const {
    projects, projectsLoaded, activeProjectId, channels, categories, members, activeChannelId,
    messages, messagesLoading, sendingMessage, landing,
    loadProjects, selectProject, reloadProjectSpace, selectChannel, sendMessage, cancelTask, approveTool,
  } = useProjectStore()
  const selectedAgent = useModelStore((s) => s.selectedAgent)
  const modelLabel = useModelStore((s) => s.label)
  const [input, setInput] = useState('')
  const [sendError, setSendError] = useState('')
  const [showCreate, setShowCreate] = useState(false)
  const [showModelPicker, setShowModelPicker] = useState(false)
  const [showPermissions, setShowPermissions] = useState(false)
  const [showPresence, setShowPresence] = useState(false)
  const [showInvites, setShowInvites] = useState(false)
  const [showModeration, setShowModeration] = useState(false)
  const [selectedMember, setSelectedMember] = useState<ProjectMember | null>(null)
  const [memberPopoverY, setMemberPopoverY] = useState(200)
  const [inviteCode, setInviteCode] = useState('')
  const [invitePreview, setInvitePreview] = useState<ProjectInvitePreview | null>(null)
  const [inviteStatus, setInviteStatus] = useState('')
  const [channelSearch, setChannelSearch] = useState('')
  const [showNewMsg, setShowNewMsg] = useState(false)
  const [attachments, setAttachments] = useState<UploadedAttachment[]>([])   // P1.4   // P1.3：新消息提示
  const feedRef = useRef<HTMLDivElement>(null)
  const textareaRef = useRef<HTMLTextAreaElement>(null)
  const modelBtnRef = useRef<HTMLButtonElement>(null)
  const atBottomRef = useRef(true)   // P1.3：用户是否在底部

  useEffect(() => { loadProjects() }, [user?.id]) // eslint-disable-line

  useEffect(() => {
    setSelectedMember(null)
  }, [activeProjectId, activeChannelId])

  useEffect(() => {
    const params = new URLSearchParams(window.location.search)
    const code = clean(params.get('invite') ?? '')
    if (!code) return
    setInviteCode(code)
    setInviteStatus('读取邀请中…')
    api.get<ProjectInvitePreviewResponse>(`/api/project-invites/${encodeURIComponent(code)}`)
      .then((data) => {
        setInvitePreview(data.invite ?? null)
        setInviteStatus('')
      })
      .catch((err: { message?: string }) => {
        setInvitePreview(null)
        setInviteStatus(err.message ?? '邀请链接不可用')
      })
  }, [])

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
      // 项目首页发送：没有选中频道时，自动选择最佳频道（ai_development > 第一个）
      if (!activeChannelId && channels.length > 0) {
        const best = channels.find((c) => c.kind === 'ai_development') ?? channels[0]
        await selectChannel(best.id)
      }
      // 从 landing 首页发送时，标记等待新会话出现后自动切入
      if (sessionView === null || sessionView === undefined) {
        prevSessionIdsRef.current = new Set(sessions.map((s) => s.id))
        waitingForNewSession.current = true
      }
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

  async function acceptInvite() {
    if (!inviteCode) return
    setInviteStatus('加入中…')
    try {
      const data = await api.post<{ project_id?: string; invite?: ProjectInvitePreview }>(
        `/api/project-invites/${encodeURIComponent(inviteCode)}/join`,
        {},
      )
      const projectId = data.project_id ?? data.invite?.project_id
      setInviteStatus('已加入')
      setInvitePreview(null)
      setInviteCode('')
      const url = new URL(window.location.href)
      url.searchParams.delete('invite')
      window.history.replaceState({}, '', url.toString())
      await loadProjects()
      if (projectId) await selectProject(projectId)
    } catch (err) {
      setInviteStatus((err as { message?: string }).message ?? '加入失败')
    }
  }

  const activeProject = projects.find((p) => p.id === activeProjectId)
  const activeChannel = channels.find((c) => c.id === activeChannelId)
  const isDevChannel = activeChannel?.kind === 'ai_development'
  const canManagePermissions = channels.some(channelCanManage)
  // taskContext 和 hasRunningTask 已在上方 P1.3 代码块中定义

  const filteredChannels = channelSearch
    ? channels.filter((c) => c.name.toLowerCase().includes(channelSearch.toLowerCase()))
    : channels

  // 成员列表：从 project space 读取
  const spaceMembers = members
  const memberPanelTitle = activeChannel ? '频道成员' : activeProjectId ? '项目成员' : '工作台'
  const memberPanelContext = activeChannel?.name ?? activeProject?.name ?? '我的项目'
  const memberPanelCount = activeProjectId ? spaceMembers.length : (user ? 1 : 0)

  // 成员卡片弹窗
  // (memberPopover state removed - not currently used)

  // 消息分组：判断某条消息是否与上一条来自同一发送者（仅用于非任务消息）
  function isGrouped(idx: number): boolean {
    if (idx === 0) return false
    const cur  = messages[idx]
    const prev = messages[idx - 1]
    const curRole  = clean(cur.kind  ?? cur.role  ?? '').toLowerCase()
    const prevRole = clean(prev.kind ?? prev.role ?? '').toLowerCase()
    const curId  = clean(cur.user_id  ?? (cur as Record<string, unknown>).userId  ?? '')
    const prevId = clean(prev.user_id ?? (prev as Record<string, unknown>).userId ?? '')
    if (['ai_task','ai_progress','ai_result'].includes(curRole)) return false
    if (['ai_task','ai_progress','ai_result'].includes(prevRole)) return false
    if (curRole === prevRole) {
      if (curRole === 'user' || curRole === 'human') return curId !== '' && curId === prevId
      return true
    }
    return false
  }

  // 会话视图模式：null=默认(全部) / 'new'=新建空会话 / string=只看该task_id
  const [sessionView, setSessionView] = useState<string | 'new' | null>(null)
  const prevSessionIdsRef = useRef<Set<string>>(new Set())
  const waitingForNewSession = useRef(false)

  // 根据会话视图过滤显示的消息（必须在 messageGroups 之前声明）
  const displayMessages = useMemo(() => {
    if (!sessionView) return messages
    if (sessionView === 'new') return []
    return messages.filter((msg) => {
      const tid = String((msg.task_id ?? (msg as Record<string, unknown>).taskId) ?? '')
      return tid === sessionView
    })
  }, [messages, sessionView])

  // 消息分组：dev频道中把同一 task_id 的消息聚合为 DevTaskGroup（任务级折叠层）
  type SingleGroup = { type: 'single'; msg: Message; grouped: boolean; key: string }
  type TaskGroup   = { type: 'task';   taskId: string; msgs: Message[]; key: string }
  const messageGroups = useMemo(() => {
    const src = displayMessages
    const groups: Array<SingleGroup | TaskGroup> = []
    for (let i = 0; i < src.length; i++) {
      const msg  = src[i]
      const kind = clean(msg.kind ?? msg.role ?? '').toLowerCase()
      const tid  = String((msg.task_id ?? (msg as Record<string, unknown>).taskId) ?? '')
      const isTask = isDevChannel && ['ai_task','ai_progress','ai_result'].includes(kind) && !!tid
      if (isTask) {
        const last = groups[groups.length - 1]
        if (last?.type === 'task' && last.taskId === tid) last.msgs.push(msg)
        else groups.push({ type: 'task', taskId: tid, msgs: [msg], key: `task-${tid}-${i}` })
      } else {
        groups.push({ type: 'single', msg, grouped: isGrouped(i), key: msg.id ?? String(i) })
      }
    }
    return groups
  }, [displayMessages, isDevChannel]) // eslint-disable-line

  // 会话列表：从已加载消息中提取（每个 task_id = 一个会话）
  const sessions = useMemo(() => {
    const taskOrder = new Map<string, number>()
    messages.forEach((msg, i) => {
      const tid = String((msg.task_id ?? (msg as Record<string, unknown>).taskId) ?? '')
      if (tid && !taskOrder.has(tid)) taskOrder.set(tid, i)
    })
    const list: Array<{ id: string; title: string; done: boolean; failed: boolean; steps: number }> = []
    taskContext.tasks.forEach((task, taskId) => {
      list.push({
        id: taskId,
        title: (task.request ?? '').slice(0, 40) || '新会话',
        done: !!task.result,
        failed: task.failed || task.canceled,
        steps: task.progressCount,
      })
    })
    return list.sort((a, b) => (taskOrder.get(b.id) ?? 0) - (taskOrder.get(a.id) ?? 0))
  }, [taskContext, messages])

  // sessionView='new' 时，一旦出现新 task_id 自动切到它
  useEffect(() => {
    if (!waitingForNewSession.current) return
    const newSession = sessions.find((s) => !prevSessionIdsRef.current.has(s.id))
    if (newSession) {
      setSessionView(newSession.id)
      waitingForNewSession.current = false
    }
  }, [sessions])

  // 切换频道时重置会话视图
  useEffect(() => {
    setSessionView(null)
    waitingForNewSession.current = false
  }, [activeChannelId]) // eslint-disable-line

  function startNewSession() {
    prevSessionIdsRef.current = new Set(sessions.map((s) => s.id))
    setSessionView('new')
    waitingForNewSession.current = true
    setTimeout(() => textareaRef.current?.focus(), 50)
  }

  function openSession(taskId: string) {
    setSessionView(taskId)
  }

  return (
    <div className={styles.layout}>

      {/* ══ 频道面板（左 304px）══ */}
      <aside className={styles.channelPanel}>
        {/* 工作区标题（58px）*/}
        <div className={styles.workspaceTitle}>
          {activeProjectId ? (
            /* 项目视图：显项目名，点击返回项目列表 */
            <>
              <button
                className={styles.workspaceBackBtn}
                onClick={() => useProjectStore.getState().selectProject('')}
                title="返回项目列表"
                type="button"
              >←</button>
              <div style={{ minWidth: 0, flex: 1 }}>
                <strong className={styles.workspaceTitleText}>{activeProject?.name}</strong>
                {activeProject?.description && (
                  <span className={styles.workspaceTitleMeta}>{activeProject.description}</span>
                )}
              </div>
              <button
                className={styles.iconBtn}
                onClick={() => navigate(`/projects/${activeProjectId}`)}
                title="项目设置"
                type="button"
                style={{ fontSize: 14 }}
              >⚙</button>
            </>
          ) : (
            /* 项目列表视图：显我的项目标题 */
            <>
              <div style={{ minWidth: 0, flex: 1 }}>
                <strong className={styles.workspaceTitleText}>我的项目</strong>
              </div>
              <button className={styles.iconBtn} onClick={() => setShowCreate(true)} title="新建项目" type="button">+</button>
            </>
          )}
        </div>

        {/* 搜索栏（48px）*/}
        <div className={styles.channelSearch}>
          <input
            value={channelSearch}
            onChange={(e) => setChannelSearch(e.target.value)}
            placeholder={activeProjectId ? '搜索频道' : '搜索项目'}
          />
        </div>

        {/* 内容区：根据是否有选中项目切换两种视图 */}
        <div className={styles.channelList}>
          {activeProjectId ? (
            /* —— Discord 式：只显当前项目的频道 + 会话列表 —— */
            <>
              {filteredChannels.length === 0 ? (
                <div style={{ padding: '12px 16px', color: 'var(--text-muted)', fontSize: 13 }}>
                  还没有频道
                </div>
              ) : (
                filteredChannels.map((c) => {
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
                })
              )}

              {/* ── 会话列表：选中频道时显示，点击进入隔离语境运行 ── */}
              {activeChannelId && (
                <div style={{ borderTop: '1px solid rgba(255,255,255,.04)', marginTop: 4, paddingBottom: 4 }}>
                  <div style={{ padding: '8px 12px 3px', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                    <span style={{ fontSize: 11, fontWeight: 800, textTransform: 'uppercase', letterSpacing: '.05em', color: 'var(--text-muted)' }}>会话</span>
                    <button
                      type="button"
                      title="新建会话"
                      onClick={startNewSession}
                      style={{ background: sessionView === 'new' ? 'rgba(60,111,162,.3)' : 'rgba(255,255,255,.08)', border: 'none', borderRadius: 4, width: 18, height: 18, color: 'var(--text-soft)', fontSize: 14, lineHeight: 1, cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'center', padding: 0 }}
                    >+</button>
                  </div>
                  {sessions.length === 0 && (
                    <div style={{ padding: '4px 12px 6px', fontSize: 11, color: 'var(--text-muted)' }}>发送第一条消息自动创建会话</div>
                  )}
                  {sessions.map((s) => (
                    <button
                      key={s.id}
                      type="button"
                      onClick={() => openSession(s.id)}
                      style={{
                        width: 'calc(100% - 8px)', display: 'flex', flexDirection: 'column', gap: 1,
                        padding: '5px 10px', margin: '1px 4px',
                        background: s.id === sessionView ? 'rgba(60,111,162,.2)' : 'transparent',
                        border: 'none', borderRadius: 5, textAlign: 'left', cursor: 'pointer',
                        color: 'var(--text-soft)', transition: 'background .1s',
                      }}
                    >
                      <span style={{ fontSize: 12, fontWeight: 500, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', display: 'block' }}>
                        {s.failed ? '✗ ' : s.done ? '✓ ' : '⟳ '}{s.title}
                      </span>
                      {s.steps > 0 && (
                        <span style={{ fontSize: 10, color: 'var(--text-muted)' }}>{s.steps} 步</span>
                      )}
                    </button>
                  ))}
                </div>
              )}
            </>
          ) : (
            <>
              {!projectsLoaded && (
                <div style={{ padding: '6px 9px', color: 'var(--text-muted)', fontSize: 13 }}>读取中…</div>
              )}
              {projects
                .filter(p => !channelSearch || p.name.toLowerCase().includes(channelSearch.toLowerCase()))
                .map((p) => (
                  <button
                    key={p.id}
                    className={styles.channelItem}
                    onClick={() => selectProject(p.id)}
                    type="button"
                  >
                    <span className={styles.channelGlyph}>
                      {p.icon_data_url || p.icon
                        ? <img src={p.icon_data_url || p.icon} alt="" style={{ width: 20, height: 20, borderRadius: 4, objectFit: 'cover' }} />
                        : '📦'
                      }
                    </span>
                    <span className={styles.channelMain}>
                      <strong>{p.name}</strong>
                      {p.description && <span>{p.description}</span>}
                    </span>
                  </button>
                ))
              }
              {projectsLoaded && projects.length === 0 && (
                <div style={{ padding: '6px 9px', color: 'var(--text-muted)', fontSize: 12 }}>
                  暂无项目，点击 + 新建
                </div>
              )}
            </>
          )}
        </div>

        {/* 用户条（64px）*/}
        <div className={styles.userStrip}>
          <button
            className={styles.userProfileBtn}
            type="button"
            title="账号设置"
            onClick={() => navigate('/account')}
          >
            <div className={styles.userDot}>
              {(user?.nickname ?? user?.account)?.[0]?.toUpperCase() ?? '?'}
            </div>
            <div className={styles.userInfo}>
              <strong>{user?.nickname ?? user?.account ?? (token ? '加载中…' : '未登录')}</strong>
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
            <button className={styles.textBtn} type="button"
              title="分享这台电脑的算力并查看连接状态"
              onClick={() => navigate('/node')}>
              分享算力
            </button>
            <button className={styles.textBtn} type="button"
              title="打开移动端入口"
              onClick={() => window.open('/app/download', '_blank', 'noopener')}>
              打开移动端
            </button>
            <button className={styles.textBtn} type="button"
              title="切换到旧版 PC 工作台"
              onClick={() => {
                const tok = useAuthStore.getState().token
                if (tok) {
                  localStorage.setItem('lodex_token', tok)
                  localStorage.setItem('elon_token', tok)
                }
                window.open('/pc-legacy', '_blank', 'noopener')
              }}>
              旧版
            </button>
          </div>
        </header>

        {/* 节点离线提示：电脑重启后节点未运行时出现 */}
        {activeProjectId && <NodeOfflineBanner />}

        {/* 消息列表（1fr）*/}
        {/* 无频道或未选中会话（landing）vs 选中会话（feed）*/}
        {!activeChannelId || sessionView === null ? (
          <div className={styles.messageList}>
            {!activeProjectId ? (
              /* 无项目：全局欢迎页 */
              <div className={styles.emptyState}>
                <strong>欢迎使用一龙工作台</strong>
                <p>从左侧选择一个项目，或新建一个开始开发。</p>
                <button className={styles.bigCreateBtn} onClick={() => setShowCreate(true)}>+ 新建项目</button>
              </div>
            ) : (
              /* 项目首页：富内容 landing（与旧版 pc_project_landing.js 功能对等）*/
              activeProject && (
                <ProjectLanding
                  project={activeProject}
                  channels={channels}
                  landing={landing}
                  onSelectChannel={(id) => { setSessionView(null); selectChannel(id) }}
                />
              )
            )}
          </div>
        ) : (
          <div className={styles.messageList} ref={feedRef} onScroll={handleFeedScroll}>
            {messagesLoading && messages.length === 0 && (
              <div className={styles.emptyState} style={{ marginTop: '4vh' }}>
                <p>正在读取消息…</p>
              </div>
            )}
            {!messagesLoading && displayMessages.length === 0 && (
              <div className={styles.emptyState} style={{ marginTop: '4vh' }}>
                {sessionView === 'new'
                  ? <><strong>新会话</strong><p>输入消息开始全新对话，发送后自动保存为独立会话。</p></>
                  : <p>还没有消息，发送第一条吧！</p>
                }
              </div>
            )}
            {displayMessages.length > 0 && messageGroups.map((group) =>
              group.type === 'task' ? (
                <div key={group.key} data-task-id={group.taskId} className={styles.devTaskWrap}>
                  <DevTaskGroup
                    messages={group.msgs as Parameters<typeof DevTaskGroup>[0]['messages']}
                    taskContext={taskContext}
                    onCancel={cancelTask}
                    onApprove={approveTool}
                  />
                </div>
              ) : (
                <MessageItem
                  key={group.key}
                  message={group.msg}
                  isDevChannel={isDevChannel}
                  taskContext={taskContext}
                  user={user}
                  onCancel={cancelTask}
                  onApprove={approveTool}
                  grouped={group.grouped}
                />
              )
            )}
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

        {/* 输入框（composer）——项目开启时始终可见 */}
        {activeProjectId && (
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
                  !activeChannelId
                    ? `向 ${activeProject?.name ?? '项目'} 发送消息或需求… (Enter 发送)`
                    : isDevChannel
                      ? `向 ${activeChannel?.name ?? 'AI'} 描述开发需求… (Enter 发送，Shift+Enter 换行)`
                      : `在 #${activeChannel?.name ?? ''} 发送消息`
                }
                disabled={sendingMessage || (!activeChannelId && channels.length === 0)}
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
          <div className={styles.memberTitleCopy}>
            <strong>{memberPanelTitle}{memberPanelCount > 0 ? ` — ${memberPanelCount}` : ''}</strong>
            <span>{memberPanelContext}</span>
          </div>
          <div className={styles.memberActions}>
            <button className={styles.memberInviteBtn} type="button" onClick={() => setShowPresence(true)}>状态</button>
            {activeProjectId && <button className={styles.memberInviteBtn} type="button" onClick={() => setShowInvites(true)}>邀请</button>}
            {activeProjectId && <button className={styles.memberInviteBtn} type="button" onClick={() => setShowModeration(true)}>管理</button>}
            {activeProjectId && activeChannelId && canManagePermissions && (
              <button className={styles.memberInviteBtn} type="button" onClick={() => setShowPermissions(true)}>权限</button>
            )}
          </div>
        </div>
        <div className={styles.memberList}>
          {selectedMember && createPortal(
            <MemberProfilePopover
              member={selectedMember}
              anchorY={memberPopoverY}
              onClose={() => setSelectedMember(null)}
            />,
            document.body
          )}
          {activeProjectId && messagesLoading && spaceMembers.length === 0 && (
            <MemberLoadingRows />
          )}
          {activeProjectId && spaceMembers.length > 0 && (
            <MemberSearch members={spaceMembers} onSelect={(m, y) => { setSelectedMember(m); setMemberPopoverY(y) }} />
          )}
          {activeProjectId && !messagesLoading && spaceMembers.length === 0 && (
            <p className={styles.sideHint}>暂无项目成员</p>
          )}
          {!activeProjectId && user && (
            <>
              <div className={styles.memberSection}>当前账号</div>
              <div className={styles.memberItem}>
                <div className={[styles.memberAvatar, styles.memberAvatarOnline].join(' ')}>
                  {(user.nickname ?? user.account)?.[0]?.toUpperCase() ?? '?'}
                </div>
                <div className={styles.memberCopy}>
                  <div className={styles.memberLine}>
                    <strong className={styles.memberItemName}>{user.nickname ?? user.account}</strong>
                  </div>
                  <span className={styles.memberSub}>在线</span>
                </div>
              </div>
            </>
          )}
        </div>
      </aside>

      {(invitePreview || inviteStatus) && inviteCode && (
        <div className={styles.inviteBanner}>
          <div>
            <strong>{invitePreview ? inviteTitle(invitePreview) : '邀请链接'}</strong>
            <span>{inviteStatus || `将以 ${roleLabel(invitePreview?.role ?? 'member')} 身份加入`}</span>
          </div>
          <button className={styles.primaryBtn} onClick={acceptInvite} disabled={!invitePreview || inviteStatus === '加入中…'}>加入</button>
          <button className={styles.drawerCloseBtn} onClick={() => {
            setInvitePreview(null)
            setInviteStatus('')
            setInviteCode('')
          }}>关闭</button>
        </div>
      )}

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

      {showPresence && (
        <PresenceDrawer onClose={() => setShowPresence(false)} onSaved={reloadProjectSpace} />
      )}
      {showInvites && activeProjectId && (
        <InviteDrawer projectId={activeProjectId} onClose={() => setShowInvites(false)} />
      )}
      {showModeration && activeProjectId && (
        <ModerationDrawer projectId={activeProjectId} members={members} onClose={() => setShowModeration(false)} onSaved={reloadProjectSpace} />
      )}
      {showPermissions && activeProjectId && activeChannelId && (
        <PermissionDrawer
          projectId={activeProjectId}
          activeChannelId={activeChannelId}
          channels={channels}
          categories={categories}
          members={members}
          onClose={() => setShowPermissions(false)}
          onSaved={reloadProjectSpace}
        />
      )}
    </div>
  )
}

const CHANNEL_PERMISSION_OPTIONS: PermissionOption[] = [
  { key: 'view_channel', label: '查看频道' },
  { key: 'send_messages', label: '发送消息' },
  { key: 'start_ai_tasks', label: '发起 AI 任务' },
  { key: 'manage_channel', label: '管理频道权限' },
]

function channelCanManage(channel: Channel) {
  const permissions = channel.permissions ?? {}
  return !!(permissions.can_manage || permissions.canManage)
}

function filterMembers(members: ProjectMember[], query: string) {
  const needle = clean(query).toLowerCase()
  if (!needle) return members
  return members.filter((member) => {
    const haystack = [
      member.account,
      member.user_id,
      member.role,
      member.custom_status,
      member.activity,
      ...(member.roles ?? []).map((role) => role.name || role.id),
    ].join(' ').toLowerCase()
    return haystack.includes(needle)
  })
}

function memberInitial(member: ProjectMember) {
  return clean(member.account ?? member.user_id).slice(0, 1).toUpperCase() || '员'
}

function memberSubtitle(member: ProjectMember) {
  if (member.is_banned) return '已封禁'
  if (member.is_muted) return `禁言至 ${formatDateTime(member.muted_until)}`
  const activity = clean(member.activity ?? '')
  const customStatus = clean(member.custom_status ?? '')
  if (activity) return activity
  if (customStatus) return customStatus
  return memberRoleSummary(member)
}

function memberPresenceStatus(member: ProjectMember) {
  const status = clean(member.presence_status ?? '').toLowerCase()
  if (!member.is_online || status === 'offline' || status === 'invisible') return 'offline'
  if (status === 'idle' || status === 'dnd') return status
  return 'online'
}

function presenceLabel(status: string) {
  const labels: Record<string, string> = {
    online: '在线',
    idle: '离开',
    dnd: '勿扰',
    invisible: '隐身',
    offline: '离线',
  }
  return labels[status] ?? status
}

function inviteTitle(invite: ProjectInvitePreview) {
  return invite.display_name || invite.project_name || '项目邀请'
}

function formatDateTime(value?: string | null) {
  if (!value) return '无限期'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return date.toLocaleString()
}

function memberRoleSummary(member: ProjectMember) {
  const roles = member.roles ?? []
  if (roles.length) return roles.map((role) => role.name || role.id).join(' / ')
  return roleLabel(member.role ?? 'member')
}

function roleLabel(role: string) {
  const labels: Record<string, string> = {
    owner: '拥有者',
    admin: '管理员',
    editor: '协作者',
    developer: '开发者',
    maintainer: '维护者',
    member: '成员',
    observer: '只读成员',
  }
  return labels[role] ?? role
}

const PRESENCE_OPTIONS = [
  { value: 'online', label: '在线' },
  { value: 'idle', label: '离开' },
  { value: 'dnd', label: '勿扰' },
  { value: 'invisible', label: '隐身' },
]

function PresenceDrawer({
  onClose,
  onSaved,
}: {
  onClose: () => void
  onSaved: () => Promise<void>
}) {
  const [status, setStatus] = useState('online')
  const [customStatus, setCustomStatus] = useState('')
  const [activity, setActivity] = useState('')
  const [message, setMessage] = useState('')
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    setMessage('读取中…')
    api.get<UserPresenceSettings>('/api/me/presence')
      .then((data) => {
        setStatus(data.status || 'online')
        setCustomStatus(data.custom_status ?? '')
        setActivity(data.activity ?? '')
        setMessage('')
      })
      .catch((err: { message?: string }) => setMessage(err.message ?? '状态读取失败'))
  }, [])

  async function save() {
    setSaving(true)
    setMessage('保存中…')
    try {
      const data = await api.patch<UserPresenceSettings>('/api/me/presence', {
        status,
        custom_status: customStatus,
        activity,
      })
      setStatus(data.status || status)
      setCustomStatus(data.custom_status ?? '')
      setActivity(data.activity ?? '')
      setMessage('已保存')
      await onSaved()
    } catch (err) {
      setMessage((err as { message?: string }).message ?? '保存失败')
    } finally {
      setSaving(false)
    }
  }

  return (
    <div className={styles.drawerBackdrop}>
      <section className={[styles.permissionDrawer, styles.compactDrawer].join(' ')} role="dialog" aria-modal="true">
        <header className={styles.drawerHeader}>
          <div>
            <strong>在线状态</strong>
            <span>{message}</span>
          </div>
          <button className={styles.drawerCloseBtn} onClick={onClose}>关闭</button>
        </header>
        <div className={styles.drawerBody}>
          <label className={styles.field}>
            <span>展示状态</span>
            <select value={status} onChange={(event) => setStatus(event.target.value)}>
              {PRESENCE_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>{option.label}</option>
              ))}
            </select>
          </label>
          <label className={styles.field}>
            <span>自定义状态</span>
            <input value={customStatus} onChange={(event) => setCustomStatus(event.target.value)} maxLength={80} placeholder="例如：写代码中" />
          </label>
          <label className={styles.field}>
            <span>正在做</span>
            <input value={activity} onChange={(event) => setActivity(event.target.value)} maxLength={80} placeholder="例如：调试 PC 网页版" />
          </label>
          <div className={styles.actionRow}>
            <button className={styles.primaryBtn} onClick={save} disabled={saving}>保存</button>
          </div>
        </div>
      </section>
    </div>
  )
}

function InviteDrawer({
  projectId,
  onClose,
}: {
  projectId: string
  onClose: () => void
}) {
  const [invites, setInvites] = useState<ProjectInviteLink[]>([])
  const [role, setRole] = useState('member')
  const [expiresInHours, setExpiresInHours] = useState('168')
  const [maxUses, setMaxUses] = useState('')
  const [temporary, setTemporary] = useState(false)
  const [message, setMessage] = useState('')
  const [loading, setLoading] = useState(false)

  async function refreshInvites() {
    setLoading(true)
    try {
      const data = await api.get<ProjectInviteLinksResponse>(`/api/projects/${encodeURIComponent(projectId)}/invite-links`)
      setInvites(data.invites ?? [])
      setMessage('')
    } catch (err) {
      setMessage((err as { message?: string }).message ?? '邀请链接读取失败')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    refreshInvites()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId])

  async function createInvite() {
    setMessage('创建中…')
    try {
      const data = await api.post<ProjectInviteResponse>(`/api/projects/${encodeURIComponent(projectId)}/invite-links`, {
        role,
        expires_in_hours: numberOrUndefined(expiresInHours),
        max_uses: numberOrUndefined(maxUses),
        temporary,
      })
      if (data.invite) setInvites((items) => [data.invite as ProjectInviteLink, ...items])
      setMessage('已创建')
    } catch (err) {
      setMessage((err as { message?: string }).message ?? '创建失败')
    }
  }

  async function revokeInvite(code: string) {
    setMessage('撤销中…')
    try {
      await api.delete<ProjectInviteResponse>(`/api/projects/${encodeURIComponent(projectId)}/invite-links/${encodeURIComponent(code)}`)
      await refreshInvites()
      setMessage('已撤销')
    } catch (err) {
      setMessage((err as { message?: string }).message ?? '撤销失败')
    }
  }

  async function copyInvite(code: string) {
    try {
      await navigator.clipboard.writeText(inviteUrl(code))
      setMessage('已复制邀请链接')
    } catch {
      setMessage(inviteUrl(code))
    }
  }

  return (
    <div className={styles.drawerBackdrop}>
      <section className={[styles.permissionDrawer, styles.inviteDrawer].join(' ')} role="dialog" aria-modal="true">
        <header className={styles.drawerHeader}>
          <div>
            <strong>邀请链接</strong>
            <span>{loading ? '同步中…' : message}</span>
          </div>
          <button className={styles.drawerCloseBtn} onClick={onClose}>关闭</button>
        </header>
        <div className={styles.drawerBody}>
          <section className={styles.drawerSection}>
            <div className={styles.formGrid}>
              <label className={styles.field}>
                <span>加入角色</span>
                <input value={role} onChange={(event) => setRole(event.target.value)} placeholder="member" />
              </label>
              <label className={styles.field}>
                <span>有效小时</span>
                <input value={expiresInHours} onChange={(event) => setExpiresInHours(event.target.value)} inputMode="numeric" placeholder="空为永久" />
              </label>
              <label className={styles.field}>
                <span>最大次数</span>
                <input value={maxUses} onChange={(event) => setMaxUses(event.target.value)} inputMode="numeric" placeholder="空为不限" />
              </label>
              <label className={styles.checkField}>
                <input type="checkbox" checked={temporary} onChange={(event) => setTemporary(event.target.checked)} />
                <span>临时邀请</span>
              </label>
            </div>
            <div className={styles.actionRow}>
              <button className={styles.primaryBtn} onClick={createInvite}>创建链接</button>
            </div>
          </section>

          <section className={styles.drawerSection}>
            <strong className={styles.sectionTitle}>已创建</strong>
            <div className={styles.inviteList}>
              {invites.length === 0 && <p className={styles.sideHint}>暂无邀请链接</p>}
              {invites.map((invite) => (
                <article key={invite.id} className={styles.inviteRow}>
                  <div>
                    <strong>{inviteUrl(invite.code)}</strong>
                    <span>
                      {roleLabel(invite.role)} · {invite.use_count}/{invite.max_uses ?? '不限'} · {invite.revoked_at ? '已撤销' : invite.expires_at ? `过期 ${formatDateTime(invite.expires_at)}` : '永久'}
                    </span>
                  </div>
                  <button className={styles.drawerCloseBtn} onClick={() => copyInvite(invite.code)}>复制</button>
                  <button className={styles.dangerBtn} onClick={() => revokeInvite(invite.code)} disabled={!!invite.revoked_at}>撤销</button>
                </article>
              ))}
            </div>
          </section>
        </div>
      </section>
    </div>
  )
}

function ModerationDrawer({
  projectId,
  members,
  onClose,
  onSaved,
}: {
  projectId: string
  members: ProjectMember[]
  onClose: () => void
  onSaved: () => Promise<void>
}) {
  const [query, setQuery] = useState('')
  const [message, setMessage] = useState('')
  const visibleMembers = useMemo(() => filterMembers(members, query), [members, query])

  async function moderate(member: ProjectMember, action: 'mute' | 'unmute' | 'ban' | 'unban', durationMinutes?: number) {
    setMessage('提交中…')
    try {
      await api.patch(`/api/projects/${encodeURIComponent(projectId)}/members/${encodeURIComponent(member.user_id)}/moderation`, {
        action,
        duration_minutes: durationMinutes,
        note: 'PC 成员管理页操作',
      })
      setMessage('已更新')
      await onSaved()
    } catch (err) {
      setMessage((err as { message?: string }).message ?? '操作失败')
    }
  }

  return (
    <div className={styles.drawerBackdrop}>
      <section className={[styles.permissionDrawer, styles.moderationDrawer].join(' ')} role="dialog" aria-modal="true">
        <header className={styles.drawerHeader}>
          <div>
            <strong>禁言与封禁</strong>
            <span>{message}</span>
          </div>
          <button className={styles.drawerCloseBtn} onClick={onClose}>关闭</button>
        </header>
        <div className={styles.drawerBody}>
          <input className={styles.drawerSearchInput} value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索成员" />
          <div className={styles.moderationList}>
            {visibleMembers.map((member) => (
              <article key={member.user_id} className={styles.moderationRow}>
                <span className={styles.memberAvatar}>{memberInitial(member)}</span>
                <div className={styles.moderationInfo}>
                  <strong>{member.account || member.user_id}</strong>
                  <span>{memberModerationSummary(member)}</span>
                </div>
                <div className={styles.moderationActions}>
                  <button className={styles.drawerCloseBtn} onClick={() => moderate(member, 'mute', 60)}>禁言1小时</button>
                  <button className={styles.drawerCloseBtn} onClick={() => moderate(member, 'mute', 1440)}>禁言1天</button>
                  <button className={styles.drawerCloseBtn} onClick={() => moderate(member, 'unmute')} disabled={!member.is_muted}>解禁言</button>
                  <button className={styles.dangerBtn} onClick={() => moderate(member, 'ban')}>封禁</button>
                  <button className={styles.drawerCloseBtn} onClick={() => moderate(member, 'unban')} disabled={!member.is_banned}>解封</button>
                </div>
              </article>
            ))}
            {visibleMembers.length === 0 && <p className={styles.sideHint}>没有匹配成员</p>}
          </div>
        </div>
      </section>
    </div>
  )
}

function memberModerationSummary(member: ProjectMember) {
  if (member.is_banned) return `已封禁${member.banned_until ? `至 ${formatDateTime(member.banned_until)}` : ''}`
  if (member.is_muted) return `禁言至 ${formatDateTime(member.muted_until)}`
  return `${memberRoleSummary(member)} · ${presenceLabel(memberPresenceStatus(member))}`
}

function numberOrUndefined(value: string) {
  const trimmed = clean(value)
  if (!trimmed) return undefined
  const parsed = Number(trimmed)
  return Number.isFinite(parsed) && parsed > 0 ? Math.floor(parsed) : undefined
}

function inviteUrl(code: string) {
  const url = new URL('/pc', window.location.origin)
  url.searchParams.set('invite', code)
  return url.toString()
}

function PermissionDrawer({
  projectId,
  activeChannelId,
  channels,
  categories,
  members,
  onClose,
  onSaved,
}: {
  projectId: string
  activeChannelId: string
  channels: Channel[]
  categories: ChannelCategory[]
  members: ProjectMember[]
  onClose: () => void
  onSaved: () => Promise<void>
}) {
  const [roles, setRoles] = useState<ProjectRole[]>([])
  const [permissionOptions, setPermissionOptions] = useState<PermissionOption[]>(CHANNEL_PERMISSION_OPTIONS)
  const activeChannel = channels.find((channel) => channel.id === activeChannelId) ?? channels[0]
  const [categoryId, setCategoryId] = useState(activeChannel?.category_id ?? categories[0]?.id ?? '')
  const [channelId, setChannelId] = useState(activeChannel?.id ?? '')
  const [categoryRoleOverrides, setCategoryRoleOverrides] = useState<PermissionOverride[]>([])
  const [categoryMemberOverrides, setCategoryMemberOverrides] = useState<PermissionOverride[]>([])
  const [channelRoleOverrides, setChannelRoleOverrides] = useState<PermissionOverride[]>([])
  const [channelMemberOverrides, setChannelMemberOverrides] = useState<PermissionOverride[]>([])
  const [memberId, setMemberId] = useState(members[0]?.user_id ?? '')
  const [loading, setLoading] = useState(false)
  const [status, setStatus] = useState('')

  useEffect(() => {
    api.get<ProjectRolesResponse>(`/api/projects/${encodeURIComponent(projectId)}/roles`)
      .then((data) => {
        setRoles(data.roles ?? [])
      })
      .catch((err: { message?: string }) => setStatus(err.message ?? '角色加载失败'))
  }, [projectId])

  useEffect(() => {
    if (!categoryId) return
    setLoading(true)
    api.get<ChannelPermissionResponse>(`/api/projects/${encodeURIComponent(projectId)}/channel-categories/${encodeURIComponent(categoryId)}/permissions`)
      .then((data) => {
        setCategoryRoleOverrides(data.overrides ?? [])
        setCategoryMemberOverrides(data.member_overrides ?? data.memberOverrides ?? [])
        if (data.permissions?.length) setPermissionOptions(data.permissions)
      })
      .catch((err: { message?: string }) => setStatus(err.message ?? '分类权限加载失败'))
      .finally(() => setLoading(false))
  }, [projectId, categoryId])

  useEffect(() => {
    if (!channelId) return
    setLoading(true)
    api.get<ChannelPermissionResponse>(`/api/projects/${encodeURIComponent(projectId)}/channels/${encodeURIComponent(channelId)}/permissions`)
      .then((data) => {
        setChannelRoleOverrides(data.overrides ?? [])
        setChannelMemberOverrides(data.member_overrides ?? data.memberOverrides ?? [])
        if (data.permissions?.length) setPermissionOptions(data.permissions)
      })
      .catch((err: { message?: string }) => setStatus(err.message ?? '频道权限加载失败'))
      .finally(() => setLoading(false))
  }, [projectId, channelId])

  function changeChannel(nextChannelId: string) {
    const channel = channels.find((item) => item.id === nextChannelId)
    setChannelId(nextChannelId)
    if (channel?.category_id) setCategoryId(channel.category_id)
  }

  async function saveRole(scope: 'category' | 'channel', roleId: string) {
    const overrides = scope === 'category' ? categoryRoleOverrides : channelRoleOverrides
    const targetId = scope === 'category' ? categoryId : channelId
    if (!targetId) return
    const override = findOverride(overrides, roleId, 'role')
    await savePermissions(scope, targetId, { role_id: roleId, allow: override.allow ?? [], deny: override.deny ?? [] })
  }

  async function saveMember(scope: 'category' | 'channel') {
    if (!memberId) return
    const overrides = scope === 'category' ? categoryMemberOverrides : channelMemberOverrides
    const targetId = scope === 'category' ? categoryId : channelId
    if (!targetId) return
    const override = findOverride(overrides, memberId, 'member')
    await savePermissions(scope, targetId, { member_id: memberId, allow: override.allow ?? [], deny: override.deny ?? [] })
  }

  async function savePermissions(scope: 'category' | 'channel', targetId: string, body: unknown) {
    setStatus('保存中…')
    const base = scope === 'category'
      ? `/api/projects/${encodeURIComponent(projectId)}/channel-categories/${encodeURIComponent(targetId)}/permissions`
      : `/api/projects/${encodeURIComponent(projectId)}/channels/${encodeURIComponent(targetId)}/permissions`
    try {
      const data = await api.patch<ChannelPermissionResponse>(base, body)
      if (scope === 'category') {
        setCategoryRoleOverrides(data.overrides ?? [])
        setCategoryMemberOverrides(data.member_overrides ?? data.memberOverrides ?? [])
      } else {
        setChannelRoleOverrides(data.overrides ?? [])
        setChannelMemberOverrides(data.member_overrides ?? data.memberOverrides ?? [])
      }
      setStatus('已保存')
      await onSaved()
    } catch (err) {
      setStatus((err as { message?: string }).message ?? '保存失败')
    }
  }

  const selectedMember = members.find((member) => member.user_id === memberId)

  return (
    <div className={styles.drawerBackdrop}>
      <section className={styles.permissionDrawer} role="dialog" aria-modal="true">
        <header className={styles.drawerHeader}>
          <div>
            <strong>成员权限</strong>
            <span>{loading ? '同步中…' : status}</span>
          </div>
          <button className={styles.drawerCloseBtn} onClick={onClose}>关闭</button>
        </header>

        <div className={styles.permissionColumns}>
          <section className={styles.permissionBlock}>
            <div className={styles.permissionToolbar}>
              <strong>分类权限</strong>
              <select value={categoryId} onChange={(event) => setCategoryId(event.target.value)}>
                {categories.map((category) => (
                  <option key={category.id} value={category.id}>{category.name || category.kind || category.id}</option>
                ))}
              </select>
            </div>
            <PermissionRoleGrid
              roles={roles}
              options={permissionOptions}
              overrides={categoryRoleOverrides}
              onChange={(roleId, permission, effect) => setCategoryRoleOverrides(updateOverride(categoryRoleOverrides, roleId, 'role', permission, effect))}
              onSave={(roleId) => saveRole('category', roleId)}
            />
            <PermissionMemberGrid
              member={selectedMember}
              members={members}
              memberId={memberId}
              options={permissionOptions}
              overrides={categoryMemberOverrides}
              onMemberChange={setMemberId}
              onChange={(permission, effect) => setCategoryMemberOverrides(updateOverride(categoryMemberOverrides, memberId, 'member', permission, effect))}
              onSave={() => saveMember('category')}
            />
          </section>

          <section className={styles.permissionBlock}>
            <div className={styles.permissionToolbar}>
              <strong>频道覆盖</strong>
              <select value={channelId} onChange={(event) => changeChannel(event.target.value)}>
                {channels.map((channel) => (
                  <option key={channel.id} value={channel.id}>{channel.name}</option>
                ))}
              </select>
            </div>
            <PermissionRoleGrid
              roles={roles}
              options={permissionOptions}
              overrides={channelRoleOverrides}
              onChange={(roleId, permission, effect) => setChannelRoleOverrides(updateOverride(channelRoleOverrides, roleId, 'role', permission, effect))}
              onSave={(roleId) => saveRole('channel', roleId)}
            />
            <PermissionMemberGrid
              member={selectedMember}
              members={members}
              memberId={memberId}
              options={permissionOptions}
              overrides={channelMemberOverrides}
              onMemberChange={setMemberId}
              onChange={(permission, effect) => setChannelMemberOverrides(updateOverride(channelMemberOverrides, memberId, 'member', permission, effect))}
              onSave={() => saveMember('channel')}
            />
          </section>
        </div>
      </section>
    </div>
  )
}

function PermissionRoleGrid({
  roles,
  options,
  overrides,
  onChange,
  onSave,
}: {
  roles: ProjectRole[]
  options: PermissionOption[]
  overrides: PermissionOverride[]
  onChange: (roleId: string, permission: string, effect: PermissionEffect) => void
  onSave: (roleId: string) => void
}) {
  return (
    <div className={styles.permissionCards}>
      {roles.map((role) => (
        <article key={role.id} className={styles.permissionCard}>
          <div className={styles.permissionCardHead}>
            <span className={styles.roleSwatch} style={{ background: role.color ?? '#747f8d' }} />
            <strong>{role.name || roleLabel(role.id)}</strong>
          </div>
          <PermissionGrid
            options={options}
            override={findOverride(overrides, role.id, 'role')}
            onChange={(permission, effect) => onChange(role.id, permission, effect)}
          />
          <button className={styles.savePermissionBtn} onClick={() => onSave(role.id)}>保存</button>
        </article>
      ))}
    </div>
  )
}

function PermissionMemberGrid({
  member,
  members,
  memberId,
  options,
  overrides,
  onMemberChange,
  onChange,
  onSave,
}: {
  member?: ProjectMember
  members: ProjectMember[]
  memberId: string
  options: PermissionOption[]
  overrides: PermissionOverride[]
  onMemberChange: (memberId: string) => void
  onChange: (permission: string, effect: PermissionEffect) => void
  onSave: () => void
}) {
  return (
    <article className={styles.permissionCard}>
      <div className={styles.permissionToolbar}>
        <strong>成员覆盖</strong>
        <select value={memberId} onChange={(event) => onMemberChange(event.target.value)}>
          {members.map((item) => (
            <option key={item.user_id} value={item.user_id}>{item.account || item.user_id}</option>
          ))}
        </select>
      </div>
      {member && <small className={styles.permissionMemberName}>{memberRoleSummary(member)}</small>}
      <PermissionGrid
        options={options}
        override={findOverride(overrides, memberId, 'member')}
        onChange={onChange}
      />
      <button className={styles.savePermissionBtn} onClick={onSave} disabled={!memberId}>保存</button>
    </article>
  )
}

type PermissionEffect = '' | 'allow' | 'deny'

function PermissionGrid({
  options,
  override,
  onChange,
}: {
  options: PermissionOption[]
  override: PermissionOverride
  onChange: (permission: string, effect: PermissionEffect) => void
}) {
  return (
    <div className={styles.permissionGrid}>
      {options.map((option) => (
        <label key={option.key}>
          <span>{option.label}</span>
          <select value={permissionEffect(override, option.key)} onChange={(event) => onChange(option.key, event.target.value as PermissionEffect)}>
            <option value="">继承</option>
            <option value="allow">允许</option>
            <option value="deny">拒绝</option>
          </select>
        </label>
      ))}
    </div>
  )
}

function permissionEffect(override: PermissionOverride, permission: string): PermissionEffect {
  if ((override.deny ?? []).includes(permission)) return 'deny'
  if ((override.allow ?? []).includes(permission)) return 'allow'
  return ''
}

function findOverride(overrides: PermissionOverride[], targetId: string, kind: 'role' | 'member') {
  const key = kind === 'role' ? 'role_id' : 'user_id'
  const altKey = kind === 'role' ? 'roleId' : 'userId'
  return overrides.find((override) => clean(String(override[key] ?? override[altKey] ?? '')) === targetId) ?? {}
}

function updateOverride(
  overrides: PermissionOverride[],
  targetId: string,
  kind: 'role' | 'member',
  permission: string,
  effect: PermissionEffect,
) {
  if (!targetId) return overrides
  const next = overrides.slice()
  const index = next.findIndex((override) => clean(String(kind === 'role' ? (override.role_id ?? override.roleId) : (override.user_id ?? override.userId))) === targetId)
  const current = index >= 0 ? next[index] : (kind === 'role' ? { role_id: targetId } : { user_id: targetId })
  const allow = new Set(current.allow ?? [])
  const deny = new Set(current.deny ?? [])
  allow.delete(permission)
  deny.delete(permission)
  if (effect === 'allow') allow.add(permission)
  if (effect === 'deny') deny.add(permission)
  const updated = { ...current, allow: Array.from(allow), deny: Array.from(deny) }
  if (index >= 0) next[index] = updated
  else next.push(updated)
  return next
}

/* ── 单条消息组件 ── */
function MessageItem({ message, isDevChannel, taskContext, user, onCancel, onApprove, grouped }: {
  message: Message
  isDevChannel: boolean
  taskContext: ReturnType<typeof buildContext>
  user: { nickname?: string; account?: string } | null
  onCancel: (id: string) => Promise<void>
  onApprove: (taskId: string, approvalId: string, decision: 'approve' | 'deny') => Promise<void>
  grouped?: boolean
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
    <div className={[styles.messageRow, isUser ? styles.ownRow : '', grouped ? styles.grouped : ''].filter(Boolean).join(' ')}>
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


/* ── 成员搜索 + 虚拟分组列表 ── */
type MemberVirtualRow =
  | { kind: 'header'; id: string; label: string; count: number }
  | { kind: 'member'; id: string; member: ProjectMember }

const MEMBER_VIRTUAL_ROW_HEIGHT = 48
const MEMBER_LIST_OVERSCAN = 6
const MEMBER_LIST_WINDOW = 28

function MemberSearch({ members, onSelect }: { members: ProjectMember[]; onSelect: (member: ProjectMember, y: number) => void }) {
  const [query, setQuery] = useState('')
  const [scrollTop, setScrollTop] = useState(0)
  const q = query.trim().toLowerCase()
  const filtered = useMemo(
    () => q ? filterVisibleMembers(members, q) : members,
    [members, q],
  )
  const rows = useMemo(() => buildMemberRows(filtered), [filtered])
  const start = Math.max(0, Math.floor(scrollTop / MEMBER_VIRTUAL_ROW_HEIGHT) - MEMBER_LIST_OVERSCAN)
  const end = Math.min(rows.length, start + MEMBER_LIST_WINDOW)
  const visibleRows = rows.slice(start, end)
  return (
    <>
      <div className={styles.memberSearch}>
        <input
          className={styles.memberSearchInput}
          value={query}
          onChange={e => {
            setQuery(e.target.value)
            setScrollTop(0)
          }}
          placeholder="搜索成员"
          autoComplete="off"
        />
        {query && (
          <button className={styles.memberSearchClear} type="button" onClick={() => setQuery('')}>×</button>
        )}
      </div>
      <div className={styles.memberVirtualList} onScroll={event => setScrollTop(event.currentTarget.scrollTop)}>
        {rows.length === 0 && <div className={styles.memberSection}>没有匹配成员</div>}
        {rows.length > 0 && (
          <div className={styles.memberVirtualCanvas} style={{ height: rows.length * MEMBER_VIRTUAL_ROW_HEIGHT }}>
            <div style={{ transform: `translateY(${start * MEMBER_VIRTUAL_ROW_HEIGHT}px)` }}>
              {visibleRows.map(row => row.kind === 'header'
                ? <div key={row.id} className={styles.memberVirtualHeader}><div className={styles.memberSection}>{row.label} · {row.count}</div></div>
                : <MemberListItem key={row.id} member={row.member} onSelect={onSelect} />
              )}
            </div>
          </div>
        )}
      </div>
    </>
  )
}

function filterVisibleMembers(members: ProjectMember[], query: string) {
  return members.filter(member => {
    const haystack = [
      member.account,
      member.user_id,
      member.role,
      member.custom_status,
      member.activity,
      ...(member.roles ?? []).map(role => role.name || role.id),
    ].join(' ').toLowerCase()
    return haystack.includes(query)
  })
}

function buildMemberRows(members: ProjectMember[]): MemberVirtualRow[] {
  const roleLabelMap: Record<string, string> = {
    admin: '管理员',
    owner: '管理员',
    collaborator: '协作者',
    editor: '协作者',
  }
  const groups: [string, ProjectMember[]][] = [
    ['管理员', members.filter(m => ['admin', 'owner'].includes((m.role ?? '').toLowerCase()))],
    ['协作者', members.filter(m => ['collaborator', 'editor'].includes((m.role ?? '').toLowerCase()))],
    ['成员', members.filter(m => !roleLabelMap[(m.role ?? '').toLowerCase()])],
  ]
  return groups.flatMap(([label, list]) => {
    if (!list.length) return []
    return [
      { kind: 'header' as const, id: `header-${label}`, label, count: list.length },
      ...list.map(member => ({ kind: 'member' as const, id: member.user_id, member })),
    ]
  })
}

function MemberListItem({ member, onSelect }: { member: ProjectMember; onSelect: (member: ProjectMember, y: number) => void }) {
  const roleKey = (member.role ?? '').toLowerCase()
  const roleLabelMap: Record<string, string> = {
    admin: '管理员', owner: '管理员',
    collaborator: '协作者', editor: '协作者',
  }
  const roleCss: Record<string, string> = {
    admin: styles.memberRolePillAdmin, owner: styles.memberRolePillOwner,
    collaborator: styles.memberRolePillEditor, editor: styles.memberRolePillEditor,
  }
  const avatarCss: Record<string, string> = {
    admin: styles.memberAvatarAdmin, owner: styles.memberAvatarOwner,
    collaborator: styles.memberAvatarEditor, editor: styles.memberAvatarEditor,
  }
  const roleBadge = roleLabelMap[roleKey]
  const name = member.account ?? member.user_id
  const avatarCls = [
    styles.memberAvatar,
    avatarCss[roleKey] ?? '',
    member.is_online ? styles.memberAvatarOnline : styles.memberAvatarOffline,
  ].filter(Boolean).join(' ')
  return (
    <button className={styles.memberItem} type="button" onClick={(e) => {
      const rect = e.currentTarget.getBoundingClientRect()
      onSelect(member, rect.top + rect.height / 2)
    }}>
      <div className={avatarCls}>
        {member.avatar_data_url
          ? <img src={member.avatar_data_url} alt="" style={{ width: '100%', height: '100%', borderRadius: '50%', objectFit: 'cover', display: 'block' }} />
          : name[0].toUpperCase()
        }
      </div>
      <div className={styles.memberCopy}>
        <div className={styles.memberLine}>
          <strong className={styles.memberItemName}>{name}</strong>
          {roleBadge && <em className={[styles.memberRolePill, roleCss[roleKey] ?? ''].join(' ')}>{roleBadge}</em>}
        </div>
        <span className={styles.memberSub}>{memberSubtitle(member)}</span>
      </div>
    </button>
  )
}

/* ── 浮动用户卡片（Discord 风格，定位于右侧栏左侧）── */
function MemberProfilePopover({ member, anchorY, onClose }: {
  member: ProjectMember
  anchorY: number
  onClose: () => void
}) {
  const popRef = useRef<HTMLDivElement>(null)
  const status = memberPresenceStatus(member)
  const name = member.account || member.user_id
  const roleKey = (member.role ?? '').toLowerCase()
  const roleHeadCls = {
    owner: styles.popoverHeadOwner, admin: styles.popoverHeadAdmin,
    editor: styles.popoverHeadEditor, collaborator: styles.popoverHeadEditor,
  }[roleKey] ?? ''
  const [isFriend, setIsFriend] = useState(false)
  const [addingFriend, setAddingFriend] = useState(false)
  const [addMsg, setAddMsg] = useState('')

  // 启动时检查是否已是好友
  useEffect(() => {
    if (!member.user_id) return
    api.get<{ friends?: Array<{ id: string }> }>('/api/me/friends')
      .then(d => setIsFriend(!!(d.friends ?? []).find(f => f.id === member.user_id)))
      .catch(() => {})
  }, [member.user_id])

  // 点击外部关闭
  useEffect(() => {
    function onDown(e: MouseEvent) {
      if (popRef.current && !popRef.current.contains(e.target as Node)) onClose()
    }
    document.addEventListener('mousedown', onDown)
    return () => document.removeEventListener('mousedown', onDown)
  }, [onClose])

  function copyId() {
    navigator.clipboard.writeText(member.user_id).catch(() => {})
  }

  async function addFriend() {
    if (isFriend || addingFriend) return
    setAddingFriend(true)
    try {
      await api.post('/api/me/friends', { query: member.user_id, search_type: 'user_id' })
      setIsFriend(true)
      setAddMsg('已添加')
    } catch (err) {
      setAddMsg((err as { message?: string }).message ?? '添加失败')
    } finally {
      setAddingFriend(false)
    }
  }

  const details: [string, string][] = [
    member.account && ['账号', member.account],
    member.user_id && ['用户 ID', member.user_id.slice(0, 14)],
    member.joined_at && ['加入时间', formatTime(member.joined_at)],
  ].filter(Boolean) as [string, string][]

  const POPOVER_WIDTH = 284
  const POPOVER_HEIGHT = 280
  const viewW = window.innerWidth
  const viewH = window.innerHeight
  const popTop = Math.min(Math.max(anchorY - 20, 12), viewH - POPOVER_HEIGHT - 12)
  // 定位在右侧栏左侧（右侧栏约 280px）
  const popLeft = Math.max(8, viewW - 280 - POPOVER_WIDTH - 8)

  return (
    <div ref={popRef} className={styles.memberPopover}
      style={{ position: 'fixed', left: popLeft, top: popTop, zIndex: 9999, width: POPOVER_WIDTH }}>
      {/* 头部 */}
      <div className={[styles.memberPopoverHead, roleHeadCls].join(' ')}>
        <div className={[
          styles.memberPopoverAvatar,
          status === 'online' ? styles.memberAvatarOnline : styles.memberAvatarOffline,
        ].join(' ')}>
          {member.avatar_data_url
            ? <img src={member.avatar_data_url} alt="" />
            : <span>{name[0]?.toUpperCase() ?? '?'}</span>
          }
        </div>
        <button className={styles.memberPopoverClose} onClick={onClose} type="button">×</button>
      </div>
      {/* 主体 */}
      <div className={styles.memberPopoverBody}>
        <strong className={styles.memberPopoverName}>{name}</strong>
        <span className={styles.memberPopoverSub}>{presenceLabel(status)}</span>
        <div className={styles.memberPopoverMeta}>
          <em className={styles.memberPopoverPill}>{memberRoleSummary(member)}</em>
          <em className={[styles.memberPopoverPill, status === 'online' ? styles.memberPopoverPillOnline : ''].join(' ')}>
            {presenceLabel(status)}
          </em>
        </div>
        {details.length > 0 && (
          <div className={styles.memberPopoverDetails}>
            {details.map(([label, value]) => (
              <div key={label} className={styles.memberPopoverDetail}>
                <span>{label}</span>
                <strong title={value}>{value}</strong>
              </div>
            ))}
          </div>
        )}
        <div className={styles.memberPopoverActions}>
          <button className={styles.memberPopoverBtn} type="button" onClick={copyId}>复制 ID</button>
          <button className={styles.memberPopoverBtn} type="button"
            onClick={addFriend} disabled={isFriend || addingFriend}
            style={{ background: isFriend ? 'rgba(88,190,106,.1)' : undefined, color: isFriend ? 'var(--green,#58BE6A)' : undefined, cursor: isFriend ? 'default' : 'pointer' }}>
            {addMsg || (isFriend ? '已是好友' : addingFriend ? '添加中…' : '加好友')}
          </button>
        </div>
      </div>
    </div>
  )
}

function MemberLoadingRows() {
  return (
    <div className={styles.memberLoadingRows}>
      <span />
      <span />
      <span />
    </div>
  )
}
