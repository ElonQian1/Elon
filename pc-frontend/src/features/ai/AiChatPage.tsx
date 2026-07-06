import { useEffect, useMemo, useRef, useState, useCallback } from 'react'
import { createPortal } from 'react-dom'
import { useNavigate } from 'react-router-dom'
import { ChevronLeft, Folder } from 'lucide-react'
import { api } from '../../api/client'
import { useAuthStore } from '../../store/auth'
import { useModelStore } from '../models/useModelStore'
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
import MarkdownContent from '../markdown/MarkdownContent'
import SidebarUserStrip from '../shell/SidebarUserStrip'
import UserAvatar from '../shell/UserAvatar'
import { formatTime, safeNodeAdminUrl } from '../../lib/utils'
import { compactDisplayMessageContent, displayMessageContentOrAttachment } from '../../lib/messageDisplay'
import { DEFAULT_POPOVER_ANCHOR, fixedPopoverPosition, popoverAnchorFromRect, type PopoverAnchor } from '../../lib/popoverPosition'
import NodeStatusBanner from './NodeStatusBanner'
import AiChatTopbar from './AiChatTopbar'
import AiPinnedTools from './AiPinnedTools'
import { isCodexVaultBackupIntent, runCodexVaultBackupFromAiChat } from './codexVaultQuickAction'
import styles from './AiChatPage.module.css'
import { v4 as uuidv4 } from 'uuid'

interface AiConversation {
  id: string
  title?: string
  updated_at?: string
  message_count?: number
  project_id?: string
  project_name?: string
  first_user_message?: string
}

interface AiMessage {
  id?: string
  role: 'user' | 'assistant' | 'system'
  content: string
  created_at?: string
  // 节点本机执行输出扩展字段
  node_exec?: boolean
  node_display_name?: string
  node_remote?: boolean
  exit_ok?: boolean
  model?: string
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
  already_friend?: boolean
}

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

const GENERIC_AI_TITLES = new Set(['普通聊天会话', '新对话', 'AI 对话', '一龙 AI 对话'])

function compactConversationText(text?: string, maxLength = 28) {
  return compactDisplayMessageContent(text, maxLength)
}

function isGenericConversationTitle(title?: string) {
  const normalized = (title ?? '').trim()
  return !normalized || GENERIC_AI_TITLES.has(normalized)
}

function displayProjectName(name?: string) {
  const normalized = (name ?? '').trim()
  if (!normalized || GENERIC_AI_TITLES.has(normalized)) return '一龙 AI'
  return normalized
}

function makeConversationTitle(text: string) {
  return compactConversationText(text, 32) || '新对话'
}

function conversationTitle(conversation: AiConversation | undefined, previews: Record<string, string>) {
  if (!conversation) return '新对话'
  const title = conversation.title?.trim()
  if (title && !GENERIC_AI_TITLES.has(title)) return title
  return compactConversationText(conversation.first_user_message, 32)
    || compactConversationText(previews[conversation.id], 32)
    || title
    || '新对话'
}

