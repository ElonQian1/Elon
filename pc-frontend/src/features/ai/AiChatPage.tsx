import { useEffect, useMemo, useRef, useState, useCallback } from 'react'
import { createPortal } from 'react-dom'
import { useNavigate } from 'react-router-dom'
import { ChevronLeft, Folder } from 'lucide-react'
import { api } from '../../api/client'
import { useAuthStore } from '../../store/auth'
import { useModelStore } from '../models/useModelStore'
import { useProjectStore } from '../conversation/useProjectStore'
import {
  routeModelButtonCopy,
  selectedAgentForRuntimeRoute,
} from '../models/routeModelPolicy'
import { ModelPickerPopover } from '../models/ModelPicker'
import {
  initialRuntimeRouteFromStorage,
  persistRuntimeRouteSelection,
} from '../conversation/runtimeRoutes'
import type { RuntimeRoute } from '../conversation/runtimeRoutes'
import { APP_UPDATE_BEFORE_RELOAD_EVENT } from '../updates/appUpdateSession'
import {
  clearAiComposerDraft,
  readAiComposerDraft,
  saveAiComposerDraft,
  type AiComposerDraft,
} from '../updates/composerDrafts'
import AuthDialog from '../auth/AuthDialog'
import SidebarUserStrip from '../shell/SidebarUserStrip'
import { safeNodeAdminUrl } from '../../lib/utils'
import { DEFAULT_POPOVER_ANCHOR, popoverAnchorFromRect, type PopoverAnchor } from '../../lib/popoverPosition'
import AiWebChatSidebar from '../user-browser/AiWebChatSidebar'
import AiWebProviderPopover from '../user-browser/AiWebProviderPopover'
import AiWebComposerControls, { AiBrowserExperience } from '../user-browser/AiWebComposerControls'
import useAiWebChatBackend from '../user-browser/useAiWebChatBackend'
import useLocalAiOwnerIdentity from '../user-browser/useLocalAiOwnerIdentity'
import NodeStatusBanner from './NodeStatusBanner'
import AiChatTopbar from './AiChatTopbar'
import AiChatWelcome from './AiChatWelcome'
import AiWebClientUpgradeNotice from './AiWebClientUpgradeNotice'
import type { AiHomeMode } from './AiHomeModeSwitch'
import AiPinnedTools from './AiPinnedTools'
import AiUserProfilePopover, { type AiChatFriend } from './AiUserProfilePopover'
import AiChatMessageRow, {
  type AiHandoff,
  type AiMessage,
  type AiProjectCandidate,
} from './AiChatMessageRow'
import { isCodexVaultBackupIntent, runCodexVaultBackupFromAiChat } from './codexVaultQuickAction'
import {
  compactConversationText,
  conversationTitle,
  displayProjectName,
  formatHistoryAge,
  isGenericConversationTitle,
  makeConversationTitle,
  type AiConversation,
} from './aiConversationPresentation'
import styles from './AiChatPage.module.css'
import { v4 as uuidv4 } from 'uuid'
interface RemoteNodeInfo {
  node_id?: string
  agent_id?: string
  display_name?: string
  device_name?: string
  owner_user_id?: string
  online?: boolean
  ai_cli_ready?: boolean
  route_a_ready?: boolean
  allowed_clis?: string[]
}
function remoteNodeId(node: RemoteNodeInfo) {
  return String(node.node_id ?? node.agent_id ?? '').trim()
}
function remoteNodeName(node: RemoteNodeInfo) {
  const id = remoteNodeId(node)
  return String(node.display_name ?? node.device_name ?? id.slice(0, 8) ?? '远程节点').trim()
}

function remoteNodeHasCli(node: RemoteNodeInfo) {
  return !!node.online && (
    node.ai_cli_ready === true
    || node.route_a_ready === true
    || (node.allowed_clis?.length ?? 0) > 0
  )
}

function pickRemoteCliNode(
  nodes: RemoteNodeInfo[],
  userId?: string,
  preferredNodeId?: string | null,
  excludedNodeIds: string[] = [],
) {
  const excluded = new Set(excludedNodeIds.filter(Boolean))
  const ready = nodes.filter((node) => {
    const id = remoteNodeId(node)
    return id && !excluded.has(id) && remoteNodeHasCli(node)
  })
  if (preferredNodeId) {
    const preferred = ready.find((node) => remoteNodeId(node) === preferredNodeId)
    if (preferred) return preferred
  }
  return ready.find((node) => node.owner_user_id && node.owner_user_id !== userId) ?? ready[0] ?? null
}

function shouldRetryRemoteNodeExec(result: { output?: string; error?: string; exit_ok?: boolean }) {
  if ((result.output ?? '').trim()) return false
  const error = result.error ?? ''
  return result.exit_ok === false && (
    error.includes('指定的节点未在线')
    || error.includes('没有确认接收')
    || error.includes('没有返回任何 CLI 输出')
    || error.includes('连接假在线')
    || error.includes('通道在确认接收')
    || error.includes('节点连接已关闭')
    || error.includes('执行超时')
  )
}

