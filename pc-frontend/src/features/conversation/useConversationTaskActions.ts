import { useCallback } from 'react'
import type { Dispatch, MutableRefObject, SetStateAction } from 'react'
import { v4 as uuidv4 } from 'uuid'
import { api } from '../../api/client'
import { safeNodeAdminUrl } from '../../lib/utils'
import { selectedAgentForRuntimeRoute } from '../models/routeModelPolicy'
import type { AgentOption } from '../models/types'
import { ensureLocalFullAccessGrant } from './localPcRuntime'
import type { RuntimeRoute } from './runtimeRoutes'
import type { SendMessageResponse } from './types'

type SessionView = string | 'new' | null

type SendMessageFn = (
  content: string,
  agent?: string | null,
  runtimeRoute?: RuntimeRoute,
  conversationId?: string | null,
  conversationTitle?: string | null,
  localNodeId?: string | null,
  localWorkspacePath?: string | null,
  channelIdOverride?: string | null,
  directPcCli?: boolean,
) => Promise<SendMessageResponse | null>

interface UseConversationTaskActionsOptions {
  activeProjectId: string
  activeProjectName?: string
  activeWorkspacePath: string
  taskActionChannelId: string
  runtimePermission?: string
  sendingMessage: boolean
  directPcCliActive: boolean
  runtimeRoute: RuntimeRoute
  shouldPreferLocalNode: boolean
  localNodeReady: boolean
  localNodeId: string
  selectedAgent: string
  modelOptions: AgentOption[]
  sessionView: SessionView
  draftConversationId: string
  waitingForNewSession: MutableRefObject<boolean>
  sendMessage: SendMessageFn
  refreshTaskSurface: () => Promise<void>
  requestFeedAutoFollow: () => void
  setSendError: Dispatch<SetStateAction<string>>
  setSessionView: Dispatch<SetStateAction<SessionView>>
}

export function useConversationTaskActions({
  activeProjectId,
  activeProjectName,
  activeWorkspacePath,
  taskActionChannelId,
  runtimePermission,
  sendingMessage,
  directPcCliActive,
  runtimeRoute,
  shouldPreferLocalNode,
  localNodeReady,
  localNodeId,
  selectedAgent,
  modelOptions,
  sessionView,
  draftConversationId,
  waitingForNewSession,
  sendMessage,
  refreshTaskSurface,
  requestFeedAutoFollow,
  setSendError,
  setSessionView,
}: UseConversationTaskActionsOptions) {
  const handleCancelTask = useCallback(async (taskId: string) => {
    if (!activeProjectId || !taskActionChannelId) {
      setSendError('当前项目没有可操作的 AI 开发频道')
      return
    }
    await api.post(
      `/api/projects/${encodeURIComponent(activeProjectId)}/channels/${encodeURIComponent(taskActionChannelId)}/ai-tasks/${encodeURIComponent(taskId)}/cancel`,
      {},
    )
    await refreshTaskSurface()
  }, [activeProjectId, refreshTaskSurface, setSendError, taskActionChannelId])

  const handleContinueTask = useCallback(async (taskId: string) => {
    if (!taskId || sendingMessage) return
    if (!activeProjectId || !taskActionChannelId) {
      setSendError('当前项目没有可操作的 AI 开发频道')
      return
    }
    setSendError('')
    try {
      requestFeedAutoFollow()
      const directPcCliForRequest = directPcCliActive
      const requestRuntimeRoute: RuntimeRoute = directPcCliForRequest ? 'route_a' : runtimeRoute
      const useLocalNodeForRequest = (directPcCliForRequest || shouldPreferLocalNode) && localNodeReady
      const requestAgent = selectedAgentForRuntimeRoute(selectedAgent, modelOptions, requestRuntimeRoute)
      await ensureLocalFullAccessGrant({
        adminUrl: safeNodeAdminUrl(),
        projectId: activeProjectId,
        projectName: activeProjectName,
        workspacePath: activeWorkspacePath,
        runtimePermission,
        useLocalRouteA: useLocalNodeForRequest && requestRuntimeRoute === 'route_a',
      })
      const isExistingConversation = typeof sessionView === 'string' && sessionView !== 'new'
      const conversationId = isExistingConversation ? sessionView : (draftConversationId || uuidv4())
      const response = await sendMessage(
        '继续处理这个任务。',
        requestAgent || null,
        requestRuntimeRoute,
        conversationId,
        isExistingConversation ? null : '继续处理任务',
        useLocalNodeForRequest ? localNodeId : null,
        useLocalNodeForRequest ? activeWorkspacePath : null,
        taskActionChannelId,
        directPcCliForRequest,
      )
      setSessionView(response?.conversation_id ?? conversationId)
      waitingForNewSession.current = false
      await refreshTaskSurface()
    } catch (err) {
      setSendError((err as { message?: string }).message ?? '继续任务失败')
    }
  }, [
    activeProjectId,
    activeProjectName,
    activeWorkspacePath,
    directPcCliActive,
    draftConversationId,
    localNodeId,
    localNodeReady,
    modelOptions,
    refreshTaskSurface,
    requestFeedAutoFollow,
    runtimePermission,
    runtimeRoute,
    selectedAgent,
    sendMessage,
    sendingMessage,
    sessionView,
    setSendError,
    setSessionView,
    shouldPreferLocalNode,
    taskActionChannelId,
    waitingForNewSession,
  ])

  const handleApproveTool = useCallback(async (taskId: string, approvalId: string, decision: 'approve' | 'deny') => {
    if (!activeProjectId || !taskActionChannelId) {
      setSendError('当前项目没有可操作的 AI 开发频道')
      return
    }
    await api.post(
      `/api/projects/${encodeURIComponent(activeProjectId)}/channels/${encodeURIComponent(taskActionChannelId)}/ai-tasks/${encodeURIComponent(taskId)}/tool-approvals/${encodeURIComponent(approvalId)}/decision`,
      { decision },
    )
    await refreshTaskSurface()
  }, [activeProjectId, refreshTaskSurface, setSendError, taskActionChannelId])

  return { handleCancelTask, handleContinueTask, handleApproveTool }
}