function formatHistoryAge(input?: string) {
  if (!input) return ''
  const time = new Date(input).getTime()
  if (!Number.isFinite(time)) return ''
  const diffMs = Math.max(0, Date.now() - time)
  const minute = 60 * 1000
  const hour = 60 * minute
  const day = 24 * hour
  const week = 7 * day
  const month = 30 * day
  if (diffMs < minute) return '刚刚'
  if (diffMs < hour) return `${Math.floor(diffMs / minute)}分`
  if (diffMs < day) return `${Math.floor(diffMs / hour)}小时`
  if (diffMs < week) return `${Math.floor(diffMs / day)}天`
  if (diffMs < month) return `${Math.floor(diffMs / week)}周`
  return formatTime(input)
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

export default function AiChatPage() {
  const navigate = useNavigate()
  const user = useAuthStore((s) => s.user)
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
  const [error, setError] = useState('')
  const [showModelPicker, setShowModelPicker] = useState(false)
  const [runtimeRoute, setRuntimeRoute] = useState<RuntimeRoute>(() => initialRuntimeRouteFromStorage(
    typeof window === 'undefined' ? null : window.localStorage,
  ))
  const [friends, setFriends] = useState<Friend[]>([])
  const [totalUserCount, setTotalUserCount] = useState(0)
  const [userQuery, setUserQuery] = useState('')
  const [usersLoading, setUsersLoading] = useState(true)
  const [usersError, setUsersError] = useState('')
  const [selectedFriend, setSelectedFriend] = useState<Friend | null>(null)
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
  const modelButtonCopy = useMemo(
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

  useEffect(() => {
    persistRuntimeRouteSelection(window.localStorage, runtimeRoute)
  }, [runtimeRoute])

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
    api.get<{ recommendations?: Friend[]; total_count?: number }>('/api/me/friends/recommendations?limit=50')
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
  }, [messages])

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

  async function handleCodexVaultShortcut() {
    if (sending) return
    if (!user?.id) {
      setError('请先登录账号后再备份 Codex auth.json。')
      setLoginDialogOpen(true)
      return
    }
    const text = '帮我把本机 Codex auth.json 备份到云端保险箱'
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
        // ── 无节点：走云端 AI 对话 ───────────────────────────────────────
        const res = await api.post<LmChatResponse>('/api/llm/chat', {
          messages: [{ role: 'user', content: text }],
          agent: requestAgent || null,
          runtimeRoute: requestRuntimeRoute,
          conversation_id: convId,
          conversation_title: newConversationTitle,
          scope: 'chat_memory',
        })
        const reply = res.reply ?? res.content ?? ''
        const aiMsg: AiMessage = { role: 'assistant', content: reply, created_at: new Date().toISOString() }
        setMessages((prev) => [...prev, aiMsg])
        loadConversations()
      }
    } catch (err) {
      restoreComposerAfterError(previousInput, err)
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
    <div className={styles.layout} data-user-panel-collapsed={userPanelCollapsed ? 'true' : undefined}>
      {/* 会话列表（左栏）*/}
      <aside className={styles.sidebar}>
        <div className={styles.sideHeader}>
          <span>一龙 AI</span>
          <button className={styles.newBtn} onClick={newConversation} title="新对话" type="button">+</button>
        </div>
        <AiPinnedTools
          sending={sending}
          onNewConversation={newConversation}
          onOpenDoctor={() => navigate('/doctor')}
          onCodexVaultBackup={handleCodexVaultShortcut}
        />
        <div className={styles.convList}>
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
                    key={conversation.id}
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
                    className={styles.historyExpand}
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
        </div>
        <SidebarUserStrip />
      </aside>

      {/* 聊天区 */}
      <div className={styles.chat}>
        <AiChatTopbar
          title={activeConversationTitle}
          userPanelCollapsed={userPanelCollapsed}
          modelButtonCopy={modelButtonCopy}
          sending={sending}
          onToggleUserPanel={() => setUserPanelCollapsed((collapsed) => !collapsed)}
          onCodexVaultBackup={handleCodexVaultShortcut}
        />

        <div
          className={styles.feed}
          ref={feedRef}
          onScroll={() => {
            const el = feedRef.current
            if (el) atBottomRef.current = el.scrollHeight - el.scrollTop - el.clientHeight < 80
          }}
        >
          {onlineNodeId && (
            <NodeStatusBanner onlineNodeId={onlineNodeId} onlineNodeName={onlineNodeName} />
          )}
          {messages.length === 0 && !messagesLoading && (
            <div className={styles.welcome}>
              <h2>你好，我是一龙 AI</h2>
              <p>{!user?.id
                ? '登录账号后即可开始和我对话。'
                : onlineNodeId
                  ? `本机「${onlineNodeName}」已就绪，直接输入需求或命令。`
                  : '随时可以开始对话，我会记住我们聊过的内容。'}</p>
              {!user?.id && (
                <div className={styles.loginPrompt}>
                  <button
                    className={styles.startBtn}
                    type="button"
                    onClick={() => setLoginDialogOpen(true)}
                  >
                    登录账号
                  </button>
                  <span>登录后可以开始对话，并同步你的项目、好友和电脑节点。</span>
                </div>
              )}
            </div>
          )}
          {messagesLoading && <p className={styles.hint}>读取消息…</p>}
          {messages.filter((m) => m.role !== 'system').map((m, i) => {
            const isUser = m.role === 'user'
            const isNode = !isUser && m.node_exec === true
            const content = displayMessageContentOrAttachment(m.content)
            const hasMarkdown = !isUser && /[#*`\[\]>|]/.test(content)
            const nodePrefix = m.node_remote ? '远程' : '本机'
            const nameLabel = isUser ? (user?.nickname ?? user?.account ?? '我') : (isNode ? `${nodePrefix} · ${m.node_display_name ?? ''}` : 'AI')
            return (
              <div key={i} className={[styles.msgRow, isUser ? styles.ownRow : ''].join(' ')}>
                {isUser
                  ? <UserAvatar user={user} size="compact" className={styles.avatar} />
                  : <div className={[styles.avatar, isNode ? styles.nodeAvatar : ''].join(' ')}>{isNode ? '🖥' : 'AI'}</div>}
                <div className={styles.msgBody}>
                  <div className={styles.msgMeta}>
                    <strong className={isNode ? styles.nodeLabel : ''}>{nameLabel}</strong>
                    {m.created_at && <span>{formatTime(m.created_at)}</span>}
                    {isNode && m.model && <span className={styles.modelTag}>{m.model}</span>}
                    {isNode && m.exit_ok === false && <span className={styles.exitFail}>执行失败</span>}
                  </div>
                  {hasMarkdown
                    ? <div className={styles.msgContent}><MarkdownContent content={content} copy /></div>
                    : <div className={styles.msgContent}>{content}</div>}
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
              title={modelButtonCopy.title}
              onClick={() => setShowModelPicker((v) => !v)}
            >
              <span>{modelButtonCopy.source}</span>
              <strong>{modelButtonCopy.detail}</strong>
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
            <UserProfilePopover
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

      {showModelPicker && (
        <ModelPickerPopover
          anchorRef={modelBtnRef}
          runtimeRoute={runtimeRoute}
          onRuntimeRouteChange={setRuntimeRoute}
          onClose={() => setShowModelPicker(false)}
        />
      )}
    </div>
  )
}

function UserProfilePopover({ friend, anchor, onClose }: { friend: Friend; anchor: PopoverAnchor; onClose: () => void }) {
  const navigate = useNavigate()
  const popRef = useRef<HTMLDivElement>(null)
  const name = friend.nickname ?? friend.account
  const isOnline = !!friend.is_online
  const [isFriend, setIsFriend] = useState(!!friend.already_friend)
  const [addingFriend, setAddingFriend] = useState(false)
  const [addMsg, setAddMsg] = useState('')

  useEffect(() => {
    function onDown(e: MouseEvent) {
      if (popRef.current && !popRef.current.contains(e.target as Node)) onClose()
    }
    document.addEventListener('mousedown', onDown)
    return () => document.removeEventListener('mousedown', onDown)
  }, [onClose])

  const POPOVER_WIDTH = 284
  const POPOVER_HEIGHT = 280
  const { left: popLeft, top: popTop } = fixedPopoverPosition(anchor, POPOVER_WIDTH, POPOVER_HEIGHT)

  function copyId() {
    navigator.clipboard.writeText(friend.id).catch(() => {})
  }

  async function addFriend() {
    if (isFriend || addingFriend) return
    setAddingFriend(true)
    try {
      await api.post('/api/me/friends', { query: friend.id, search_type: 'user_id' })
      setIsFriend(true)
      setAddMsg('已添加')
    } catch (err) {
      setAddMsg((err as { message?: string }).message ?? '添加失败')
    } finally {
      setAddingFriend(false)
    }
  }

  return (
    <div ref={popRef} style={{
      position: 'fixed', left: popLeft, top: popTop, zIndex: 9999,
      width: POPOVER_WIDTH, background: '#1e1f22',
      border: '1px solid rgba(255,255,255,.12)', borderRadius: 10,
      overflow: 'hidden', boxShadow: '0 8px 32px rgba(0,0,0,.55)',
    }}>
      {/* 头部 */}
      <div style={{ position: 'relative', height: 72, background: isOnline ? 'linear-gradient(135deg,#0a2d1f,#0d2012)' : '#2c2e35' }}>
        <div style={{
          position: 'absolute', bottom: -18, left: 14,
          width: 56, height: 56, borderRadius: '50%', border: '4px solid #1e1f22',
          background: '#38414a', display: 'grid', placeItems: 'center', overflow: 'hidden',
        }}>
          {friend.avatar_data_url
            ? <img src={friend.avatar_data_url} alt="" style={{ width: '100%', height: '100%', objectFit: 'cover', borderRadius: '50%' }} />
            : <span style={{ fontSize: 20, fontWeight: 800, color: 'white' }}>{name[0]?.toUpperCase()}</span>
          }
          {/* 在线小点 */}
          <span style={{
            position: 'absolute', right: 1, bottom: 1,
            width: 13, height: 13, borderRadius: '50%', border: '3px solid #1e1f22',
            background: isOnline ? 'var(--green,#58BE6A)' : '#545862',
          }} />
        </div>
        <button onClick={onClose} type="button" style={{
          position: 'absolute', top: 8, right: 8,
          width: 28, height: 28, border: 0, borderRadius: '50%',
          background: 'rgba(0,0,0,.35)', color: '#c4c8d4', fontSize: 18,
          cursor: 'pointer', display: 'grid', placeItems: 'center',
        }}>×</button>
      </div>
      {/* 主体 */}
      <div style={{ padding: '24px 14px 14px', display: 'flex', flexDirection: 'column', gap: 4 }}>
        <strong style={{ display: 'block', fontSize: 15, fontWeight: 800, color: 'var(--text)', marginTop: 6, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{name}</strong>
        <span style={{ fontSize: 12, color: 'var(--text-muted)' }}>{isOnline ? '在线' : '离线'}</span>
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 5, marginTop: 6 }}>
          <em style={{ display: 'inline-flex', alignItems: 'center', height: 17, padding: '0 6px', borderRadius: 4, border: '1px solid rgba(255,255,255,.1)', background: 'rgba(255,255,255,.04)', color: isOnline ? 'var(--green,#58BE6A)' : '#aab0bd', fontSize: 10, fontWeight: 800, fontStyle: 'normal' }}>
            {isOnline ? '在线' : '离线'}
          </em>
          {friend.id && (
            <em style={{ display: 'inline-flex', alignItems: 'center', height: 17, padding: '0 6px', borderRadius: 4, border: '1px solid rgba(255,255,255,.1)', background: 'rgba(255,255,255,.04)', color: '#aab0bd', fontSize: 10, fontWeight: 800, fontStyle: 'normal' }}>
              {friend.id.slice(0, 7).toUpperCase()}
            </em>
          )}
        </div>
        <div style={{ marginTop: 10, borderTop: '1px solid rgba(255,255,255,.06)', paddingTop: 8, display: 'flex', flexDirection: 'column', gap: 5 }}>
          {friend.account && (
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 12, gap: 8 }}>
              <span style={{ color: 'var(--text-muted)', flexShrink: 0 }}>账号</span>
              <strong style={{ color: 'var(--text-soft)', fontWeight: 500, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', textAlign: 'right' }}>{friend.account}</strong>
            </div>
          )}
          {friend.id && (
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 12, gap: 8 }}>
              <span style={{ color: 'var(--text-muted)', flexShrink: 0 }}>用户 ID</span>
              <strong style={{ color: 'var(--text-soft)', fontWeight: 500, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', textAlign: 'right' }} title={friend.id}>{friend.id.slice(0, 14).toUpperCase()}</strong>
            </div>
          )}
        </div>
        <div style={{ display: 'flex', gap: 6, marginTop: 10, flexWrap: 'wrap' }}>
          <button style={{ flex: 1, height: 30, border: '1px solid rgba(255,255,255,.12)', borderRadius: 6, background: 'rgba(255,255,255,.04)', color: 'var(--text-soft)', fontSize: 12, fontWeight: 600, cursor: 'pointer' }}
            type="button" onClick={() => { onClose(); navigate('/friends') }}>
            发消息
          </button>
          <button
            style={{ flex: 1, height: 30, border: '1px solid rgba(255,255,255,.12)', borderRadius: 6, background: isFriend ? 'rgba(88,190,106,.1)' : 'rgba(255,255,255,.04)', color: isFriend ? 'var(--green,#58BE6A)' : 'var(--text-soft)', fontSize: 12, fontWeight: 600, cursor: isFriend ? 'default' : 'pointer', opacity: addingFriend ? 0.6 : 1 }}
            type="button" onClick={addFriend} disabled={isFriend || addingFriend}>
            {addMsg || (isFriend ? '已是好友' : addingFriend ? '添加中…' : '加好友')}
          </button>
          <button style={{ flex: 1, height: 30, border: '1px solid rgba(255,255,255,.12)', borderRadius: 6, background: 'rgba(255,255,255,.04)', color: 'var(--text-soft)', fontSize: 12, fontWeight: 600, cursor: 'pointer' }}
            type="button" onClick={copyId}>
            复制 ID
          </button>
        </div>
      </div>
    </div>
  )
}