export default function AiChatPage({ mode, onModeChange }: { mode: AiHomeMode; onModeChange: (mode: AiHomeMode) => void }) {
  const navigate = useNavigate()
  const user = useAuthStore((s) => s.user)
  const localAiOwner = useLocalAiOwnerIdentity()
  const web = useAiWebChatBackend(mode, localAiOwner.ownerKey)
  const selectedAgent = useModelStore((s) => s.selectedAgent)
  const modelLabel = useModelStore((s) => s.label)
  const modelOptions = useModelStore((s) => s.options)

  const [conversations, setConversations] = useState<AiConversation[]>([])
  const [conversationsLoaded, setConversationsLoaded] = useState(false)
  const [conversationPreviews, setConversationPreviews] = useState<Record<string, string>>({})
  const [historyProjectName, setHistoryProjectName] = useState('一龙 AI')
  const [expandedHistoryGroups, setExpandedHistoryGroups] = useState<Record<string, boolean>>({})
  const pendingAiDraftRef = useRef<AiComposerDraft | null>(readAiComposerDraft())
  const aiDraftReadyRef = useRef(false)
  // 初始即创建新会话 ID，保证输入框始终可见（与旧版一致）
  const [activeConvId, setActiveConvId] = useState<string>(() => pendingAiDraftRef.current?.activeConvId || uuidv4())
  const [messages, setMessages] = useState<AiMessage[]>([])
  const [messagesLoading, setMessagesLoading] = useState(false)
  const [input, setInput] = useState(() => pendingAiDraftRef.current?.input ?? '')
  const [sending, setSending] = useState(false)
  const [streamStatus, setStreamStatus] = useState('')
  const [streamingMessageId, setStreamingMessageId] = useState<string | null>(null)
  const [handoffSending, setHandoffSending] = useState(false)
  const [error, setError] = useState('')
  const [showModelPicker, setShowModelPicker] = useState(false)
  const [runtimeRoute, setRuntimeRoute] = useState<RuntimeRoute>(() => initialRuntimeRouteFromStorage(
    typeof window === 'undefined' ? null : window.localStorage,
    user?.id,
  ))
  const [friends, setFriends] = useState<AiChatFriend[]>([])
  const [totalUserCount, setTotalUserCount] = useState(0)
  const [userQuery, setUserQuery] = useState('')
  const [usersLoading, setUsersLoading] = useState(true)
  const [usersError, setUsersError] = useState('')
  const [selectedFriend, setSelectedFriend] = useState<AiChatFriend | null>(null)
  const [friendPopoverAnchor, setFriendPopoverAnchor] = useState<PopoverAnchor>(DEFAULT_POPOVER_ANCHOR)
  const [userPanelCollapsed, setUserPanelCollapsed] = useState(() => (
    typeof window !== 'undefined'
      ? window.localStorage.getItem('elon.pc.aiUserPanelCollapsed') === 'true'
      : false
  ))
  const [loginDialogOpen, setLoginDialogOpen] = useState(false)
  // 节点在线状态（由本页面轮询，同时传给 NodeStatusBanner 避免重复请求）
  const [onlineNodeId, setOnlineNodeId] = useState<string | null>(null)
  const [onlineNodeName, setOnlineNodeName] = useState<string>('')
  const [nodeStatusChecked, setNodeStatusChecked] = useState(false)
  const [remoteNodeIdValue, setRemoteNodeIdValue] = useState<string | null>(null)
  const [remoteNodeNameValue, setRemoteNodeNameValue] = useState('')
  const [remoteNodeStatusChecked, setRemoteNodeStatusChecked] = useState(false)

  const feedRef = useRef<HTMLDivElement>(null)
  const textareaRef = useRef<HTMLTextAreaElement>(null)
  const modelBtnRef = useRef<HTMLButtonElement>(null)
  const atBottomRef = useRef(true)
  const remoteNodeIdRef = useRef<string | null>(null)
  const workModelButtonCopy = useMemo(
    () => {
      const copy = routeModelButtonCopy(runtimeRoute, modelLabel, modelOptions, selectedAgent)
      if (runtimeRoute === 'route_c3' && remoteNodeNameValue) {
        return {
          ...copy,
          detail: remoteNodeNameValue,
          title: `AI来源：远程 Codex；节点：${remoteNodeNameValue}`,
        }
      }
      return copy
    },
    [runtimeRoute, modelLabel, modelOptions, selectedAgent, remoteNodeNameValue],
  )
  const chatMode = mode === 'chat'
  const visibleIdentityReady = chatMode ? Boolean(localAiOwner.ownerKey) : Boolean(user?.id)
  const visibleMessages = chatMode ? web.messages : messages
  const lastVisibleAssistantId = [...visibleMessages].reverse().find((message) => message.role === 'assistant')?.id
  const visibleInput = chatMode ? web.controller.draft : input
  const visibleSending = chatMode ? Boolean(web.controller.busyAction) : sending
  const visibleMessageLoading = chatMode ? web.capability.state === 'checking' : messagesLoading
  const modelButtonCopy = chatMode ? web.modelButtonCopy : workModelButtonCopy

  useEffect(() => {
    setError('')
    setShowModelPicker(false)
  }, [mode])

  useEffect(() => {
    setRuntimeRoute(initialRuntimeRouteFromStorage(window.localStorage, user?.id))
  }, [user?.id])

  useEffect(() => {
    persistRuntimeRouteSelection(window.localStorage, runtimeRoute, user?.id)
  }, [runtimeRoute, user?.id])

  useEffect(() => {
    try {
      window.localStorage.setItem('elon.pc.aiUserPanelCollapsed', userPanelCollapsed ? 'true' : 'false')
    } catch {
      // Layout preference is best-effort only.
    }
  }, [userPanelCollapsed])

  useEffect(() => {
    remoteNodeIdRef.current = remoteNodeIdValue
  }, [remoteNodeIdValue])

  useEffect(() => {
    let cancelled = false
    if (!user?.id) {
      setConversations([])
      setConversationPreviews({})
      setHistoryProjectName('一龙 AI')
      setConversationsLoaded(true)
      setFriends([])
      setTotalUserCount(0)
      setUsersLoading(false)
      setUsersError('')
      return () => { cancelled = true }
    }
    loadConversations()
    setUsersLoading(true)
    setUsersError('')
    api.get<{ recommendations?: AiChatFriend[]; total_count?: number }>('/api/me/friends/recommendations?limit=50')
      .then(d => {
        if (cancelled) return
        setFriends(d.recommendations ?? [])
        setTotalUserCount(d.total_count ?? d.recommendations?.length ?? 0)
        setUsersLoading(false)
      })
      .catch((err: { message?: string }) => {
        if (cancelled) return
        setUsersError(err.message ?? '用户加载失败')
        setUsersLoading(false)
      })
    return () => { cancelled = true }
  }, [user?.id])

  // ── 节点状态轮询（每 6s）──────────────────────────────────────────────
  useEffect(() => {
    if (!user?.id) {
      setOnlineNodeId(null)
      setOnlineNodeName('')
      setNodeStatusChecked(true)
      return
    }
    setNodeStatusChecked(false)
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
          setNodeStatusChecked(true)
        })
        .catch(() => {
          setOnlineNodeId(null)
          setOnlineNodeName('')
          setNodeStatusChecked(true)
        })
    }
    checkNode()
    const t = setInterval(checkNode, 6000)
    return () => clearInterval(t)
  }, [user?.id])

  useEffect(() => {
    if (nodeStatusChecked && runtimeRoute === 'route_a' && !onlineNodeId) {
      setRuntimeRoute('auto')
    }
  }, [nodeStatusChecked, onlineNodeId, runtimeRoute])

  // ── 远程 Codex 节点状态轮询 ─────────────────────────────────────────────
  useEffect(() => {
    if (!user?.id || runtimeRoute !== 'route_c3') {
      setRemoteNodeIdValue(null)
      setRemoteNodeNameValue('')
      setRemoteNodeStatusChecked(runtimeRoute !== 'route_c3')
      return
    }

    let cancelled = false
    setRemoteNodeStatusChecked(false)
    function checkRemoteNodes() {
      api.get<{ nodes?: RemoteNodeInfo[] }>('/api/nodes')
        .then((data) => {
          if (cancelled) return
          const node = pickRemoteCliNode(data.nodes ?? [], user?.id, remoteNodeIdRef.current)
          if (node) {
            setRemoteNodeIdValue(remoteNodeId(node))
            setRemoteNodeNameValue(remoteNodeName(node))
          } else {
            setRemoteNodeIdValue(null)
            setRemoteNodeNameValue('')
          }
          setRemoteNodeStatusChecked(true)
        })
        .catch(() => {
          if (cancelled) return
          setRemoteNodeIdValue(null)
          setRemoteNodeNameValue('')
          setRemoteNodeStatusChecked(true)
        })
    }
    checkRemoteNodes()
    const t = setInterval(checkRemoteNodes, 6000)
    return () => {
      cancelled = true
      clearInterval(t)
    }
  }, [runtimeRoute, user?.id])

  // 客户端搜索过滤
  const filteredFriends = useMemo(() => {
    const needle = userQuery.trim().toLowerCase()
    if (!needle) return friends
    return friends.filter(f =>
      [f.nickname, f.account, f.id].join(' ').toLowerCase().includes(needle)
    )
  }, [friends, userQuery])
  const onlineFriends = useMemo(() => filteredFriends.filter(f => f.is_online), [filteredFriends])
  const offlineFriends = useMemo(() => filteredFriends.filter(f => !f.is_online), [filteredFriends])
  const visibleUserCount = totalUserCount || friends.length
  const visibleConversations = useMemo(
    () => conversations.filter((conversation) => (conversation.message_count ?? 0) > 0 || conversation.id === activeConvId),
    [activeConvId, conversations],
  )
  const historyGroups = useMemo(() => {
    const groups = new Map<string, { id: string; name: string; conversations: AiConversation[] }>()
    visibleConversations.forEach((conversation) => {
      const name = displayProjectName(conversation.project_name || historyProjectName)
      const id = conversation.project_id || name
      const group = groups.get(id) ?? { id, name, conversations: [] }
      group.conversations.push(conversation)
      groups.set(id, group)
    })
    return Array.from(groups.values())
  }, [historyProjectName, visibleConversations])
  const activeConversation = useMemo(
    () => conversations.find((conversation) => conversation.id === activeConvId),
    [activeConvId, conversations],
  )
  const activeConversationTitle = activeConversation
    ? conversationTitle(activeConversation, conversationPreviews)
    : conversationPreviews[activeConvId] || '新对话'

  useEffect(() => {
    if (!user?.id || visibleConversations.length === 0) return
    let cancelled = false
    const needsPreview = visibleConversations
      .filter((conversation) =>
        (conversation.message_count ?? 0) > 0
        && isGenericConversationTitle(conversation.title)
        && !conversation.first_user_message
        && !conversationPreviews[conversation.id]
      )
      .slice(0, 24)
    if (needsPreview.length === 0) return

    Promise.all(needsPreview.map(async (conversation) => {
      try {
        const data = await api.get<{ messages?: AiMessage[] }>(
          `/api/me/ai/conversations/${encodeURIComponent(conversation.id)}/messages?limit=12`,
        )
        const firstUserMessage = (data.messages ?? []).find((message) => message.role === 'user')?.content
        const preview = compactConversationText(firstUserMessage, 32)
        return preview ? [conversation.id, preview] as const : null
      } catch {
        return null
      }
    })).then((entries) => {
      if (cancelled) return
      const nextEntries = entries.filter((entry): entry is readonly [string, string] => Boolean(entry))
      if (nextEntries.length === 0) return
      setConversationPreviews((current) => {
        const next = { ...current }
        nextEntries.forEach(([id, preview]) => {
          next[id] = preview
        })
        return next
      })
    })

    return () => { cancelled = true }
  }, [conversationPreviews, user?.id, visibleConversations])

  useEffect(() => {
    if (atBottomRef.current && feedRef.current) {
      feedRef.current.scrollTop = feedRef.current.scrollHeight
    }
  }, [messages, web.messages])

  async function loadConversations() {
    setConversationsLoaded(false)
    try {
      const data = await api.get<{ conversations?: AiConversation[]; project_id?: string; project_name?: string }>(
        '/api/me/ai/conversations?limit=50',
      )
      const projectName = displayProjectName(data.project_name)
      setHistoryProjectName(projectName)
      setConversations((data.conversations ?? []).map((conversation) => ({
        ...conversation,
        project_id: conversation.project_id ?? data.project_id,
        project_name: conversation.project_name ?? projectName,
      })))
    } catch { /* ignore */ }
    finally { setConversationsLoaded(true) }
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

  async function openForkedConversation(convId: string) { await loadConversations(); await selectConversation(convId) }
  function newConversation() {
    if (chatMode) {
      void web.controller.run('new_conversation')
      return
    }
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

  const persistAiComposerDraft = useCallback(() => {
    if (!activeConvId && !input) return
    saveAiComposerDraft({
      userId: user?.id,
      input,
      activeConvId,
    })
  }, [activeConvId, input, user?.id])

  function resetComposer() {
    setInput('')
    clearAiComposerDraft()
    setError('')
    if (textareaRef.current) textareaRef.current.style.height = '46px'
  }

  function enqueueUserMessage(text: string) {
    const convId = activeConvId
    const isExistingConversation = conversations.some(
      (conversation) => conversation.id === convId && (conversation.message_count ?? 0) > 0,
    )
    const newConversationTitle = isExistingConversation ? undefined : makeConversationTitle(text)
    if (newConversationTitle) {
      setConversationPreviews((current) => ({ ...current, [convId]: newConversationTitle }))
    }
    setMessages((prev) => [...prev, { role: 'user', content: text, created_at: new Date().toISOString() }])
    atBottomRef.current = true
    return { convId, newConversationTitle }
  }

  function restoreComposerAfterError(previousInput: string, err: unknown) {
    setInput(previousInput)
    saveAiComposerDraft({ userId: user?.id, input: previousInput, activeConvId })
    window.setTimeout(autoResize, 0)
    setError((err as { message?: string }).message ?? '发送失败')
  }

  async function runCodexVaultBackupMessage() {
    const reply = await runCodexVaultBackupFromAiChat(safeNodeAdminUrl())
    setMessages((prev) => [...prev, {
      role: 'assistant',
      content: reply,
      created_at: new Date().toISOString(),
    }])
  }

  async function handleProjectHandoff(handoff: AiHandoff, candidate?: AiProjectCandidate) {
    if (handoffSending) return
    if (!candidate?.id) {
      navigate('/projects')
      return
    }
    setHandoffSending(true)
    setError('')
    try {
      const projectStore = useProjectStore.getState()
      await projectStore.selectProject(candidate.id)
      const aiChannel = useProjectStore.getState().channels.find((channel) => channel.kind === 'ai_development')
      if (!aiChannel) throw new Error('该项目暂时没有可用的项目 AI 频道。')
      await useProjectStore.getState().selectChannel(aiChannel.id)
      const response = await useProjectStore.getState().sendMessage(
        handoff.request,
        selectedAgentForRuntimeRoute(selectedAgent, modelOptions, 'auto'),
      )
      if (!response) throw new Error('项目 AI 没有接收这条任务，请稍后重试。')
      navigate(`/?project=${encodeURIComponent(candidate.id)}`)
    } catch (err) {
      setError((err as { message?: string }).message ?? '交接到项目 AI 失败，请打开项目后重试。')
    } finally {
      setHandoffSending(false)
    }
  }

  async function handleCodexVaultShortcut() {
    if (sending) return
    if (!user?.id) {
      setError('请先登录一龙账号后再保存 Codex 账号。')
      setLoginDialogOpen(true)
      return
    }
    const text = '帮我把这台电脑的 Codex 账号保存到云端账号保险箱'
    const previousInput = input
    resetComposer()
    enqueueUserMessage(text)
    setSending(true)
    try {
      await runCodexVaultBackupMessage()
    } catch (err) {
      restoreComposerAfterError(previousInput, err)
    } finally {
      setSending(false)
    }
  }

  useEffect(() => {
    if (aiDraftReadyRef.current) return
    const draft = pendingAiDraftRef.current
    if (!draft) {
      aiDraftReadyRef.current = true
      return
    }
    if (draft.userId && user?.id && draft.userId !== user.id) {
      pendingAiDraftRef.current = null
      aiDraftReadyRef.current = true
      clearAiComposerDraft()
      return
    }
    if (draft.activeConvId && !conversationsLoaded) return
    if (draft.activeConvId && conversations.some((conv) => conv.id === draft.activeConvId)) {
      void selectConversation(draft.activeConvId)
    }
    setInput(draft.input ?? '')
    pendingAiDraftRef.current = null
    aiDraftReadyRef.current = true
    window.setTimeout(autoResize, 0)
  }, [autoResize, conversations, conversationsLoaded, user?.id])

  useEffect(() => {
    if (!aiDraftReadyRef.current) return
    persistAiComposerDraft()
  }, [persistAiComposerDraft])

  useEffect(() => {
    function saveBeforeReload() {
      persistAiComposerDraft()
    }
    window.addEventListener(APP_UPDATE_BEFORE_RELOAD_EVENT, saveBeforeReload)
    return () => window.removeEventListener(APP_UPDATE_BEFORE_RELOAD_EVENT, saveBeforeReload)
  }, [persistAiComposerDraft])

  async function selectFreshRemoteCodexNode(excludedNodeIds: string[] = []) {
    const data = await api.get<{ nodes?: RemoteNodeInfo[] }>('/api/nodes')
    const node = pickRemoteCliNode(data.nodes ?? [], user?.id, remoteNodeIdRef.current, excludedNodeIds)
    if (!node) return null
    const id = remoteNodeId(node)
    const name = remoteNodeName(node)
    setRemoteNodeIdValue(id)
    setRemoteNodeNameValue(name)
    setRemoteNodeStatusChecked(true)
    return { id, name }
  }

  async function handleSend(e: React.FormEvent | React.KeyboardEvent) {
    e.preventDefault()
    if (chatMode) {
      const text = web.controller.draft.trim()
      if (!localAiOwner.ownerKey) {
        setError(localAiOwner.detail)
        if (localAiOwner.source === 'none') setLoginDialogOpen(true)
        return
      }
      if (web.controller.snapshot?.streaming) {
        await web.controller.run('stop_generation')
        return
      }
      if (!web.canCompose) {
        setError(web.userState.detail)
        return
      }
      if (!text) return
      setError('')
      await web.controller.run('send_prompt', text, web.controller.snapshot?.draft ?? '')
      return
    }
    const text = input.trim()
    if (!text || sending) return
    if (!user?.id) {
      setError('请先登录账号后开始对话。')
      return
    }
    const previousInput = input
    const isVaultBackup = isCodexVaultBackupIntent(text)
    if (!isVaultBackup && runtimeRoute === 'route_c2') {
      setError('远程 AI 模型聊天还没有接入普通对话页，请先切到远程 Codex、自动选择或平台 AI。')
      return
    }
    if (!isVaultBackup && runtimeRoute === 'route_c3' && !remoteNodeStatusChecked) {
      setError('正在同步远程 Codex 节点，请稍后再发送。')
      return
    }
    if (!isVaultBackup && runtimeRoute === 'route_c3' && !remoteNodeIdValue) {
      setError('没有找到在线可用的远程 Codex 节点。请确认夜云 PC 节点在线且 Codex/Claude 已就绪。')
      return
    }
    let requestRuntimeRoute: RuntimeRoute = runtimeRoute
    let forcePlatformFallback = false
    if (!isVaultBackup && runtimeRoute === 'route_a' && !onlineNodeId) {
      requestRuntimeRoute = 'route_c'
      forcePlatformFallback = true
      setRuntimeRoute('auto')
    }
    resetComposer()

    // 乐观更新：先显示用户消息
    const { convId, newConversationTitle } = enqueueUserMessage(text)

    setSending(true)
    setStreamStatus('')
    let pendingStreamMessageId: string | null = null
    try {
      if (isVaultBackup) {
        await runCodexVaultBackupMessage()
        return
      }
      const requestAgent = forcePlatformFallback
        ? ''
        : selectedAgentForRuntimeRoute(selectedAgent, modelOptions, requestRuntimeRoute)
      const useLocalNode = !!onlineNodeId && (requestRuntimeRoute === 'auto' || requestRuntimeRoute === 'route_a')
      const useRemoteCodexNode = requestRuntimeRoute === 'route_c3' && !!remoteNodeIdValue
      if (useLocalNode || useRemoteCodexNode) {
        // ── 节点在线：直接在 PC 节点上执行 ────────────────────────────────
        let targetNodeId = useRemoteCodexNode ? remoteNodeIdValue : onlineNodeId
        let targetNodeName = useRemoteCodexNode ? remoteNodeNameValue : onlineNodeName
        let res: { output: string; req_id: string; node_id: string; node_display_name: string; exit_ok: boolean; error?: string }
        try {
          res = await api.post<{ output: string; req_id: string; node_id: string; node_display_name: string; exit_ok: boolean; error?: string }>(
            '/api/me/node/exec',
            { prompt: text, node_id: targetNodeId },
          )
        } catch (err) {
          const message = (err as { message?: string }).message ?? ''
          if (!useRemoteCodexNode || !message.includes('指定的节点未在线')) {
            throw err
          }
          const nextNode = await selectFreshRemoteCodexNode(targetNodeId ? [targetNodeId] : [])
          if (!nextNode) {
            throw err
          }
          targetNodeId = nextNode.id
          targetNodeName = nextNode.name
          res = await api.post<{ output: string; req_id: string; node_id: string; node_display_name: string; exit_ok: boolean; error?: string }>(
            '/api/me/node/exec',
            { prompt: text, node_id: targetNodeId },
          )
        }
        if (useRemoteCodexNode && shouldRetryRemoteNodeExec(res)) {
          const nextNode = await selectFreshRemoteCodexNode(targetNodeId ? [targetNodeId] : [])
          if (nextNode) {
            targetNodeId = nextNode.id
            targetNodeName = nextNode.name
            res = await api.post<{ output: string; req_id: string; node_id: string; node_display_name: string; exit_ok: boolean; error?: string }>(
              '/api/me/node/exec',
              { prompt: text, node_id: targetNodeId },
            )
          }
        }
        const nodeMsg: AiMessage = {
          role: 'assistant',
          content: res.output
            || (res.error
              ? `执行失败：${res.error}`
              : res.exit_ok === false
                ? '远程 Codex 执行失败，但节点没有返回错误详情。请稍后重试，或换一个远程节点。'
                : '（无输出）'),
          created_at: new Date().toISOString(),
          node_exec: true,
          node_display_name: res.node_display_name || targetNodeName,
          node_remote: useRemoteCodexNode,
          exit_ok: res.exit_ok,
        }
        setMessages((prev) => [...prev, nodeMsg])
      } else {
        // ── 无节点：走首页 AI 流式对话 ────────────────────────────────────
        pendingStreamMessageId = uuidv4()
        setStreamingMessageId(pendingStreamMessageId)
        setMessages((prev) => [...prev, {
          id: pendingStreamMessageId!,
          role: 'assistant',
          content: '',
          created_at: new Date().toISOString(),
        }])
        await api.streamPost('/api/llm/chat/stream', {
          messages: [{ role: 'user', content: text }],
          agent: requestAgent || null,
          runtimeRoute: requestRuntimeRoute,
          conversation_id: convId,
          conversation_title: newConversationTitle,
          scope: 'chat_memory',
        }, (event) => {
          if (event.type === 'status') {
            setStreamStatus(typeof event.message === 'string' ? event.message : '正在生成回答…')
            return
          }
          if (event.type === 'delta') {
            const delta = typeof event.content === 'string' ? event.content : ''
            if (!delta || !pendingStreamMessageId) return
            setStreamStatus('正在生成回答…')
            setMessages((prev) => prev.map((message) => message.id === pendingStreamMessageId
              ? { ...message, content: message.content + delta }
              : message))
            return
          }
          if (event.type === 'sources' && pendingStreamMessageId) {
            setMessages((prev) => prev.map((message) => message.id === pendingStreamMessageId
              ? { ...message, sources: Array.isArray(event.sources) ? event.sources as AiMessage['sources'] : [] }
              : message))
            return
          }
          if (event.type === 'handoff' && pendingStreamMessageId) {
            setMessages((prev) => prev.map((message) => message.id === pendingStreamMessageId
              ? { ...message, handoff: event.handoff as AiMessage['handoff'] }
              : message))
            return
          }
          if (event.type === 'error') {
            throw new Error(typeof event.message === 'string' ? event.message : 'AI 请求失败')
          }
          if (event.type === 'done' && pendingStreamMessageId) {
            setMessages((prev) => prev.map((message) => message.id === pendingStreamMessageId
              ? {
                ...message,
                content: message.content || (typeof event.reply === 'string' ? event.reply : ''),
                assistant_mode: event.assistant_mode as AiMessage['assistant_mode'],
                tool_used: typeof event.tool_used === 'string' ? event.tool_used || null : null,
                sources: Array.isArray(event.sources) ? event.sources as AiMessage['sources'] : message.sources,
                handoff: event.handoff as AiMessage['handoff'],
              }
              : message))
          }
        })
        loadConversations()
      }
    } catch (err) {
      if (pendingStreamMessageId) {
        setMessages((prev) => prev.filter((message) => message.id !== pendingStreamMessageId))
      }
      restoreComposerAfterError(previousInput, err)
    } finally {
      setSending(false)
      setStreamingMessageId(null)
      setStreamStatus('')
    }
  }

  function handleKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSend(e)
    }
  }

  return (
    <div
      className={styles.layout}
      data-ai-surface="production-home"
      data-user-panel-collapsed={userPanelCollapsed ? 'true' : undefined}
    >
      {/* 会话列表（左栏）*/}
      <aside className={styles.sidebar}>
        <div className={styles.sideHeader}>
          <span>一龙 AI</span>
          <button className={styles.newBtn} onClick={() => chatMode ? void web.controller.run('new_conversation') : newConversation()} title="新对话" type="button" disabled={visibleSending || (chatMode && !web.userState.canNewConversation)}>+</button>
        </div>
        {chatMode ? <AiWebChatSidebar web={web} /> : <><AiPinnedTools
          sending={sending}
          onNewConversation={newConversation}
          onOpenDoctor={() => navigate('/doctor')}
          onCodexVaultBackup={handleCodexVaultShortcut}
        /><div className={styles.convList} data-testid="ai-conversation-list">
          {historyGroups.length === 0 && (
            <p className={styles.hint}>{conversationsLoaded ? '还没有对话记录' : '正在加载对话记录...'}</p>
          )}
          {historyGroups.map((group) => {
            const expanded = expandedHistoryGroups[group.id] ?? false
            const visibleItems = expanded ? group.conversations : group.conversations.slice(0, 5)
            return (
              <section className={styles.historyGroup} key={group.id}>
                <div className={styles.historyGroupHeader}>
                  <Folder aria-hidden="true" size={14} strokeWidth={2} />
                  <span>{group.name}</span>
                </div>
                {visibleItems.map((conversation) => (
                  <button
                    key={conversation.id} data-testid="ai-conversation-row" data-conversation-id={conversation.id}
                    className={[styles.historyItem, conversation.id === activeConvId ? styles.historyItemActive : ''].join(' ')}
                    onClick={() => selectConversation(conversation.id)}
                    type="button"
                  >
                    <span className={styles.historyItemMain}>
                      <strong className={styles.historyItemTitle}>
                        {conversationTitle(conversation, conversationPreviews)}
                      </strong>
                      {conversation.updated_at && (
                        <time className={styles.historyItemTime} dateTime={conversation.updated_at}>
                          {formatHistoryAge(conversation.updated_at)}
                        </time>
                      )}
                    </span>
                    {(conversation.message_count ?? 0) > 1 && (
                      <span className={styles.historyItemMeta}>{conversation.message_count} 条</span>
                    )}
                  </button>
                ))}
                {group.conversations.length > 5 && (
                  <button
                    className={styles.historyExpand} data-testid="ai-conversation-list-more"
                    onClick={() => setExpandedHistoryGroups((current) => ({
                      ...current,
                      [group.id]: !expanded,
                    }))}
                    type="button"
                  >
                    {expanded ? '收起' : '展开显示'}
                  </button>
                )}
              </section>
            )
          })}
        </div></>}
        <SidebarUserStrip />
      </aside>

      <div className={styles.chat} data-ai-chat-main>
        <AiChatTopbar
          title={chatMode ? web.title : activeConversationTitle}
          userPanelCollapsed={userPanelCollapsed}
          modelButtonCopy={modelButtonCopy}
          sending={visibleSending}
          mode={mode}
          onModeChange={onModeChange}
          onToggleUserPanel={() => setUserPanelCollapsed((collapsed) => !collapsed)}
          onCodexVaultBackup={handleCodexVaultShortcut} onOpenOfficial={chatMode && web.ready ? () => { void web.controller.openOfficial() } : undefined}
        />

        <div
          className={styles.feed}
          ref={feedRef}
          onScroll={() => {
            const el = feedRef.current
            if (el) atBottomRef.current = el.scrollHeight - el.scrollTop - el.clientHeight < 80
          }}
        >
          {chatMode && <AiWebClientUpgradeNotice web={web} />}
          {!chatMode && onlineNodeId && (
            <NodeStatusBanner onlineNodeId={onlineNodeId} onlineNodeName={onlineNodeName} />
          )}
          {visibleMessages.length === 0 && !visibleMessageLoading && (
            <AiChatWelcome
              chatMode={chatMode}
              identityReady={visibleIdentityReady}
              onlineNodeId={onlineNodeId || ''}
              onlineNodeName={onlineNodeName}
              sending={visibleSending}
              web={web}
              onLogin={() => setLoginDialogOpen(true)}
            />
          )}
          {visibleMessageLoading && <p className={styles.hint}>{chatMode ? '正在连接本地网页 AI…' : '读取消息…'}</p>}
          {visibleMessages.filter((m) => m.role !== 'system').map((m, i) => (
            <AiChatMessageRow
              key={m.id ?? `${m.role}:${m.created_at ?? i}`}
              activeConvId={chatMode ? '' : activeConvId}
              index={i}
              message={m}
              user={user}
              streaming={m.id === (chatMode ? web.streamingMessageId : streamingMessageId)}
              streamingStatus={chatMode ? web.streamingStatus : streamStatus || '正在处理…'}
              onConversationForked={chatMode ? undefined : openForkedConversation}
              onProjectHandoff={chatMode ? undefined : handleProjectHandoff}
              onRegenerate={chatMode && m.id === lastVisibleAssistantId && web.provider?.adapterActions.includes('regenerate_response')
                ? async () => { await web.controller.run('regenerate_response') }
                : undefined}
              onOpenOfficial={chatMode && m.renderer_compatibility ? () => { void web.controller.openOfficial() } : undefined} onCheckUpdates={chatMode && m.renderer_compatibility ? () => navigate('/pc/node') : undefined}
            />
          ))}
        </div>

        {chatMode && <AiWebComposerControls web={web} />}
        <form className={styles.composer} onSubmit={handleSend}>
            <button
              ref={modelBtnRef}
              className={styles.modelBtn}
              type="button"
              title={modelButtonCopy.title}
              onClick={() => setShowModelPicker((v) => !v)}
            >
              <span>{modelButtonCopy.source}</span>
              <strong>{modelButtonCopy.detail}</strong>
            </button>
            <textarea
              ref={textareaRef}
              className={styles.composerInput}
              value={visibleInput}
              onChange={(e) => {
                if (chatMode) web.controller.setDraft(e.target.value)
                else setInput(e.target.value)
                autoResize()
              }}
              onKeyDown={handleKeyDown}
              placeholder="输入消息，Enter 发送，Shift+Enter 换行"
              disabled={chatMode ? !web.canEdit : visibleSending}
              rows={1}
            />
            <button
              className={styles.sendBtn}
              type="submit"
              disabled={chatMode
                ? !web.controller.snapshot?.streaming && (!visibleInput.trim() || !web.canCompose
                  || Boolean(web.controller.busyAction && web.controller.busyAction !== 'new_conversation'))
                : visibleSending || !visibleInput.trim()}
            >
              {visibleSending ? '…' : chatMode && web.controller.snapshot?.streaming ? '停止' : '发送'}
            </button>
          </form>
        {error && <p className={styles.sendError}>{error}</p>}<AiBrowserExperience />
      </div>

      {/* ══ 右侧用户栏 ══ */}
      {!userPanelCollapsed && <aside className={styles.userPanel}>
        <div className={styles.userPanelTitle}>
          <div className={styles.userPanelTitleCopy}>
            <strong>全站用户{friends.length > 0 ? ` — ${friends.length}` : ''}</strong>
            <span>AI 大厅</span>
          </div>
          {visibleUserCount > friends.length && (
            <small className={styles.userPanelMore}>共{visibleUserCount}位</small>
          )}
        </div>
        <div className={styles.userPanelList}>
          {selectedFriend && createPortal(
            <AiUserProfilePopover
              friend={selectedFriend}
              anchor={friendPopoverAnchor}
              onClose={() => setSelectedFriend(null)}
            />,
            document.body
          )}
          {/* 搜索框 */}
          {(friends.length > 0 || userQuery) && (
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
          {usersLoading && (
            <div className={styles.userPanelSkeleton}>
              <span />
              <span />
              <span />
            </div>
          )}
          {!user && (
            <p className={styles.userPanelHint}>登录后显示全站用户、好友状态和协作入口。</p>
          )}
          {user && !usersLoading && usersError && (
            <p className={styles.userPanelHint}>{usersError}</p>
          )}
          {user && !usersLoading && !usersError && friends.length === 0 && (
            <p className={styles.userPanelHint}>暂无推荐用户</p>
          )}
          {!usersLoading && !usersError && filteredFriends.length === 0 && userQuery && (
            <p className={styles.userPanelHint}>没有匹配的用户</p>
          )}
          {/* 在线 */}
          {!usersLoading && onlineFriends.length > 0 && (
            <>
              <div className={styles.userPanelSection}>
                在线 · {onlineFriends.length}
              </div>
              {onlineFriends.map(f => (
                <button key={f.id} className={styles.userPanelItem} type="button" onClick={(e) => {
                  const r = e.currentTarget.getBoundingClientRect()
                  setSelectedFriend(f)
                  setFriendPopoverAnchor(popoverAnchorFromRect(r))
                }}>
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
                </button>
              ))}
            </>
          )}
          {/* 离线 */}
          {!usersLoading && offlineFriends.length > 0 && (
            <>
              <div className={styles.userPanelSection}>
                离线 · {offlineFriends.length}
              </div>
              {offlineFriends.map(f => (
                <button key={f.id} className={styles.userPanelItem} type="button" onClick={(e) => {
                  const r = e.currentTarget.getBoundingClientRect()
                  setSelectedFriend(f)
                  setFriendPopoverAnchor(popoverAnchorFromRect(r))
                }}>
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
                </button>
              ))}
            </>
          )}
          {/* 提示条 */}
          {!usersLoading && visibleUserCount > friends.length && !userQuery && (
            <p className={styles.userPanelHint}>已显示 {friends.length} 位</p>
          )}
        </div>
      </aside>}

      {userPanelCollapsed && (
        <button
          className={styles.userPanelRestoreBtn}
          type="button"
          title="展开右侧用户栏"
          aria-label="展开右侧用户栏"
          onClick={() => setUserPanelCollapsed(false)}
        >
          <ChevronLeft size={17} aria-hidden="true" />
        </button>
      )}

      <AuthDialog
        open={loginDialogOpen && !user?.id}
        initialMode="login"
        onClose={() => setLoginDialogOpen(false)}
      />

      {showModelPicker && (chatMode ? (
        <AiWebProviderPopover
          anchorRef={modelBtnRef}
          web={web}
          onClose={() => setShowModelPicker(false)}
        />
      ) : (
        <ModelPickerPopover
          anchorRef={modelBtnRef}
          runtimeRoute={runtimeRoute}
          onRuntimeRouteChange={setRuntimeRoute}
          onClose={() => setShowModelPicker(false)}
        />
      ))}
    </div>
  )
}
