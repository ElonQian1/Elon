import { useEffect, useMemo, useRef, useState, useCallback } from 'react'
import { useNavigate } from 'react-router-dom'
import { v4 as uuidv4 } from 'uuid'
import { useProjectStore } from './useProjectStore'
import { useChannelAutoRefresh } from './useChannelAutoRefresh'
import { useMemberRealtimeRefresh } from './useMemberRealtimeRefresh'
import { attachmentsToMarkdown } from './AttachmentButton'
import type { UploadedAttachment } from './AttachmentButton'
import { useAuthStore } from '../../store/auth'
import { useModelStore } from '../models/useModelStore'
import { ModelPickerPopover } from '../models/ModelPicker'
import { buildContext } from '../dev/devTaskUtils'
import { CreateProjectModal } from '../projects/CreateProjectModal'
import ConversationComposer from './ConversationComposer'
import ConversationTopbar from './ConversationTopbar'
import ConversationMemberPanel from './ConversationMemberPanel'
import type { MemberPanelScope } from './ConversationMemberPanel'
import ConversationStatusStack from './ConversationStatusStack'
import ConversationContent from './ConversationContent'
import { api } from '../../api/client'
import { clean } from '../../lib/utils'
import {
  routeModelButtonCopy,
  selectedAgentForRuntimeRoute,
} from '../models/routeModelPolicy'
import { initialProjectRuntimeRouteFromStorage, persistProjectRuntimeRouteSelection } from './runtimeRoutes'
import type { RuntimeRoute } from './runtimeRoutes'
import type {
  Message,
  ProjectInvitePreview,
  ProjectInvitePreviewResponse,
  ProjectMember,
} from './types'
import ConversationChannelPanel from './ConversationChannelPanel'
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
import type { MemberMenuRequest, MemberModerationAction } from './MemberPanel'
import {
  channelAllowsAiStart,
  delay,
  initialDirectPcCliFromStorage,
  mergeProjectRecords,
  persistDirectPcCliSelection,
  projectRoleLabel,
  sameMessageList,
  titleFromMessage,
} from './conversationPageUtils'
import {
  CACHED_CONVERSATION_REFRESH_DELAY_MS,
  CONVERSATION_CACHE_FRESH_MS,
  useConversationMessageCache,
} from './useConversationMessageCache'
import { useLocalNodeStatus } from './useLocalNodeStatus'
import { useActiveTaskRefresh } from './useActiveTaskRefresh'
import { useOwnPresence } from './useOwnPresence'
import { useProjectLocalNodeBinding } from './useProjectLocalNodeBinding'
import styles from './ConversationPage.module.css'

