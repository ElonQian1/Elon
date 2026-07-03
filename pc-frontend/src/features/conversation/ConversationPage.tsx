import { useEffect, useMemo, useRef, useState, useCallback } from 'react'
import { createPortal } from 'react-dom'
import { useNavigate } from 'react-router-dom'
import { Cpu, History, RefreshCw, Smartphone } from 'lucide-react'
import { v4 as uuidv4 } from 'uuid'
import { useProjectStore } from './useProjectStore'
import { useChannelAutoRefresh } from './useChannelAutoRefresh'
import { useMemberRealtimeRefresh } from './useMemberRealtimeRefresh'
import { AttachmentButton, AttachmentChip, attachmentsToMarkdown } from './AttachmentButton'
import type { UploadedAttachment } from './AttachmentButton'
import { useAuthStore } from '../../store/auth'
import { useModelStore } from '../models/useModelStore'
import { ModelPickerPopover } from '../models/ModelPicker'
import AgentRunsPanel from '../dev/AgentRunsPanel'
import { buildContext } from '../dev/devTaskUtils'
import { CreateProjectModal } from '../projects/CreateProjectModal'
import ProjectLanding from './ProjectLanding'
import NodeOfflineBanner from './NodeOfflineBanner'
import ConversationFeed from './ConversationFeed'
import { api } from '../../api/client'
import { clean, safeNodeAdminUrl } from '../../lib/utils'
import { localJson } from '../doctor/localApi'
import {
  routeModelButtonCopy,
  selectedAgentForRuntimeRoute,
} from '../models/routeModelPolicy'
import { initialProjectRuntimeRouteFromStorage, persistProjectRuntimeRouteSelection } from './runtimeRoutes'
import type { RuntimeRoute } from './runtimeRoutes'
import {
  ensureLocalFullAccessGrant,
  initialDirectPcCliFromStorage,
  persistDirectPcCliSelection,
} from './localPcRuntime'
import type { LocalNodeStatus } from './localPcRuntime'
import type {
  Channel,
  ChannelMessagesResponse,
  Message,
  Project,
  ProjectInvitePreview,
  ProjectInvitePreviewResponse,
  ProjectMember,
  UserPresenceSettings,
} from './types'
import MemberConversationList from './MemberConversationList'
import {
  buildDisplayMessages,
  buildMessageGroups,
  buildTaskProcessMessageMap,
  containsTaskProcess,
  hasRunningTask as hasRunningTaskInMessages,
} from './messageFlow'
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
  presenceLabel,
  membersHaveChannelPermissionMap,
  membersForChannel,
  projectMemberHasRolePermission,
  ROLE_PERMISSION_INVITE_MEMBERS,
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
import { MemberDetailDrawer } from './MemberDetailDrawer'
import { MemberDirectoryDrawer } from './MemberDirectoryDrawer'
import {
  MemberSearch,
  MemberContextMenu,
  MemberProfilePopover,
  MemberContextSummary,
  MemberLoadingRows,
  memberPresenceAvatarClass,
} from './MemberPanel'
import type { MemberMenuRequest, MemberModerationAction } from './MemberPanel'
import SidebarUserStrip from '../shell/SidebarUserStrip'
import styles from './ConversationPage.module.css'

type MemberPanelScope = 'channel' | 'project'

interface ProjectRealtimeDetail {
  projectId?: string
  channelId?: string
  conversationId?: string
  taskId?: string
  kind?: string
}

interface ConversationMessageCacheEntry {
  messages: Message[]
  taskMessages: Message[]
  loadedAt: number
}

interface TaskMessageCacheEntry {
  messages: Message[]
  loadedAt: number
}

const TASK_MESSAGE_CACHE_FRESH_MS = 4000
const CONVERSATION_CACHE_FRESH_MS = 12000
const CACHED_CONVERSATION_REFRESH_DELAY_MS = 450

