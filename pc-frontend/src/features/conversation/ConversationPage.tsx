import { useEffect, useMemo, useRef, useState, useCallback } from 'react'
import { createPortal } from 'react-dom'
import { useNavigate } from 'react-router-dom'
import { v4 as uuidv4 } from 'uuid'
import { useProjectStore } from './useProjectStore'
import { useChannelAutoRefresh } from './useChannelAutoRefresh'
import { AttachmentButton, AttachmentChip, attachmentsToMarkdown } from './AttachmentButton'
import type { UploadedAttachment } from './AttachmentButton'
import { useAuthStore } from '../../store/auth'
import { useModelStore } from '../models/useModelStore'
import { ModelPickerPopover } from '../models/ModelPicker'
import DevTaskGroup from '../dev/DevTaskGroup'
import AgentRunsPanel from '../dev/AgentRunsPanel'
import { buildContext } from '../dev/devTaskUtils'
import { CreateProjectModal } from '../projects/CreateProjectModal'
import ProjectLanding from './ProjectLanding'
import NodeOfflineBanner from './NodeOfflineBanner'
import { api } from '../../api/client'
import { clean, safeNodeAdminUrl } from '../../lib/utils'
import { localJson } from '../doctor/localApi'
import {
  routeModelButtonCopy,
  selectedAgentForRuntimeRoute,
} from '../models/routeModelPolicy'
import { initialRuntimeRouteFromStorage, persistRuntimeRouteSelection } from './runtimeRoutes'
import type { RuntimeRoute } from './runtimeRoutes'
import type {
  Channel,
  Message,
  Project,
  ProjectInvitePreview,
  ProjectInvitePreviewResponse,
  ProjectMember,
} from './types'
import MemberConversationList from './MemberConversationList'
import {
  listMemberConversationMessages,
  listMemberConversations,
  sameConversationTarget,
  sendMemberConversationDiscussion,
  targetDisplayName,
  targetFromProjectMember,
  targetFromUser,
} from './memberConversationApi'
import type {
  MemberConversationEntry,
  MemberConversationMessage,
  MemberConversationTarget,
} from './memberConversationApi'
import {
  channelCanManage,
  channelPermissionSummary,
  membersHaveChannelPermissionMap,
  membersForChannel,
  projectMemberHasRolePermission,
  ROLE_PERMISSION_MANAGE_MEMBERS,
  ROLE_PERMISSION_MANAGE_ROLES,
  ROLE_PERMISSION_MODERATE_MEMBERS,
  ROLE_PERMISSION_VIEW_AUDIT_LOG,
  inviteTitle,
  roleLabel,
} from './memberUtils'
import { PresenceDrawer } from './PresenceDrawer'
import { InviteDrawer } from './InviteDrawer'
import { ModerationDrawer } from './ModerationDrawer'
import { MemberAuditDrawer } from './MemberAuditDrawer'
import { RoleManagementDrawer } from './RoleManagementDrawer'
import { PermissionDrawer } from './PermissionDrawer'
import { MessageItem } from './ConversationMessage'
import {
  MemberSearch,
  MemberContextMenu,
  MemberProfilePopover,
  MemberContextSummary,
  MemberLoadingRows,
} from './MemberPanel'
import type { MemberMenuRequest, MemberModerationAction } from './MemberPanel'
import SidebarUserStrip from '../shell/SidebarUserStrip'
import styles from './ConversationPage.module.css'

interface LocalNodeStatus {
  agent_id?: string
  owner_user_id?: string
  device_name?: string
  connected?: boolean
  codex_cli?: { available?: boolean; logged_in?: boolean; status?: string }
}

interface ProjectRealtimeDetail {
  projectId?: string
  channelId?: string
  conversationId?: string
  taskId?: string
  kind?: string
}