interface ProjectRealtimeDetail {
  projectId?: string
  channelId?: string
  conversationId?: string
  taskId?: string
  kind?: string
}

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
  const { myPresence, handlePresenceSaved } = useOwnPresence(user, reloadProjectSpace)
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
  const { localNode, localNodeError } = useLocalNodeStatus()

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
  const conversationLoadSeqRef = useRef(0)
  const {
    clearConversationCaches,
    getConversationCache,
    loadCachedTaskMessages,
    touchConversationCache,
    writeConversationCache,
  } = useConversationMessageCache()
  const handleRuntimeRouteChange = useCallback((route: RuntimeRoute) => {
    setRuntimeRoute(route)
    setDirectPcCli(false)
  }, [])

  useEffect(() => { loadProjects() }, [user?.id]) // eslint-disable-line

  useEffect(() => {
    persistProjectRuntimeRouteSelection(window.localStorage, runtimeRoute)
  }, [runtimeRoute])

  useEffect(() => {
    persistDirectPcCliSelection(window.localStorage, directPcCli)
  }, [directPcCli])

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
    conversationLoadSeqRef.current += 1
    clearConversationCaches()
    setConvMessages([])
    setSessionTaskMessages([])
    setSessionView(null)
    setMemberConversationTarget(null)
    waitingForNewSession.current = false
  }, [activeProjectId, projectHomeVersion, clearConversationCaches])

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

      clearComposerDraft()
      const isExistingConversation = typeof sessionView === 'string' && sessionView !== 'new'
      const conversationId = isExistingConversation ? sessionView : uuidv4()
      const conversationTitle = isExistingConversation ? null : titleFromMessage(text)
      const directPcCliForRequest = directPcCliActive
      const requestRuntimeRoute: RuntimeRoute = directPcCliForRequest ? 'route_a' : runtimeRoute
      const useLocalNodeForRequest = (directPcCliForRequest || shouldPreferLocalNode) && localNodeReady
      const requestAgent = selectedAgentForRuntimeRoute(selectedAgent, modelOptions, requestRuntimeRoute)
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

  const localBindStatus = useProjectLocalNodeBinding({
    activeProjectId,
    activeProject,
    activeWorkspacePath,
    activeProjectRole,
    localNodeReady,
    localNodeId,
    shouldPreferLocalNode,
    loadProjects,
    reloadProjectSpace,
  })

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

  const memberDiscussionNeedsConversation = isAssistingMember && (!sessionView || sessionView === 'new')
  const composerBusy = sendingMessage || sendingMemberDiscussion
  const composerDisabled = composerBusy
    || memberDiscussionNeedsConversation
    || aiDevelopmentChannelBlocksAi
    || (!aiDevelopmentChannelId && !isAssistingMember)
    || (!activeChannelId && channels.length === 0 && !isAssistingMember)
  const composerPlaceholder = isAssistingMember
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
  }, [memberConversations]) // eslint-disable-line

  // 切换频道时重置会话视图
  useEffect(() => {
    conversationLoadSeqRef.current += 1
    setSessionView(null)
    setConvMessages([])
    setSessionTaskMessages([])
    waitingForNewSession.current = false
  }, [activeChannelId]) // eslint-disable-line

  // 打开一个会话：从服务端加载该会话的消息（与手机端同步）
  async function openConversation(convId: string, options: { force?: boolean } = {}) {
    if (!activeProjectId || !activeConversationTargetId) return
    const projectId = activeProjectId
    const targetUserId = activeConversationTargetId
    const channelId = aiDevelopmentChannelId
    const cached = getConversationCache(projectId, targetUserId, convId)
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
        touchConversationCache(projectId, targetUserId, convId, cached)
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

  const refreshTaskSurface = useCallback(async () => {
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
  }, [
    activeProjectId,
    activeChannelId,
    activeConversationTargetId,
    aiDevelopmentChannelId,
    loadCachedTaskMessages,
    sessionView,
    writeConversationCache,
  ])

  useActiveTaskRefresh({ activeProjectId, hasRunningTask, refreshTaskSurface })

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
      <ConversationChannelPanel
        activeProjectId={activeProjectId}
        activeProject={activeProject}
        projects={projects}
        projectsLoaded={projectsLoaded}
        filteredChannels={filteredChannels}
        activeChannelId={activeChannelId}
        channelSearch={channelSearch}
        sessionView={sessionView}
        memberConversations={memberConversations}
        hasConversationTarget={!!activeConversationTarget}
        activeConversationTargetName={activeConversationTargetName}
        isOwnConversationTarget={isOwnConversationTarget}
        onBackToProjects={() => useProjectStore.getState().selectProject('')}
        onOpenProjectHome={openProjectHome}
        onOpenProjectSettings={() => navigate(`/projects/${activeProjectId}`)}
        onCreateProject={() => setShowCreate(true)}
        onChannelSearchChange={setChannelSearch}
        onSelectChannel={selectChannel}
        onSelectProject={selectProject}
        onOpenSession={openSession}
        onStartNewSession={startNewSession}
        onResetConversationTarget={resetMemberConversationTarget}
      />

      {/* ══ 聊天区（中 1fr）══ */}
      <div className={styles.chatColumn}>
        <ConversationTopbar
          activeChannel={activeChannel}
          activeProject={activeProject}
          canRefresh={!!activeProjectId && !!activeChannelId}
          onRefresh={() => {
            if (!activeProjectId || !activeChannelId) return
            return useProjectStore.getState().loadMessages(activeProjectId, activeChannelId)
          }}
          onOpenNode={() => navigate('/node')}
          onOpenMobile={() => window.open('/app/download', '_blank', 'noopener')}
          onOpenLegacy={() => {
            const token = useAuthStore.getState().token
            if (token) {
              localStorage.setItem('lodex_token', token)
              localStorage.setItem('elon_token', token)
            }
            window.open('/pc-legacy', '_blank', 'noopener')
          }}
        />

        <ConversationStatusStack
          activeProjectId={activeProjectId}
          activeProject={activeProject}
          activeChannel={activeChannel}
          activeProjectRoleLabel={activeProjectRoleLabel}
          localNode={localNode}
          localNodeId={localNodeId}
          localNodeReady={localNodeReady}
          localNodeError={localNodeError}
          localBindStatus={localBindStatus}
          projectBoundToLocalNode={projectBoundToLocalNode}
          activeChannelBlocksAi={activeChannelBlocksAi}
          activeChannelIsNotAi={activeChannelIsNotAi}
          sessionView={sessionView}
        />

        <ConversationContent
          activeProjectId={activeProjectId}
          activeChannelId={activeChannelId}
          sessionView={sessionView}
          activeProject={activeProject}
          channels={channels}
          landing={landing}
          isAssistingMember={isAssistingMember}
          activeConversationTargetName={activeConversationTargetName}
          feedRef={feedRef}
          feedLoading={feedLoading}
          displayMessages={displayMessages}
          messageGroups={messageGroups}
          taskContext={taskContext}
          isDevChannel={isDevChannel}
          user={user}
          hasRunningTask={hasRunningTask}
          sendingMessage={sendingMessage}
          showNewMsg={showNewMsg}
          onCreateProject={() => setShowCreate(true)}
          onSelectLandingChannel={(id) => { setSessionView(null); selectChannel(id) }}
          onFeedScroll={handleFeedScroll}
          onScrollToBottom={scrollToBottom}
          onCancelTask={handleCancelTask}
          onApproveTool={handleApproveTool}
        />

        {activeProjectId && (
          <ConversationComposer
            projectId={activeProjectId}
            input={input}
            attachments={attachments}
            sendError={sendError}
            modelButtonCopy={modelButtonCopy}
            modelButtonRef={modelBtnRef}
            textareaRef={textareaRef}
            directPcCliActive={directPcCliActive}
            shouldPreferLocalNode={shouldPreferLocalNode}
            localNodeReady={localNodeReady}
            directPcCliAvailable={directPcCliAvailable}
            composerDisabled={composerDisabled}
            sending={sendingMessage || sendingMemberDiscussion}
            placeholder={composerPlaceholder}
            onSubmit={handleSend}
            onOpenModelPicker={() => setShowModelPicker((visible) => !visible)}
            onToggleDirectPcCli={setDirectPcCli}
            onInputChange={(value) => { setInput(value); autoResize() }}
            onKeyDown={handleKeyDown}
            onAttach={(attachment) => setAttachments((prev) => [...prev, attachment])}
            onRemoveAttachment={(attachmentId) => {
              setAttachments((prev) => prev.filter((item) => item.attachment_id !== attachmentId))
            }}
          />
        )}
      </div>

      <ConversationMemberPanel
        activeProjectId={activeProjectId}
        activeChannelId={activeChannelId}
        activeProject={activeProject}
        activeChannel={activeChannel}
        channels={channels}
        user={user}
        myPresence={myPresence}
        members={spaceMembers}
        spaceLoading={spaceLoading}
        spaceError={spaceError}
        memberPanelScope={memberPanelScope}
        memberMenu={memberMenu}
        selectedMember={selectedMember}
        memberPopoverY={memberPopoverY}
        isDevChannel={isDevChannel}
        activeWorkspacePath={activeWorkspacePath}
        activeConversationMemberId={isAssistingMember ? activeConversationTargetId : null}
        canModerateMembers={canModerateMembers}
        canManageMembers={canManageMembers}
        canInviteMembers={canInviteMembers}
        canViewMemberAudit={canViewMemberAudit}
        canUseRoleManager={canUseRoleManager}
        canManagePermissions={canManagePermissions}
        onSetMemberPanelScope={setMemberPanelScope}
        onOpenPresence={() => setShowPresence(true)}
        onOpenDirectory={() => setShowDirectory(true)}
        onOpenMembersPage={() => navigate(`/projects/${activeProjectId}/members`)}
        onOpenInvites={() => setShowInvites(true)}
        onOpenModerationCenter={() => { setModerationFocusMemberId(''); setShowModeration(true) }}
        onOpenRoleManager={() => { setRoleFocusMemberId(''); setShowRoles(true) }}
        onOpenAudit={() => setShowAudit(true)}
        onOpenPermissionDrawer={() => { setPermissionFocusMemberId(''); setShowPermissions(true) }}
        onCloseMemberMenu={() => setMemberMenu(null)}
        onCloseSelectedMember={() => setSelectedMember(null)}
        onOpenProfile={openMemberProfile}
        onOpenDetails={openMemberDetails}
        onOpenConversations={openMemberConversations}
        onOpenPermissions={openMemberPermissions}
        onOpenRoles={openMemberRoles}
        onModerate={moderateMemberFromPopover}
        onRemove={removeMemberFromProject}
        onSelectMember={(member, y) => { setSelectedMember(member); setMemberPopoverY(y) }}
        onOpenMemberMenu={setMemberMenu}
      />

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