export default function ConversationPage() {
  useChannelAutoRefresh()
  useMemberRealtimeRefresh()
  const navigate = useNavigate()
  const user = useAuthStore((s) => s.user)
  const {
    projects, projectsLoaded, activeProjectId, channels, categories, members, activeChannelId,
    messages, messagesLoading, sendingMessage, space, landing, spaceLoading, spaceError,
    projectHomeVersion,
    loadProjects, selectProject, reloadProjectSpace, selectChannel, sendMessage,
  } = useProjectStore()
  const selectedAgent = useModelStore((s) => s.selectedAgent)
  const modelLabel = useModelStore((s) => s.label)
  const modelOptions = useModelStore((s) => s.options)
  const [input, setInput] = useState('')
  const [sendError, setSendError] = useState('')
  const [showCreate, setShowCreate] = useState(false)
  const [showModelPicker, setShowModelPicker] = useState(false)
  const [runtimeRoute, setRuntimeRoute] = useState<RuntimeRoute>(() => initialProjectRuntimeRouteFromStorage(
    typeof window === 'undefined' ? null : window.localStorage,
  ))
  const [directPcCli, setDirectPcCli] = useState(() => initialDirectPcCliFromStorage(
    typeof window === 'undefined' ? null : window.localStorage,
  ))
  const [showPermissions, setShowPermissions] = useState(false)
  const [showPresence, setShowPresence] = useState(false)
  const [myPresence, setMyPresence] = useState<UserPresenceSettings | null>(null)
  const [showInvites, setShowInvites] = useState(false)
  const [showModeration, setShowModeration] = useState(false)
  const [showAudit, setShowAudit] = useState(false)
  const [showRoles, setShowRoles] = useState(false)
  const [showDirectory, setShowDirectory] = useState(false)
  const [selectedMember, setSelectedMember] = useState<ProjectMember | null>(null)
  const [detailMember, setDetailMember] = useState<ProjectMember | null>(null)
  const [memberPanelScope, setMemberPanelScope] = useState<MemberPanelScope>('project')
  const [moderationFocusMemberId, setModerationFocusMemberId] = useState('')
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
  const [sessionTaskMessages, setSessionTaskMessages] = useState<Message[]>([])
  const [sendingMemberDiscussion, setSendingMemberDiscussion] = useState(false)
  const [inviteCode, setInviteCode] = useState('')
  const [invitePreview, setInvitePreview] = useState<ProjectInvitePreview | null>(null)
  const [inviteStatus, setInviteStatus] = useState('')
  const [channelSearch, setChannelSearch] = useState('')
  const [showNewMsg, setShowNewMsg] = useState(false)
  const [attachments, setAttachments] = useState<UploadedAttachment[]>([])   // P1.4   // P1.3：新消息提示
  const feedRef = useRef<HTMLDivElement>(null)
  const scrollFrameRef = useRef<number | null>(null)
  const textareaRef = useRef<HTMLTextAreaElement>(null)
  const modelBtnRef = useRef<HTMLButtonElement>(null)
  const atBottomRef = useRef(true)   // P1.3：用户是否在底部
  // 会话视图模式：null=默认(全部) / 'new'=新建空会话 / string=会话 ID
  const [sessionView, setSessionView] = useState<string | 'new' | null>(null)
  const prevSessionIdsRef = useRef<Set<string>>(new Set())
  const waitingForNewSession = useRef(false)
  const conversationMessageCacheRef = useRef<Map<string, ConversationMessageCacheEntry>>(new Map())
  const taskMessageCacheRef = useRef<Map<string, TaskMessageCacheEntry>>(new Map())
  const conversationLoadSeqRef = useRef(0)
  const loadCachedTaskMessages = useCallback(async (
    projectId: string,
    channelId: string,
    force = false,
  ): Promise<Message[]> => {
    if (!projectId || !channelId) return []
    const key = taskMessageCacheKey(projectId, channelId)
    const cached = taskMessageCacheRef.current.get(key)
    if (!force && cached && Date.now() - cached.loadedAt < TASK_MESSAGE_CACHE_FRESH_MS) {
      return cached.messages
    }
    const messages = await loadAiDevelopmentTaskMessages(projectId, channelId)
    taskMessageCacheRef.current.set(key, { messages, loadedAt: Date.now() })
    return messages
  }, [])
  const handleRuntimeRouteChange = useCallback((route: RuntimeRoute) => {
    setRuntimeRoute(route)
    setDirectPcCli(false)
  }, [])
  const handlePresenceSaved = useCallback(async (presence: UserPresenceSettings) => {
    setMyPresence(presence)
    await reloadProjectSpace()
  }, [reloadProjectSpace])

  useEffect(() => { loadProjects() }, [user?.id])

  useEffect(() => {
    let canceled = false
    if (!user?.id) {
      setMyPresence(null)
      return () => { canceled = true }
    }
    api.get<UserPresenceSettings>('/api/me/presence')
      .then((presence) => {
        if (!canceled) setMyPresence(presence)
      })
      .catch(() => {
        if (!canceled) setMyPresence(null)
      })
    return () => { canceled = true }
  }, [user?.id])

  useEffect(() => {
    if (!user?.id) return
    const userId = user.id
    function onPresence(event: Event) {
      const detail = (event as CustomEvent<{
        userId?: string
        status?: string
        customStatus?: string | null
        custom_status?: string | null
        activity?: string | null
        updatedAt?: string
        updated_at?: string
      }>).detail
      if (!detail || detail.userId !== userId) return
      setMyPresence((prev) => ({
        user_id: userId,
        status: detail.status ?? prev?.status ?? 'online',
        custom_status: Object.prototype.hasOwnProperty.call(detail, 'customStatus')
          || Object.prototype.hasOwnProperty.call(detail, 'custom_status')
          ? detail.customStatus ?? detail.custom_status ?? null
          : prev?.custom_status ?? null,
        activity: Object.prototype.hasOwnProperty.call(detail, 'activity')
          ? detail.activity ?? null
          : prev?.activity ?? null,
        updated_at: detail.updatedAt ?? detail.updated_at ?? prev?.updated_at,
      }))
    }
    window.addEventListener('elon:presence', onPresence)
    return () => window.removeEventListener('elon:presence', onPresence)
  }, [user?.id])

  useEffect(() => {
    persistProjectRuntimeRouteSelection(window.localStorage, runtimeRoute)
  }, [runtimeRoute])

  useEffect(() => {
    persistDirectPcCliSelection(window.localStorage, directPcCli)
  }, [directPcCli])

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
  }, [activeProjectId, activeConversationTargetId])

  // 项目切换时清空会话消息
  useEffect(() => {
    conversationLoadSeqRef.current += 1
    conversationMessageCacheRef.current.clear()
    taskMessageCacheRef.current.clear()
    setConvMessages([])
    setSessionTaskMessages([])
    setSessionView(null)
    setMemberConversationTarget(null)
    waitingForNewSession.current = false
  }, [activeProjectId, projectHomeVersion])

  useEffect(() => {
    setSelectedMember(null)
    setMemberMenu(null)
    setDetailMember(null)
    setPermissionFocusMemberId('')
    setRoleFocusMemberId('')
    setModerationFocusMemberId('')
    setShowAudit(false)
    setShowRoles(false)
    setShowDirectory(false)
    setMemberPanelScope(activeChannelId ? 'channel' : 'project')
  }, [activeProjectId, activeChannelId])

  useEffect(() => {
    if (!selectedMember?.user_id) return
    const fresh = members.find((member) => member.user_id === selectedMember.user_id)
    if (fresh && fresh !== selectedMember) setSelectedMember(fresh)
    if (!fresh) setSelectedMember(null)
  }, [members, selectedMember])

  useEffect(() => {
    if (!detailMember?.user_id) return
    const fresh = members.find((member) => member.user_id === detailMember.user_id)
    if (fresh && fresh !== detailMember) setDetailMember(fresh)
    if (!fresh) setDetailMember(null)
  }, [members, detailMember])

  useEffect(() => {
    if (!memberMenu?.member.user_id) return
    const fresh = members.find((member) => member.user_id === memberMenu.member.user_id)
    if (fresh && fresh !== memberMenu.member) setMemberMenu({ ...memberMenu, member: fresh })
    if (!fresh) setMemberMenu(null)
  }, [members, memberMenu])

  useEffect(() => {
    conversationLoadSeqRef.current += 1
    setSessionView(null)
    setConvMessages([])
    setSessionTaskMessages([])
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

  // P1.3：智能滚动——延后一帧读写 scrollHeight，避免切换会话时同步强制布局。
  useEffect(() => {
    if (scrollFrameRef.current) {
      window.cancelAnimationFrame(scrollFrameRef.current)
      scrollFrameRef.current = null
    }
    scrollFrameRef.current = window.requestAnimationFrame(() => {
      scrollFrameRef.current = null
      const el = feedRef.current
      if (!el) return
      if (atBottomRef.current) {
        el.scrollTop = el.scrollHeight
        setShowNewMsg((visible) => visible ? false : visible)
      } else {
        setShowNewMsg((visible) => visible || true)
      }
    })
    return () => {
      if (scrollFrameRef.current) {
        window.cancelAnimationFrame(scrollFrameRef.current)
        scrollFrameRef.current = null
      }
    }
  }, [messages, convMessages, sessionTaskMessages, sessionView])

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

  const autoResize = useCallback(() => {
    const el = textareaRef.current
    if (!el) return
    el.style.height = '42px'
    el.style.height = Math.min(el.scrollHeight, 120) + 'px'
    el.style.overflowY = el.scrollHeight > 120 ? 'auto' : 'hidden'
  }, [])

  const writeConversationCache = useCallback((
    projectId: string,
    targetUserId: string,
    conversationId: string,
    nextMessages: Message[],
    nextTaskMessages: Message[],
  ) => {
    conversationMessageCacheRef.current.set(
      conversationMessageCacheKey(projectId, targetUserId, conversationId),
      { messages: nextMessages, taskMessages: nextTaskMessages, loadedAt: Date.now() },
    )
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
        setConvMessages((prev) => {
          const next = [...prev, message as MemberConversationMessage]
          writeConversationCache(activeProjectId, activeConversationTargetId, String(sessionView), next, sessionTaskMessages)
          return next
        })
        listMemberConversations(activeProjectId, activeConversationTargetId)
          .then(setMemberConversations)
          .catch(() => {})
        return
      }

      let targetChannel = activeChannel?.kind === 'ai_development' ? activeChannel : aiDevelopmentChannel
      let targetChannelId = targetChannel?.id ?? ''
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

      const directPcCliForRequest = directPcCliActive
      const requestRuntimeRoute: RuntimeRoute = directPcCliForRequest ? 'route_a' : runtimeRoute
      const useLocalNodeForRequest = (directPcCliForRequest || shouldPreferLocalNode) && localNodeReady
      const requestAgent = selectedAgentForRuntimeRoute(selectedAgent, modelOptions, requestRuntimeRoute)
      await ensureLocalFullAccessGrant({
        adminUrl: safeNodeAdminUrl(),
        projectId: activeProjectId,
        projectName: activeProject?.name,
        workspacePath: activeWorkspacePath,
        runtimePermission: activeProject?.runtime_permission,
        useLocalRouteA: useLocalNodeForRequest && requestRuntimeRoute === 'route_a',
      })
      clearComposerDraft()
      const isExistingConversation = typeof sessionView === 'string' && sessionView !== 'new'
      const conversationId = isExistingConversation ? sessionView : uuidv4()
      const conversationTitle = isExistingConversation ? null : titleFromMessage(text)
      const response = await sendMessage(
        fullContent,
        requestAgent || null,
        requestRuntimeRoute,
        conversationId,
        conversationTitle,
        useLocalNodeForRequest ? localNodeId : null,
        useLocalNodeForRequest ? activeWorkspacePath : null,
        targetChannelId,
        directPcCliForRequest,
      )
      const openedConversationId = response?.conversation_id ?? conversationId
      waitingForNewSession.current = false
      setSessionView(openedConversationId)
      setSessionTaskMessages([])
      const optimisticTaskId = clean(response?.task_id ?? response?.message?.task_id ?? response?.message?.taskId)
      const optimisticMessages: Message[] = [{
        id: `optimistic-${openedConversationId}-${Date.now()}`,
        role: 'user',
        content: fullContent,
        created_at: new Date().toISOString(),
        user_id: user?.id,
        sender_name: user?.nickname ?? user?.account ?? '我',
        outgoing: true,
        task_id: optimisticTaskId || undefined,
      } as Message]
      setConvMessages(optimisticMessages)
      if (activeProjectId && activeConversationTargetId) {
        writeConversationCache(activeProjectId, activeConversationTargetId, openedConversationId, optimisticMessages, [])
      }
      // 发送后刷新会话列表和当前会话消息，保证继续输入时仍在同一上下文。
      if (activeProjectId && activeConversationTargetId) {
        setTimeout(async () => {
          try {
            const conversations = await listMemberConversations(activeProjectId, activeConversationTargetId)
            setMemberConversations(conversations)
            await openConversation(openedConversationId, { force: true })
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
    if (textareaRef.current) { textareaRef.current.style.height = '42px' }
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
  const aiDevelopmentChannel = channels.find((channel) => channel.kind === 'ai_development')
  const aiDevelopmentChannelId = aiDevelopmentChannel?.id ?? ''
  const taskActionChannelId = isDevChannel ? activeChannelId : aiDevelopmentChannelId
  const activeWorkspacePath = clean(activeProject?.workspace_path ?? activeProject?.storage_worktree_path)
  const activeProjectRole = clean(activeProject?.role ?? activeProject?.my_role ?? space?.my_role).toLowerCase()
  const activeProjectRoleLabel = projectRoleLabel(activeProjectRole)
  const localNodeId = clean(localNode?.agent_id)
  const localNodeOwnerOk = !!localNodeId && !!user?.id && clean(localNode?.owner_user_id) === user.id
  const localNodeReady = localNodeOwnerOk
    && localNode?.connected !== false
    && localNode?.codex_cli?.available !== false
  const shouldPreferLocalNode = !['route_c2', 'route_c3'].includes(runtimeRoute)
  const directPcCliAvailable = !!activeProjectId
    && !isAssistingMember
    && localNodeReady
  const directPcCliActive = directPcCli && directPcCliAvailable
  const composerRuntimeRoute: RuntimeRoute = directPcCliActive ? 'route_a' : runtimeRoute
  const modelButtonCopy = useMemo(
    () => routeModelButtonCopy(composerRuntimeRoute, modelLabel, modelOptions, selectedAgent),
    [composerRuntimeRoute, modelLabel, modelOptions, selectedAgent],
  )
  const projectBoundToLocalNode = !!localNodeId && activeProject?.node_id === localNodeId
  const activeChannelBlocksAi = !!activeChannel && activeChannel.kind === 'ai_development' && !channelAllowsAiStart(activeChannel)
  const activeChannelIsNotAi = !!activeChannel && activeChannel.kind !== 'ai_development'
  const aiDevelopmentChannelBlocksAi = !!aiDevelopmentChannel && !channelAllowsAiStart(aiDevelopmentChannel)
  const canManagePermissions = channels.some(channelCanManage)
  // taskContext 和 hasRunningTask 在当前实际渲染的消息流上推导，避免会话视图丢失折叠状态。

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
  const canInviteMembers = !!activeProjectId
    && !!currentProjectMember
    && projectMemberHasRolePermission(currentProjectMember, [], ROLE_PERMISSION_INVITE_MEMBERS)
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
  const activeMemberPanelScope: MemberPanelScope = activeChannelId && memberPanelScope === 'channel' ? 'channel' : 'project'
  const panelUsesChannelScope = activeMemberPanelScope === 'channel'
  const panelUsesChannelPermissions = panelUsesChannelScope && hasChannelMemberPermissions
  const panelMembers = useMemo(
    () => panelUsesChannelScope ? membersForChannel(spaceMembers, activeChannelId) : spaceMembers,
    [spaceMembers, activeChannelId, panelUsesChannelScope],
  )
  const memberPanelTitle = panelUsesChannelScope ? '频道成员' : activeProjectId ? '项目大厅' : '工作台'
  const memberPanelContext = panelUsesChannelScope
    ? activeChannel?.name ?? '当前频道'
    : activeProject?.name ?? '我的项目'
  const memberPanelCount = activeProjectId ? panelMembers.length : (user ? 1 : 0)
  const ownPresenceStatus = normalizeOwnPresenceStatus(myPresence?.status ?? user?.status ?? 'online')
  const ownPresenceAvatarStatus = ownPresenceStatus === 'invisible' ? 'offline' : ownPresenceStatus
  const ownPresenceSubtitle = ownPresenceSummary(myPresence, ownPresenceStatus)
  const ownAvatarUrl = clean(user?.avatar_data_url ?? '')
  const ownDisplayName = user ? (user.nickname ?? user.account) : ''
  const ownInitial = ownDisplayName ? ownDisplayName[0].toUpperCase() : '?'
  const memberPanelSummary = panelUsesChannelScope && activeChannel
    ? panelUsesChannelPermissions
      ? channelPermissionSummary(activeChannel, panelMembers.length, spaceMembers.length, true)
      : `${activeChannel.name} · 当前频道未设置成员级可见限制，显示项目内全部成员`
    : activeProjectId
      ? `项目大厅显示 ${spaceMembers.length} 位项目成员，适合查看全局在线、角色和管理状态`
      : '个人 AI 工作台'

  // 成员卡片弹窗
  // (memberPopover state removed - not currently used)

  const memberDiscussionNeedsConversation = isAssistingMember && (!sessionView || sessionView === 'new')
  const composerBusy = sendingMessage || sendingMemberDiscussion
  const composerDisabled = composerBusy
    || memberDiscussionNeedsConversation
    || aiDevelopmentChannelBlocksAi
    || (!aiDevelopmentChannelId && !isAssistingMember)
    || (!activeChannelId && channels.length === 0 && !isAssistingMember)

  const channelTaskMessagesById = useMemo(
    () => buildTaskProcessMessageMap([messages, sessionTaskMessages]),
    [messages, sessionTaskMessages],
  )

  // 根据会话视图过滤显示的消息，并把频道任务过程挂回独立会话。
  const displayMessages = useMemo(
    () => buildDisplayMessages({
      sessionView,
      channelMessages: messages,
      conversationMessages: convMessages,
      conversationLoading: convLoading,
      taskMessagesById: channelTaskMessagesById,
    }),
    [messages, sessionView, convMessages, convLoading, channelTaskMessagesById],
  )

  const taskContext = useMemo(
    () => buildContext(displayMessages as Parameters<typeof buildContext>[0]),
    [displayMessages],
  )

  // P1.3：打字指示器只看当前可见会话，避免其它历史任务让本会话一直显示处理中。
  const hasRunningTask = useMemo(() => hasRunningTaskInMessages(displayMessages), [displayMessages])

  // 消息分组：dev频道中把同一 task_id 的消息聚合为 DevTaskGroup（任务级折叠层）
  const taskFlowEnabled = isDevChannel || containsTaskProcess(displayMessages)
  const messageGroups = useMemo(
    () => buildMessageGroups(displayMessages, taskFlowEnabled),
    [displayMessages, taskFlowEnabled],
  )
  const feedLoading = !!sessionView
    && sessionView !== 'new'
    && (convLoading || (messagesLoading && displayMessages.length === 0))

  // sessionView='new' 时，一旦会话列表出现新会话，自动切入
  useEffect(() => {
    if (!waitingForNewSession.current) return
    const newConv = memberConversations.find((c) => !prevSessionIdsRef.current.has(c.id))
    if (newConv) {
      waitingForNewSession.current = false
      openConversation(newConv.id)
    }
  }, [memberConversations])

  // 切换频道时重置会话视图
  useEffect(() => {
    conversationLoadSeqRef.current += 1
    setSessionView(null)
    setConvMessages([])
    setSessionTaskMessages([])
    waitingForNewSession.current = false
  }, [activeChannelId])

  // 打开一个会话：从服务端加载该会话的消息（与手机端同步）
  async function openConversation(convId: string, options: { force?: boolean } = {}) {
    if (!activeProjectId || !activeConversationTargetId) return
    const projectId = activeProjectId
    const targetUserId = activeConversationTargetId
    const channelId = aiDevelopmentChannelId
    const cacheKey = conversationMessageCacheKey(projectId, targetUserId, convId)
    const cached = conversationMessageCacheRef.current.get(cacheKey)
    const cacheAge = cached ? Date.now() - cached.loadedAt : Number.POSITIVE_INFINITY
    const requestSeq = conversationLoadSeqRef.current + 1
    conversationLoadSeqRef.current = requestSeq

    setSessionView(convId)
    if (cached) {
      setConvMessages(cached.messages)
      setSessionTaskMessages(cached.taskMessages)
      setConvLoading(false)
    } else {
      setConvMessages([])
      setSessionTaskMessages([])
      setConvLoading(true)
    }

    if (cached && !options.force && cacheAge < CONVERSATION_CACHE_FRESH_MS) {
      return
    }

    if (cached && !options.force) {
      await delay(CACHED_CONVERSATION_REFRESH_DELAY_MS)
      if (conversationLoadSeqRef.current !== requestSeq) return
    }

    try {
      const [conversationMessages, taskMessages] = await Promise.all([
        listMemberConversationMessages(
          projectId,
          targetUserId,
          convId,
        ),
        loadCachedTaskMessages(projectId, channelId, !!options.force),
      ])
      if (conversationLoadSeqRef.current !== requestSeq) return
      const nextMessages = conversationMessages as Message[]
      if (cached && sameMessageList(cached.messages, nextMessages) && sameMessageList(cached.taskMessages, taskMessages)) {
        conversationMessageCacheRef.current.set(cacheKey, { ...cached, loadedAt: Date.now() })
        return
      }
      writeConversationCache(projectId, targetUserId, convId, nextMessages, taskMessages)
      setConvMessages((prev) => sameMessageList(prev, nextMessages) ? prev : nextMessages)
      setSessionTaskMessages((prev) => sameMessageList(prev, taskMessages) ? prev : taskMessages)
    } catch (err) { console.warn('[ConvMessages] failed:', err) }
    finally {
      if (conversationLoadSeqRef.current === requestSeq) setConvLoading(false)
    }
  }

  async function refreshTaskSurface() {
    if (activeProjectId && activeConversationTargetId && sessionView && sessionView !== 'new') {
      const conversationId = String(sessionView)
      const [nextMessages, taskMessages] = await Promise.all([
        listMemberConversationMessages(
          activeProjectId,
          activeConversationTargetId,
          conversationId,
        ),
        loadCachedTaskMessages(activeProjectId, aiDevelopmentChannelId, true),
      ])
      const conversationMessages = nextMessages as Message[]
      writeConversationCache(activeProjectId, activeConversationTargetId, conversationId, conversationMessages, taskMessages)
      setConvMessages((prev) => sameMessageList(prev, conversationMessages) ? prev : conversationMessages)
      setSessionTaskMessages((prev) => sameMessageList(prev, taskMessages) ? prev : taskMessages)
      listMemberConversations(activeProjectId, activeConversationTargetId)
        .then(setMemberConversations)
        .catch(() => {})
    } else if (activeProjectId && aiDevelopmentChannelId) {
      setSessionTaskMessages(await loadCachedTaskMessages(activeProjectId, aiDevelopmentChannelId, true))
    }

    if (activeProjectId && activeChannelId) {
      await useProjectStore.getState().loadMessages(activeProjectId, activeChannelId)
    }
  }

  async function handleCancelTask(taskId: string) {
    if (!activeProjectId || !taskActionChannelId) {
      setSendError('当前项目没有可操作的 AI 开发频道')
      return
    }
    await api.post(
      `/api/projects/${encodeURIComponent(activeProjectId)}/channels/${encodeURIComponent(taskActionChannelId)}/ai-tasks/${encodeURIComponent(taskId)}/cancel`,
      {},
    )
    await refreshTaskSurface()
  }

  async function handleApproveTool(taskId: string, approvalId: string, decision: 'approve' | 'deny') {
    if (!activeProjectId || !taskActionChannelId) {
      setSendError('当前项目没有可操作的 AI 开发频道')
      return
    }
    await api.post(
      `/api/projects/${encodeURIComponent(activeProjectId)}/channels/${encodeURIComponent(taskActionChannelId)}/ai-tasks/${encodeURIComponent(taskId)}/tool-approvals/${encodeURIComponent(approvalId)}/decision`,
      { decision },
    )
    await refreshTaskSurface()
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
        const [nextMessages, taskMessages] = await Promise.all([
          listMemberConversationMessages(
            activeProjectId,
            activeConversationTargetId,
            String(sessionView),
          ),
          loadCachedTaskMessages(activeProjectId, aiDevelopmentChannelId, true),
        ])
        if (canceled) return
        const conversationMessages = nextMessages as Message[]
        writeConversationCache(activeProjectId, activeConversationTargetId, String(sessionView), conversationMessages, taskMessages)
        setConvMessages((prev) => sameMessageList(prev, conversationMessages) ? prev : conversationMessages)
        setSessionTaskMessages((prev) => sameMessageList(prev, taskMessages) ? prev : taskMessages)
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
  }, [activeProjectId, activeConversationTargetId, sessionView, aiDevelopmentChannelId, loadCachedTaskMessages, writeConversationCache])

  function startNewSession() {
    if (!isOwnConversationTarget) {
      setSendError('只能为自己的项目会话新建对话')
      return
    }
    conversationLoadSeqRef.current += 1
    prevSessionIdsRef.current = new Set(memberConversations.map((c) => c.id))
    setSessionView('new')
    setConvMessages([])
    setSessionTaskMessages([])
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

  function openMemberDetails(member: ProjectMember) {
    setDetailMember(member)
    setSelectedMember(null)
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
    setDetailMember(null)
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

  async function removeMemberFromProject(member: ProjectMember) {
    if (!activeProjectId) return false
    const name = member.account || member.user_id
    if (!window.confirm(`确定要将 ${name} 移出项目吗？`)) return false
    await api.delete(`/api/projects/${encodeURIComponent(activeProjectId)}/members/${encodeURIComponent(member.user_id)}`)
    setSelectedMember((current) => current?.user_id === member.user_id ? null : current)
    setDetailMember((current) => current?.user_id === member.user_id ? null : current)
    setMemberMenu(null)
    await reloadProjectSpace()
    return true
  }

  async function updateMemberProfile(
    member: ProjectMember,
    payload: { display_name?: string | null; admin_note?: string | null },
  ) {
    if (!activeProjectId) return undefined
    const data = await api.patch<{ member?: ProjectMember }>(
      `/api/projects/${encodeURIComponent(activeProjectId)}/members/${encodeURIComponent(member.user_id)}/profile`,
      payload,
    )
    if (data.member) {
      setDetailMember((current) => current?.user_id === member.user_id ? data.member as ProjectMember : current)
      setSelectedMember((current) => current?.user_id === member.user_id ? data.member as ProjectMember : current)
    }
    await reloadProjectSpace()
    return data.member
  }

  function resetMemberConversationTarget() {
    setMemberConversationTarget(null)
    setSendError('')
  }

  async function openProjectHome() {
    if (!activeProjectId) return
    conversationLoadSeqRef.current += 1
    setSessionView(null)
    setConvMessages([])
    setSessionTaskMessages([])
    setMemberConversationTarget(null)
    waitingForNewSession.current = false
    await selectProject(activeProjectId)
  }

  return (
    <div className={styles.layout}>

      {/* ══ 频道面板 ══ */}
      <aside className={styles.channelPanel}>
        {/* 工作区标题 */}
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
        {/* 顶栏 */}
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
              <button className={styles.textBtn} type="button" title="刷新消息" onClick={() => useProjectStore.getState().loadMessages(activeProjectId, activeChannelId)}>
                <RefreshCw size={15} aria-hidden="true" /><span>刷新</span>
              </button>
            )}
            <button className={styles.textBtn} type="button" title="分享这台电脑的算力并查看连接状态" onClick={() => navigate('/node')}>
              <Cpu size={15} aria-hidden="true" /><span>分享算力</span>
            </button>
            <button className={styles.textBtn} type="button" title="打开移动端入口" onClick={() => window.open('/app/download', '_blank', 'noopener')}>
              <Smartphone size={15} aria-hidden="true" /><span>移动端</span>
            </button>
            <button className={styles.textBtn} type="button" title="切换到旧版 PC 工作台" onClick={() => {
              const tok = useAuthStore.getState().token
              if (tok) {
                localStorage.setItem('lodex_token', tok)
                localStorage.setItem('elon_token', tok)
              }
              window.open('/pc-legacy', '_blank', 'noopener')
            }}>
              <History size={15} aria-hidden="true" /><span>旧版</span>
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
              {(activeChannelBlocksAi || (!sessionView && activeChannelIsNotAi)) && (
                <div className={styles.permissionNotice}>
                  {activeChannelIsNotAi
                    ? '当前输入会通过 AI开发 频道发起项目 AI 对话。'
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
          <ConversationFeed
            sessionView={sessionView}
            feedRef={feedRef}
            feedLoading={feedLoading}
            displayMessages={displayMessages}
            messageGroups={messageGroups}
            taskContext={taskContext}
            isDevChannel={isDevChannel}
            user={user}
            hasRunningTask={hasRunningTask}
            sendingMessage={sendingMessage}
            onScroll={handleFeedScroll}
            onCancelTask={handleCancelTask}
            onApproveTool={handleApproveTool}
          />
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

              <label
                className={styles.directCliToggle}
                data-active={directPcCliActive ? 'true' : 'false'}
                data-default-local={shouldPreferLocalNode && localNodeReady ? 'true' : 'false'}
                data-disabled={!directPcCliAvailable || composerDisabled ? 'true' : 'false'}
                title="自动模式会在可用时使用本机节点；打开后强制交给本机 AI CLI"
              >
                <input
                  type="checkbox"
                  checked={directPcCliActive}
                  disabled={!directPcCliAvailable || composerDisabled}
                  onChange={(event) => setDirectPcCli(event.target.checked)}
                />
                <span className={styles.directCliSwitch} aria-hidden="true" />
                <span className={styles.directCliCopy}>
                  <strong>{directPcCliActive ? '直连CLI' : '自动'}</strong>
                  <em>{!directPcCliAvailable ? '未就绪' : directPcCliActive ? '直连' : '自动'}</em>
                </span>
              </label>

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
                    : sessionView && sessionView !== 'new'
                      ? '继续这个项目会话… (Enter 发送，Shift+Enter 换行)'
                    : !activeChannelId
                    ? `向 ${activeProject?.name ?? '项目'} 发送消息或需求… (Enter 发送)`
                    : activeChannelIsNotAi
                      ? '通过 AI开发 频道发送需求'
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

      {/* ══ 成员面板 ══ */}
      <aside className={styles.memberPanel}>
        <div className={styles.memberTitle}>
          <div className={styles.memberTitleCopy}>
            <strong>{memberPanelTitle}{memberPanelCount > 0 ? ` — ${memberPanelCount}` : ''}</strong>
            <span>{memberPanelContext}</span>
          </div>
          <div className={styles.memberActions}>
            <button className={styles.memberInviteBtn} type="button" onClick={() => setShowPresence(true)}>状态</button>
            {activeProjectId && <button className={styles.memberInviteBtn} type="button" onClick={() => setShowDirectory(true)}>目录</button>}
            {activeProjectId && <button className={styles.memberInviteBtn} type="button" onClick={() => navigate(`/projects/${activeProjectId}/members`)}>成员页</button>}
            {activeProjectId && canInviteMembers && <button className={styles.memberInviteBtn} type="button" onClick={() => setShowInvites(true)}>邀请</button>}
            {activeProjectId && <button className={styles.memberInviteBtn} type="button" onClick={() => { setModerationFocusMemberId(''); setShowModeration(true) }}>管理</button>}
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
              canRemove={canManageMembers && memberMenu.member.user_id !== user?.id}
              onClose={() => setMemberMenu(null)}
              onOpenProfile={openMemberProfile}
              onOpenDetails={openMemberDetails}
              onOpenConversations={openMemberConversations}
              onOpenPermissions={activeProjectId && activeChannelId && canManagePermissions ? openMemberPermissions : undefined}
              onOpenRoles={activeProjectId && canUseRoleManager ? openMemberRoles : undefined}
              onModerate={moderateMemberFromPopover}
              onRemove={removeMemberFromProject}
            />,
            document.body
          )}
          {selectedMember && createPortal(
            <MemberProfilePopover
              member={selectedMember}
              anchorY={memberPopoverY}
              projectId={activeProjectId}
              channels={channels}
              channel={activeChannel}
              canModerate={canModerateMembers && selectedMember.user_id !== user?.id}
              canRemove={canManageMembers && selectedMember.user_id !== user?.id}
              onClose={() => setSelectedMember(null)}
              onOpenDetails={openMemberDetails}
              onOpenConversations={openMemberConversations}
              onOpenRoles={canUseRoleManager ? openMemberRoles : undefined}
              onModerate={moderateMemberFromPopover}
              onRemove={removeMemberFromProject}
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
            <div className={styles.memberScopeSwitch} role="group" aria-label="成员列表范围">
              {activeChannelId && (
                <button
                  type="button"
                  data-active={panelUsesChannelScope ? 'true' : undefined}
                  onClick={() => setMemberPanelScope('channel')}
                >
                  当前频道
                </button>
              )}
              <button
                type="button"
                data-active={!panelUsesChannelScope ? 'true' : undefined}
                onClick={() => setMemberPanelScope('project')}
              >
                项目大厅
              </button>
            </div>
          )}
          {activeProjectId && (
            <MemberContextSummary
              title={panelUsesChannelScope ? '当前频道' : '项目大厅'}
              label={memberPanelSummary}
              members={panelMembers}
              channel={panelUsesChannelScope ? activeChannel : undefined}
              projectTotal={spaceMembers.length}
              usingChannelPermissions={panelUsesChannelPermissions}
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
                placeholder={panelUsesChannelScope ? '搜索频道成员' : '搜索项目成员'}
              channelId={panelUsesChannelScope ? activeChannelId ?? undefined : undefined}
            />
          )}
          {activeProjectId && !spaceLoading && !spaceError && panelMembers.length === 0 && (
            <p className={styles.sideHint}>{panelUsesChannelScope ? '暂无可见频道成员' : '暂无项目成员'}</p>
          )}
          {!activeProjectId && user && (
            <>
              <div className={styles.memberSection}>当前账号</div>
              <div className={styles.memberItem}>
                <div className={[styles.memberAvatar, memberPresenceAvatarClass(ownPresenceAvatarStatus)].join(' ')}>
                  {ownAvatarUrl
                    ? <img src={ownAvatarUrl} alt="" style={{ width: '100%', height: '100%', borderRadius: '50%', objectFit: 'cover', display: 'block' }} />
                    : ownInitial
                  }
                </div>
                <div className={styles.memberCopy}>
                  <div className={styles.memberLine}>
                    <strong className={styles.memberItemName}>{ownDisplayName}</strong>
                  </div>
                  <span className={styles.memberSub}>{ownPresenceSubtitle}</span>
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
          onRuntimeRouteChange={handleRuntimeRouteChange}
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
        <PresenceDrawer onClose={() => setShowPresence(false)} onSaved={handlePresenceSaved} />
      )}
      {showInvites && activeProjectId && (
        <InviteDrawer projectId={activeProjectId} onClose={() => setShowInvites(false)} />
      )}
      {showModeration && activeProjectId && (
        <ModerationDrawer
          projectId={activeProjectId}
          members={members}
          initialMemberId={moderationFocusMemberId}
          onClose={() => { setShowModeration(false); setModerationFocusMemberId('') }}
          onSaved={reloadProjectSpace}
        />
      )}
      {showDirectory && activeProjectId && (
        <MemberDirectoryDrawer
          projectId={activeProjectId}
          members={members}
          channels={channels}
          currentUserId={user?.id}
          canManageMembers={canManageMembers}
          canManageRoles={canUseRoleManager}
          canModerate={canModerateMembers}
          onSaved={reloadProjectSpace}
          onClose={() => setShowDirectory(false)}
          onOpenDetails={(member) => {
            setShowDirectory(false)
            openMemberDetails(member)
          }}
          onOpenConversations={(member) => {
            setShowDirectory(false)
            openMemberConversations(member)
          }}
          onOpenRoles={canUseRoleManager ? (member) => {
            setShowDirectory(false)
            openMemberRoles(member)
          } : undefined}
          onOpenModerationCenter={(member) => {
            setShowDirectory(false)
            setModerationFocusMemberId(member?.user_id ?? '')
            setShowModeration(true)
          }}
        />
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
      {detailMember && activeProjectId && (
        <MemberDetailDrawer
          projectId={activeProjectId}
          member={detailMember}
          channels={channels}
          currentChannel={activeChannel}
          canModerate={canModerateMembers && detailMember.user_id !== user?.id}
          canRemove={canManageMembers && detailMember.user_id !== user?.id}
          canEditProfile={canManageMembers && detailMember.user_id !== user?.id}
          canManageRoles={canUseRoleManager}
          canManagePermissions={!!(activeProjectId && activeChannelId && canManagePermissions)}
          onClose={() => setDetailMember(null)}
          onOpenConversations={openMemberConversations}
          onOpenRoles={canUseRoleManager ? openMemberRoles : undefined}
          onOpenPermissions={activeProjectId && activeChannelId && canManagePermissions ? openMemberPermissions : undefined}
          onModerate={moderateMemberFromPopover}
          onRemove={removeMemberFromProject}
          onUpdateProfile={updateMemberProfile}
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

function normalizeOwnPresenceStatus(status: string): string {
  const value = clean(status).toLowerCase()
  if (value === 'idle' || value === 'dnd' || value === 'invisible' || value === 'offline') return value
  return 'online'
}

function ownPresenceSummary(presence: UserPresenceSettings | null, status: string): string {
  const extras = [
    clean(presence?.activity ?? ''),
    clean(presence?.custom_status ?? ''),
  ].filter(Boolean)
  return [presenceLabel(status), ...extras].join(' · ')
}

function shortNodeId(nodeId: string): string {
  const cleanId = clean(nodeId)
  if (cleanId.length <= 18) return cleanId
  return `${cleanId.slice(0, 11)}…${cleanId.slice(-6)}`
}

async function loadAiDevelopmentTaskMessages(projectId: string, channelId: string): Promise<Message[]> {
  if (!projectId || !channelId) return []
  const data = await api.get<ChannelMessagesResponse>(
    `/api/projects/${encodeURIComponent(projectId)}/channels/${encodeURIComponent(channelId)}/messages?limit=200`,
  )
  return data.messages ?? []
}

function conversationMessageCacheKey(projectId: string, targetUserId: string, conversationId: string): string {
  return `${projectId}::${targetUserId}::${conversationId}`
}

function taskMessageCacheKey(projectId: string, channelId: string): string {
  return `${projectId}::${channelId}`
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, ms))
}

function sameMessageList(left: Message[], right: Message[]): boolean {
  if (left === right) return true
  if (left.length !== right.length) return false
  for (let index = 0; index < left.length; index += 1) {
    if (messageFingerprint(left[index]) !== messageFingerprint(right[index])) return false
  }
  return true
}

function messageFingerprint(message: Message | undefined): string {
  if (!message) return ''
  return [
    clean(message.id),
    clean(message.kind ?? message.role ?? (message as Record<string, unknown>).message_kind ?? ''),
    clean(message.task_id ?? message.taskId ?? ''),
    clean(message.task_status ?? message.taskStatus ?? ''),
    clean(message.created_at),
    clean(message.content ?? message.text ?? ''),
  ].join('|')
}