export default function ConversationPage() {
  useChannelAutoRefresh()
  const navigate = useNavigate()
  const user = useAuthStore((s) => s.user)
  const {
    projects, projectsLoaded, activeProjectId, channels, categories, members, activeChannelId,
    messages, messagesLoading, sendingMessage, space, landing, spaceLoading, spaceError,
    projectHomeVersion,
    loadProjects, selectProject, reloadProjectSpace, selectChannel, sendMessage, cancelTask, approveTool,
  } = useProjectStore()
  const selectedAgent = useModelStore((s) => s.selectedAgent)
  const modelLabel = useModelStore((s) => s.label)
  const modelOptions = useModelStore((s) => s.options)
  const [input, setInput] = useState('')
  const [sendError, setSendError] = useState('')
  const [showCreate, setShowCreate] = useState(false)
  const [showModelPicker, setShowModelPicker] = useState(false)
  const [runtimeRoute, setRuntimeRoute] = useState<RuntimeRoute>(() => initialRuntimeRouteFromStorage(
    typeof window === 'undefined' ? null : window.localStorage,
  ))
  const [showPermissions, setShowPermissions] = useState(false)
  const [showPresence, setShowPresence] = useState(false)
  const [showInvites, setShowInvites] = useState(false)
  const [showModeration, setShowModeration] = useState(false)
  const [showAudit, setShowAudit] = useState(false)
  const [showRoles, setShowRoles] = useState(false)
  const [selectedMember, setSelectedMember] = useState<ProjectMember | null>(null)
  const [memberPopoverY, setMemberPopoverY] = useState(200)
  const [memberMenu, setMemberMenu] = useState<MemberMenuRequest | null>(null)
  const [permissionFocusMemberId, setPermissionFocusMemberId] = useState('')
  const [roleFocusMemberId, setRoleFocusMemberId] = useState('')
  const [localNode, setLocalNode] = useState<LocalNodeStatus | null>(null)
  const [localNodeError, setLocalNodeError] = useState('')
  const [localBindStatus, setLocalBindStatus] = useState('')
  const autoBindRef = useRef('')

  // ── 手机/PC 同步会话列表（直接读服务端，与移动端完全同步）──
  const [memberConversationTarget, setMemberConversationTarget] = useState<MemberConversationTarget | null>(null)
  const [memberConversations, setMemberConversations] = useState<MemberConversationEntry[]>([])
  const [convMessages, setConvMessages] = useState<Message[]>([])
  const [convLoading, setConvLoading] = useState(false)
  const [sendingMemberDiscussion, setSendingMemberDiscussion] = useState(false)
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
  // 会话视图模式：null=默认(全部) / 'new'=新建空会话 / string=会话 ID
  const [sessionView, setSessionView] = useState<string | 'new' | null>(null)
  const prevSessionIdsRef = useRef<Set<string>>(new Set())
  const waitingForNewSession = useRef(false)
  const modelButtonCopy = useMemo(
    () => routeModelButtonCopy(runtimeRoute, modelLabel, modelOptions, selectedAgent),
    [runtimeRoute, modelLabel, modelOptions, selectedAgent],
  )

  useEffect(() => { loadProjects() }, [user?.id]) // eslint-disable-line

  useEffect(() => {
    persistRuntimeRouteSelection(window.localStorage, runtimeRoute)
  }, [runtimeRoute])

  useEffect(() => {
    let canceled = false
    async function loadLocalNode() {
      try {
        const status = await localJson<LocalNodeStatus>(safeNodeAdminUrl(), '/api/status')
        if (canceled) return
        setLocalNode(status)
        setLocalNodeError('')
      } catch (err) {
        if (canceled) return
        setLocalNode(null)
        setLocalNodeError((err as { message?: string }).message ?? '未检测到本机节点')
      }
    }
    loadLocalNode()
    const timer = window.setInterval(loadLocalNode, 10000)
    return () => {
      canceled = true
      window.clearInterval(timer)
    }
  }, [])

  const ownConversationTarget = useMemo(() => targetFromUser(user), [user])
  const activeConversationTarget = memberConversationTarget ?? ownConversationTarget
  const activeConversationTargetId = activeConversationTarget?.userId ?? ''
  const isOwnConversationTarget = !!ownConversationTarget
    && !!activeConversationTarget
    && sameConversationTarget(ownConversationTarget, activeConversationTarget)
  const activeConversationTargetName = targetDisplayName(activeConversationTarget)
  const isAssistingMember = !!activeConversationTarget && !isOwnConversationTarget

  // 加载项目会话列表（与手机端同步）
  useEffect(() => {
    if (!activeProjectId) return
    if (!activeConversationTargetId) return
    setMemberConversations([])
    listMemberConversations(activeProjectId, activeConversationTargetId).then(setMemberConversations).catch((err: { message?: string; status?: number }) => {
      console.warn('[MemberConversations] failed:', err?.status, err?.message)
    })
  }, [activeProjectId, activeConversationTargetId]) // eslint-disable-line

  // 项目切换时清空会话消息
  useEffect(() => {
    setConvMessages([])
    setSessionView(null)
    setMemberConversationTarget(null)
    waitingForNewSession.current = false
  }, [activeProjectId, projectHomeVersion]) // eslint-disable-line

  useEffect(() => {
    setSelectedMember(null)
    setMemberMenu(null)
    setPermissionFocusMemberId('')
    setRoleFocusMemberId('')
    setShowAudit(false)
    setShowRoles(false)
  }, [activeProjectId, activeChannelId])

  useEffect(() => {
    if (!selectedMember?.user_id) return
    const fresh = members.find((member) => member.user_id === selectedMember.user_id)
    if (fresh && fresh !== selectedMember) setSelectedMember(fresh)
    if (!fresh) setSelectedMember(null)
  }, [members, selectedMember])

  useEffect(() => {
    if (!memberMenu?.member.user_id) return
    const fresh = members.find((member) => member.user_id === memberMenu.member.user_id)
    if (fresh && fresh !== memberMenu.member) setMemberMenu({ ...memberMenu, member: fresh })
    if (!fresh) setMemberMenu(null)
  }, [members, memberMenu])

  useEffect(() => {
    setSessionView(null)
    setConvMessages([])
    waitingForNewSession.current = false
  }, [activeConversationTargetId])

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
  }, [messages, convMessages, sessionView])

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

  const taskContext = buildContext(messages as Parameters<typeof buildContext>[0])

  function isTerminalTaskStatus(status: unknown): boolean {
    return ['done', 'failed', 'error', 'canceled', 'cancelled', 'interrupted'].includes(String(status ?? '').toLowerCase())
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
    const isMemberDiscussion = isAssistingMember && !!activeProjectId && !!activeConversationTargetId && !!sessionView && sessionView !== 'new'
    if (!text || sendingMessage || sendingMemberDiscussion) return
    if (isAssistingMember && !isMemberDiscussion) {
      setSendError('请先选择这个成员的一个会话')
      return
    }
    setSendError('')
    const previousInput = input
    const previousAttachments = attachments
    const fullContent = attachments.length > 0
      ? text + attachmentsToMarkdown(attachments)
      : text
    try {
      if (isMemberDiscussion) {
        clearComposerDraft()
        setSendingMemberDiscussion(true)
        const message = await sendMemberConversationDiscussion(
          activeProjectId,
          activeConversationTargetId,
          String(sessionView),
          fullContent,
        )
        setConvMessages((prev) => [...prev, message as MemberConversationMessage])
        listMemberConversations(activeProjectId, activeConversationTargetId)
          .then(setMemberConversations)
          .catch(() => {})
        return
      }

      let targetChannelId = activeChannelId
      let targetChannel = targetChannelId ? channels.find((c) => c.id === targetChannelId) : undefined
      if (!targetChannelId) {
        const best = channels.find((c) => c.kind === 'ai_development')
        if (!best) {
          setSendError('当前项目没有 AI 开发频道，不能发起 AI 对话')
          return
        }
        targetChannelId = best.id
        targetChannel = best
        await selectChannel(best.id)
      }
      if (!targetChannel || targetChannel.kind !== 'ai_development') {
        setSendError('请选择 AI 开发频道后再发送 AI 对话')
        return
      }
      if (!channelAllowsAiStart(targetChannel)) {
        setSendError('当前项目角色不能在这个频道发起 AI 开发')
        return
      }

      clearComposerDraft()
      const isExistingConversation = typeof sessionView === 'string' && sessionView !== 'new'
      const conversationId = isExistingConversation ? sessionView : uuidv4()
      const conversationTitle = isExistingConversation ? null : titleFromMessage(text)
      const requestAgent = selectedAgentForRuntimeRoute(selectedAgent, modelOptions, runtimeRoute)
      const response = await sendMessage(
        fullContent,
        requestAgent || null,
        runtimeRoute,
        conversationId,
        conversationTitle,
        shouldPreferLocalNode && localNodeReady ? localNodeId : null,
        shouldPreferLocalNode && localNodeReady ? activeWorkspacePath : null,
        targetChannelId,
      )
      const openedConversationId = response?.conversation_id ?? conversationId
      waitingForNewSession.current = false
      setSessionView(openedConversationId)
      const optimisticTaskId = clean(response?.task_id ?? response?.message?.task_id ?? response?.message?.taskId)
      setConvMessages([{
        id: `optimistic-${openedConversationId}-${Date.now()}`,
        role: 'user',
        content: fullContent,
        created_at: new Date().toISOString(),
        user_id: user?.id,
        sender_name: user?.nickname ?? user?.account ?? '我',
        outgoing: true,
        task_id: optimisticTaskId || undefined,
      } as Message])
      // 发送后刷新会话列表和当前会话消息，保证继续输入时仍在同一上下文。
      if (activeProjectId && activeConversationTargetId) {
        setTimeout(async () => {
          try {
            const conversations = await listMemberConversations(activeProjectId, activeConversationTargetId)
            setMemberConversations(conversations)
            await openConversation(openedConversationId)
          } catch {}
        }, 400)
      }
    } catch (err) {
      setInput(previousInput)
      setAttachments(previousAttachments)
      setTimeout(autoResize, 0)
      setSendError((err as { message?: string }).message ?? '发送失败')
    } finally {
      setSendingMemberDiscussion(false)
    }
  }

  function clearComposerDraft() {
    setInput('')
    setAttachments([])
    if (textareaRef.current) { textareaRef.current.style.height = '46px' }
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

  const listedProject = useMemo(
    () => projects.find((p) => p.id === activeProjectId),
    [projects, activeProjectId],
  )
  const activeProject = useMemo(
    () => mergeProjectRecords(listedProject, space?.project),
    [listedProject, space?.project],
  )
  const activeChannel = channels.find((c) => c.id === activeChannelId)
  const isDevChannel = activeChannel?.kind === 'ai_development'
  const activeWorkspacePath = clean(activeProject?.workspace_path ?? activeProject?.storage_worktree_path)
  const activeProjectRole = clean(activeProject?.role ?? activeProject?.my_role ?? space?.my_role).toLowerCase()
  const activeProjectRoleLabel = projectRoleLabel(activeProjectRole)
  const localNodeId = clean(localNode?.agent_id)
  const localNodeOwnerOk = !!localNodeId && !!user?.id && clean(localNode?.owner_user_id) === user.id
  const localNodeReady = localNodeOwnerOk
    && localNode?.connected !== false
    && localNode?.codex_cli?.available !== false
  const shouldPreferLocalNode = !['route_c2', 'route_c3'].includes(runtimeRoute)
  const projectBoundToLocalNode = !!localNodeId && activeProject?.node_id === localNodeId
  const activeChannelBlocksAi = !!activeChannel && activeChannel.kind === 'ai_development' && !channelAllowsAiStart(activeChannel)
  const activeChannelIsNotAi = !!activeChannel && activeChannel.kind !== 'ai_development'
  const canManagePermissions = channels.some(channelCanManage)
  // taskContext 和 hasRunningTask 已在上方 P1.3 代码块中定义

  useEffect(() => {
    if (!activeProjectId || !activeProject || !localNodeReady || !localNodeId) return
    if (!shouldPreferLocalNode) return
    if (!projectRoleCanAutoBind(activeProjectRole)) {
      setLocalBindStatus(activeProjectRole ? '当前项目不是 owner，不自动切换节点' : '')
      return
    }
    if (activeProject.node_id === localNodeId) {
      setLocalBindStatus('')
      return
    }
    if (!activeWorkspacePath) {
      setLocalBindStatus('当前项目缺少工作区路径，暂不自动切换')
      return
    }
    const key = `${activeProjectId}:${localNodeId}:${activeProject.node_id ?? ''}:${activeWorkspacePath || 'no-path'}`
    if (autoBindRef.current === key) return
    autoBindRef.current = key
    setLocalBindStatus('正在切换到当前电脑…')
    let canceled = false
    async function recoverOnLocalNode() {
      const endpoint = `/api/projects/${encodeURIComponent(activeProjectId)}/workspace/recover`
      const bindPayload: { action: string; node_id: string; workspacePath: string } = {
        action: 'bind_pc_node',
        node_id: localNodeId,
        workspacePath: activeWorkspacePath,
      }
      await api.post<{ project?: unknown; message?: string }>(endpoint, bindPayload)
      if (canceled) return
      setLocalBindStatus('已优先使用当前电脑')
      await loadProjects()
      await reloadProjectSpace()
    }
    recoverOnLocalNode().catch((err: { message?: string }) => {
      if (!canceled) setLocalBindStatus(err.message ?? '当前电脑自动绑定失败')
    })
    return () => { canceled = true }
  }, [
    activeProjectId,
    activeProject,
    activeWorkspacePath,
    activeProjectRole,
    localNodeReady,
    localNodeId,
    shouldPreferLocalNode,
    loadProjects,
    reloadProjectSpace,
  ])

  const filteredChannels = channelSearch
    ? channels.filter((c) => c.name.toLowerCase().includes(channelSearch.toLowerCase()))
    : channels

  // 成员列表：从 project space 读取
  const spaceMembers = members
  const currentProjectMember = useMemo(
    () => spaceMembers.find((member) => member.user_id === user?.id),
    [spaceMembers, user?.id],
  )
  const canModerateMembers = !!activeProjectId
    && !!currentProjectMember
    && projectMemberHasRolePermission(currentProjectMember, [], ROLE_PERMISSION_MODERATE_MEMBERS)
  const canManageMembers = !!activeProjectId
    && !!currentProjectMember
    && projectMemberHasRolePermission(currentProjectMember, [], ROLE_PERMISSION_MANAGE_MEMBERS)
  const canViewMemberAudit = !!activeProjectId
    && !!currentProjectMember
    && (
      projectMemberHasRolePermission(currentProjectMember, [], ROLE_PERMISSION_VIEW_AUDIT_LOG)
      || canManageMembers
    )
  const canManageRoles = !!activeProjectId
    && !!currentProjectMember
    && projectMemberHasRolePermission(currentProjectMember, [], ROLE_PERMISSION_MANAGE_ROLES)
  const canUseRoleManager = canManageRoles || canManageMembers
  const hasChannelMemberPermissions = !!activeChannelId && membersHaveChannelPermissionMap(spaceMembers, activeChannelId)
  const panelMembers = useMemo(
    () => activeChannelId ? membersForChannel(spaceMembers, activeChannelId) : spaceMembers,
    [spaceMembers, activeChannelId],
  )
  const memberPanelTitle = activeChannel ? '频道成员' : activeProjectId ? '项目成员' : '工作台'
  const memberPanelContext = activeChannel?.name ?? activeProject?.name ?? '我的项目'
  const memberPanelCount = activeProjectId ? panelMembers.length : (user ? 1 : 0)
  const memberPanelSummary = activeChannel
    ? channelPermissionSummary(activeChannel, panelMembers.length, spaceMembers.length, hasChannelMemberPermissions)
    : activeProjectId
      ? `项目共 ${spaceMembers.length} 位成员，按角色分组`
      : '个人 AI 工作台'

  // 成员卡片弹窗
  // (memberPopover state removed - not currently used)

  // 消息分组：判断某条消息是否与上一条来自同一发送者（仅用于非任务消息）
  // src 必须与当前渲染的消息数组一致，避免索引越界
  function isGroupedIn(src: Message[], idx: number): boolean {
    if (idx === 0 || idx >= src.length) return false
    const cur  = src[idx]
    const prev = src[idx - 1]
    if (!cur || !prev) return false
    const curRole  = clean(cur.kind  ?? cur.role  ?? '').toLowerCase()
    const prevRole = clean(prev.kind ?? prev.role ?? '').toLowerCase()
    const curId  = clean(cur.user_id  ?? (cur as Record<string, unknown>).userId  ?? '')
    const prevId = clean(prev.user_id ?? (prev as Record<string, unknown>).userId ?? '')
    if (['ai_task','ai_progress','ai_result'].includes(curRole)) return false
    if (['ai_task','ai_progress','ai_result'].includes(prevRole)) return false
    if (curRole === prevRole) {
      if (curRole === 'user' || curRole === 'human' || curRole === 'discussion') return curId !== '' && curId === prevId
      return true
    }
    return false
  }

  const memberDiscussionNeedsConversation = isAssistingMember && (!sessionView || sessionView === 'new')
  const composerBusy = sendingMessage || sendingMemberDiscussion
  const composerDisabled = composerBusy
    || memberDiscussionNeedsConversation
    || activeChannelBlocksAi
    || activeChannelIsNotAi
    || (!activeChannelId && channels.length === 0 && !isAssistingMember)

  // 根据会话视图过滤显示的消息（必须在 messageGroups 之前声明）
  const displayMessages = useMemo(() => {
    if (!sessionView) return messages
    if (sessionView === 'new') return []
    // 选中了真实会话（从服务端加载）
    if (convMessages.length > 0 || convLoading) return convMessages
    // 降级：从频道消息中按 task_id 过滤
    return messages.filter((msg) => {
      const tid = String((msg.task_id ?? (msg as Record<string, unknown>).taskId) ?? '')
      return tid === sessionView
    })
  }, [messages, sessionView, convMessages, convLoading])

  // P1.3：打字指示器只看当前可见会话，避免其它历史任务让本会话一直显示处理中。
  const hasRunningTask = useMemo(() => {
    const taskIds = new Set<string>()
    const doneIds = new Set<string>()
    for (const m of displayMessages) {
      const kind = ((m.kind ?? m.role ?? '') as string).toLowerCase()
      const id = (m.task_id ?? m.taskId ?? '') as string
      if (!id) continue
      if (kind === 'ai_task') taskIds.add(id)
      if (kind === 'ai_result' || isTerminalTaskStatus(m.task_status ?? m.taskStatus)) doneIds.add(id)
    }
    for (const id of taskIds) if (!doneIds.has(id)) return true
    return false
  }, [displayMessages])

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
        groups.push({ type: 'single', msg, grouped: isGroupedIn(src, i), key: msg.id ?? String(i) })
      }
    }
    return groups
  }, [displayMessages, isDevChannel]) // eslint-disable-line

  // sessionView='new' 时，一旦会话列表出现新会话，自动切入
  useEffect(() => {
    if (!waitingForNewSession.current) return
    const newConv = memberConversations.find((c) => !prevSessionIdsRef.current.has(c.id))
    if (newConv) {
      waitingForNewSession.current = false
      openConversation(newConv.id)
    }
  }, [memberConversations]) // eslint-disable-line

  // 切换频道时重置会话视图
  useEffect(() => {
    setSessionView(null)
    setConvMessages([])
    waitingForNewSession.current = false
  }, [activeChannelId]) // eslint-disable-line

  // 打开一个会话：从服务端加载该会话的消息（与手机端同步）
  async function openConversation(convId: string) {
    if (!activeProjectId || !activeConversationTargetId) return
    setSessionView(convId)
    setConvMessages([])
    setConvLoading(true)
    try {
      const messages = await listMemberConversationMessages(
        activeProjectId,
        activeConversationTargetId,
        convId,
      )
      setConvMessages(messages as Message[])
    } catch (err) { console.warn('[ConvMessages] failed:', err) }
    finally { setConvLoading(false) }
  }

  useEffect(() => {
    if (!activeProjectId || !activeConversationTargetId || !sessionView || sessionView === 'new') return
    let canceled = false
    let refreshTimer: number | undefined

    function clearRefreshTimer() {
      if (refreshTimer) {
        window.clearTimeout(refreshTimer)
        refreshTimer = undefined
      }
    }

    async function refreshConversation() {
      try {
        const nextMessages = await listMemberConversationMessages(
          activeProjectId,
          activeConversationTargetId,
          String(sessionView),
        )
        if (canceled) return
        setConvMessages(nextMessages as Message[])
        listMemberConversations(activeProjectId, activeConversationTargetId)
          .then((items) => { if (!canceled) setMemberConversations(items) })
          .catch(() => {})
      } catch (err) {
        console.warn('[ConvMessages] refresh failed:', err)
      }
    }

    function scheduleConversationRefresh(delay = 160) {
      clearRefreshTimer()
      refreshTimer = window.setTimeout(refreshConversation, delay)
    }

    function matchesCurrentConversation(detail: ProjectRealtimeDetail | undefined) {
      if (!detail) return false
      if (detail.projectId && detail.projectId !== activeProjectId) return false
      return detail.conversationId === sessionView
    }

    function onProjectMessageUpdated(e: Event) {
      const detail = (e as CustomEvent<ProjectRealtimeDetail>).detail
      if (matchesCurrentConversation(detail)) scheduleConversationRefresh()
    }

    function onProjectTaskDone(e: Event) {
      const detail = (e as CustomEvent<ProjectRealtimeDetail>).detail
      if (matchesCurrentConversation(detail)) scheduleConversationRefresh(0)
    }

    window.addEventListener('elon:project-message-updated', onProjectMessageUpdated)
    window.addEventListener('elon:project-task-done', onProjectTaskDone)
    return () => {
      canceled = true
      clearRefreshTimer()
      window.removeEventListener('elon:project-message-updated', onProjectMessageUpdated)
      window.removeEventListener('elon:project-task-done', onProjectTaskDone)
    }
  }, [activeProjectId, activeConversationTargetId, sessionView])

  function startNewSession() {
    if (!isOwnConversationTarget) {
      setSendError('只能为自己的项目会话新建对话')
      return
    }
    prevSessionIdsRef.current = new Set(memberConversations.map((c) => c.id))
    setSessionView('new')
    setConvMessages([])
    waitingForNewSession.current = true
    setTimeout(() => textareaRef.current?.focus(), 50)
  }

  function openSession(convId: string) {
    openConversation(convId)
  }

  function openMemberConversations(member: ProjectMember) {
    const target = targetFromProjectMember(member)
    setMemberConversationTarget(target)
    setSelectedMember(null)
    setMemberMenu(null)
    setMemberPopoverY(200)
    setSendError('')
  }

  function openMemberProfile(member: ProjectMember, y: number) {
    setSelectedMember(member)
    setMemberPopoverY(y)
    setMemberMenu(null)
  }

  function openMemberPermissions(member: ProjectMember) {
    setPermissionFocusMemberId(member.user_id)
    setShowPermissions(true)
    setMemberMenu(null)
  }

  function openMemberRoles(member: ProjectMember) {
    setRoleFocusMemberId(member.user_id)
    setShowRoles(true)
    setSelectedMember(null)
    setMemberMenu(null)
  }

  async function moderateMemberFromPopover(member: ProjectMember, action: MemberModerationAction, durationMinutes?: number) {
    if (!activeProjectId) return
    await api.patch(`/api/projects/${encodeURIComponent(activeProjectId)}/members/${encodeURIComponent(member.user_id)}/moderation`, {
      action,
      duration_minutes: durationMinutes,
      note: 'PC 成员资料卡操作',
    })
    await reloadProjectSpace()
  }

  function resetMemberConversationTarget() {
    setMemberConversationTarget(null)
    setSendError('')
  }

  async function openProjectHome() {
    if (!activeProjectId) return
    setSessionView(null)
    setConvMessages([])
    setMemberConversationTarget(null)
    waitingForNewSession.current = false
    await selectProject(activeProjectId)
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
              <button
                className={styles.workspaceHomeBtn}
                onClick={openProjectHome}
                title="项目首页"
                type="button"
              >
                <strong className={styles.workspaceTitleText}>{activeProject?.name}</strong>
                {activeProject?.description && (
                  <span className={styles.workspaceTitleMeta}>{activeProject.description}</span>
                )}
              </button>
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

              {activeConversationTarget && (
                <MemberConversationList
                  conversations={memberConversations}
                  selectedId={sessionView}
                  targetName={activeConversationTargetName}
                  isOwnTarget={isOwnConversationTarget}
                  onOpen={openSession}
                  onStartNew={startNewSession}
                  onResetTarget={resetMemberConversationTarget}
                />
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

        <SidebarUserStrip />
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

        <div className={styles.chatStatusStack}>
          {activeProjectId && (
            <>
              {/* 节点离线提示：电脑重启后节点未运行时出现 */}
              <NodeOfflineBanner />
              <div className={[
                styles.localNodeNotice,
                !localNodeReady ? styles.localNodeNoticeWarn : projectBoundToLocalNode ? styles.localNodeNoticeOk : styles.localNodeNoticeInfo,
              ].join(' ')}>
                <strong>
                  {localNodeReady
                    ? projectBoundToLocalNode ? '当前电脑节点已锁定' : '当前电脑节点优先'
                    : '未锁定当前电脑节点'}
                </strong>
                <span>
                  {localNodeReady
                    ? `${clean(localNode?.device_name) || '本机'} · ${localNodeId}${localBindStatus ? ` · ${localBindStatus}` : ''}`
                    : localNodeError || '请确认 Windows 节点助手正在运行并已登录当前账号'}
                </span>
              </div>
              <div className={styles.projectRouteNotice}>
                <span>
                  <strong>当前项目</strong>
                  {activeProject?.name ?? activeProjectId} · {activeProjectRoleLabel}
                  {activeChannel ? ` · ${activeChannel.name}` : ' · 默认 AI开发频道'}
                </span>
                <span>
                  {projectBoundToLocalNode
                    ? '会使用本机节点'
                    : activeProject?.node_id
                      ? `项目记录绑定 ${shortNodeId(activeProject.node_id)}`
                      : '项目尚未记录节点'}
                </span>
              </div>
              {(activeChannelBlocksAi || activeChannelIsNotAi) && (
                <div className={styles.permissionNotice}>
                  {activeChannelIsNotAi
                    ? '当前频道不是 AI 开发频道，请切换到 AI开发 后发起 AI 对话。'
                    : '当前角色不能在这个频道发起 AI 开发。'}
                </div>
              )}
            </>
          )}
        </div>

        {/* 消息列表（1fr）*/}
        {/* 无频道或未选中会话（landing）vs 选中会话（feed）*/}
        {sessionView === null ? (
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
              isAssistingMember ? (
                <div className={styles.emptyState}>
                  <strong>{activeConversationTargetName} 的项目会话</strong>
                  <p>从左侧选择一个公开会话后，你可以用自己的账号继续协助他。</p>
                </div>
              ) : activeProject && (
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
              {/* AI 来源和模型选择按钮 */}
              <button
                ref={modelBtnRef}
                className={styles.composerModelBtn}
                type="button"
                title={modelButtonCopy.title}
                onClick={() => setShowModelPicker((v) => !v)}
              >
                <span>{modelButtonCopy.source}</span>
                <strong>{modelButtonCopy.detail}</strong>
              </button>

              {/* Textarea */}
              <textarea
                ref={textareaRef}
                className={styles.composerTextarea}
                value={input}
                onChange={(e) => { setInput(e.target.value); autoResize() }}
                onKeyDown={handleKeyDown}
                placeholder={
                  isAssistingMember
                    ? `以我的账号在 ${activeConversationTargetName} 的会话中发送协助消息…`
                    : !activeChannelId
                    ? `向 ${activeProject?.name ?? '项目'} 发送消息或需求… (Enter 发送)`
                    : activeChannelIsNotAi
                      ? '请选择 AI开发 频道后发起 AI 对话'
                    : isDevChannel
                      ? `向 ${activeChannel?.name ?? 'AI'} 描述开发需求… (Enter 发送，Shift+Enter 换行)`
                      : `在 #${activeChannel?.name ?? ''} 发送消息`
                }
                disabled={composerDisabled}
                rows={1}
              />

              {/* P1.4：附件按钮 */}
              {activeProjectId && (
                <AttachmentButton
                  projectId={activeProjectId}
                  disabled={composerDisabled}
                  onAttached={(att) => setAttachments((prev) => [...prev, att])}
                />
              )}

              {/* 发送按钮 */}
              <button
                className={styles.sendBtn}
                type="submit"
                disabled={(!input.trim() && attachments.length === 0) || composerDisabled}
              >
                {sendingMessage || sendingMemberDiscussion ? '…' : '发送'}
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
            {activeProjectId && canUseRoleManager && <button className={styles.memberInviteBtn} type="button" onClick={() => { setRoleFocusMemberId(''); setShowRoles(true) }}>角色</button>}
            {activeProjectId && canViewMemberAudit && <button className={styles.memberInviteBtn} type="button" onClick={() => setShowAudit(true)}>日志</button>}
            {activeProjectId && activeChannelId && canManagePermissions && (
              <button className={styles.memberInviteBtn} type="button" onClick={() => { setPermissionFocusMemberId(''); setShowPermissions(true) }}>权限</button>
            )}
          </div>
        </div>
        <div className={styles.memberList}>
          {memberMenu && createPortal(
            <MemberContextMenu
              member={memberMenu.member}
              x={memberMenu.x}
              y={memberMenu.y}
              canModerate={canModerateMembers && memberMenu.member.user_id !== user?.id}
              onClose={() => setMemberMenu(null)}
              onOpenProfile={openMemberProfile}
              onOpenConversations={openMemberConversations}
              onOpenPermissions={activeProjectId && activeChannelId && canManagePermissions ? openMemberPermissions : undefined}
              onOpenRoles={activeProjectId && canUseRoleManager ? openMemberRoles : undefined}
              onModerate={moderateMemberFromPopover}
            />,
            document.body
          )}
          {selectedMember && createPortal(
            <MemberProfilePopover
              member={selectedMember}
              anchorY={memberPopoverY}
              channel={activeChannel}
              canModerate={canModerateMembers && selectedMember.user_id !== user?.id}
              onClose={() => setSelectedMember(null)}
              onOpenConversations={openMemberConversations}
              onOpenRoles={canUseRoleManager ? openMemberRoles : undefined}
              onModerate={moderateMemberFromPopover}
            />,
            document.body
          )}
          {activeProjectId && (
            isDevChannel && activeWorkspacePath ? (
              <div className={styles.agentRunsSlot}>
                <AgentRunsPanel workspacePath={activeWorkspacePath} />
              </div>
            ) : null
          )}
          {activeProjectId && (
            <MemberContextSummary
              label={memberPanelSummary}
              members={panelMembers}
              channel={activeChannel}
              projectTotal={spaceMembers.length}
              usingChannelPermissions={hasChannelMemberPermissions}
            />
          )}
          {activeProjectId && spaceLoading && panelMembers.length === 0 && (
            <MemberLoadingRows />
          )}
          {activeProjectId && !spaceLoading && spaceError && (
            <p className={styles.sideHint}>{spaceError}</p>
          )}
          {activeProjectId && panelMembers.length > 0 && (
              <MemberSearch
                members={panelMembers}
                onSelect={(m, y) => { setSelectedMember(m); setMemberPopoverY(y) }}
                onOpenConversations={openMemberConversations}
                onOpenMenu={setMemberMenu}
                activeConversationMemberId={isAssistingMember ? activeConversationTargetId : null}
                placeholder={activeChannel ? '搜索频道成员' : '搜索项目成员'}
              channelId={activeChannelId ?? undefined}
            />
          )}
          {activeProjectId && !spaceLoading && !spaceError && panelMembers.length === 0 && (
            <p className={styles.sideHint}>{activeChannel ? '暂无可见频道成员' : '暂无项目成员'}</p>
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
        <ModelPickerPopover
          anchorRef={modelBtnRef}
          runtimeRoute={runtimeRoute}
          onRuntimeRouteChange={setRuntimeRoute}
          onClose={() => setShowModelPicker(false)}
        />
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
      {showAudit && activeProjectId && (
        <MemberAuditDrawer projectId={activeProjectId} onClose={() => setShowAudit(false)} />
      )}
      {showRoles && activeProjectId && (
        <RoleManagementDrawer
          projectId={activeProjectId}
          members={members}
          currentUserId={user?.id}
          initialMemberId={roleFocusMemberId}
          canManageRoles={canManageRoles}
          canManageMembers={canManageMembers}
          onClose={() => { setShowRoles(false); setRoleFocusMemberId('') }}
          onSaved={reloadProjectSpace}
        />
      )}
      {showPermissions && activeProjectId && activeChannelId && (
        <PermissionDrawer
          projectId={activeProjectId}
          activeChannelId={activeChannelId}
          initialMemberId={permissionFocusMemberId}
          channels={channels}
          categories={categories}
          members={members}
          onClose={() => { setShowPermissions(false); setPermissionFocusMemberId('') }}
          onSaved={reloadProjectSpace}
        />
      )}
    </div>
  )
}

function titleFromMessage(message: string): string {
  const title = message.replace(/\s+/g, ' ').trim()
  if (!title) return '新会话'
  return title.length > 24 ? `${title.slice(0, 24)}...` : title
}

function mergeProjectRecords(listedProject?: Project, spaceProject?: Project | null): Project | undefined {
  if (!listedProject) return spaceProject ?? undefined
  if (!spaceProject) return listedProject
  return {
    ...listedProject,
    ...spaceProject,
    source_type: spaceProject.source_type ?? listedProject.source_type,
    workspace_path: spaceProject.workspace_path ?? listedProject.workspace_path,
    storage_worktree_path: spaceProject.storage_worktree_path ?? listedProject.storage_worktree_path,
    node_id: spaceProject.node_id ?? listedProject.node_id,
    role: spaceProject.role ?? listedProject.role,
    my_role: spaceProject.my_role ?? listedProject.my_role,
    runtime_permission: spaceProject.runtime_permission ?? listedProject.runtime_permission,
  }
}

function channelAllowsAiStart(channel?: Channel | null): boolean {
  if (!channel) return false
  if (channel.kind !== 'ai_development') return false
  const permissions = channel.permissions
  if (!permissions) return true
  return Boolean(permissions.can_start_ai ?? permissions.canStartAi)
}

function projectRoleCanAutoBind(role: string): boolean {
  return role === 'owner'
}

function projectRoleLabel(role: string): string {
  if (role === 'owner') return 'Owner'
  if (role === 'admin') return 'Admin'
  if (role === 'editor') return '协作者'
  if (role === 'member') return '成员'
  if (role === 'observer') return '只读'
  return role || '未知角色'
}

function shortNodeId(nodeId: string): string {
  const cleanId = clean(nodeId)
  if (cleanId.length <= 18) return cleanId
  return `${cleanId.slice(0, 11)}…${cleanId.slice(-6)}`
}
