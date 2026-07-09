import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { api, getAuthToken } from './api/client'
import { cloudWebSocketUrl } from './api/runtime'
import ConversationFeed from './features/conversation/ConversationFeed'
import { loadAiDevelopmentTaskMessages } from './features/conversation/conversationPageHelpers'
import {
  buildDisplayMessages,
  buildMessageGroups,
  buildTaskProcessMessageMap,
  hasRunningTask,
  messageConversationId,
  messageTaskId,
} from './features/conversation/messageFlow'
import type { Message, ProjectSpace } from './features/conversation/types'
import { buildContext } from './features/dev/devTaskUtils'
import { clean } from './lib/utils'

const livePreviewUser = { nickname: '钱一龙', account: 'elon', avatar_data_url: null }

export interface LivePreviewConfig {
  enabled: boolean
  projectId: string
  channelId: string
  conversationId: string
  taskId: string
  actionsEnabled: boolean
}

interface LivePreviewState {
  loading: boolean
  error: string
  messages: Message[]
  projectId: string
  channelId: string
  conversationId: string
  taskId: string
  connected: boolean
  refreshedAt: number
}

export function livePreviewConfigFromLocation(): LivePreviewConfig {
  const params = new URLSearchParams(window.location.search)
  const source = clean(params.get('source') ?? '')
  const enabled = params.get('live') === '1' || source === 'live' || source === 'real'
  return {
    enabled,
    projectId: clean(params.get('project') ?? params.get('projectId') ?? 'elon-self'),
    channelId: clean(params.get('channel') ?? params.get('channelId') ?? ''),
    conversationId: clean(params.get('conversation') ?? params.get('conversationId') ?? ''),
    taskId: clean(params.get('task') ?? params.get('taskId') ?? ''),
    actionsEnabled: params.get('actions') === '1',
  }
}

export function LivePreview({ config, expandAll, onToggleExpand }: {
  config: LivePreviewConfig
  expandAll: boolean
  onToggleExpand: () => void
}) {
  const feedRef = useRef<HTMLDivElement>(null)
  const [state, setState] = useState<LivePreviewState>({
    loading: true,
    error: '',
    messages: [],
    projectId: config.projectId || 'elon-self',
    channelId: config.channelId,
    conversationId: config.conversationId,
    taskId: config.taskId,
    connected: false,
    refreshedAt: 0,
  })

  const refresh = useCallback(async () => {
    const projectId = config.projectId || state.projectId || 'elon-self'
    setState((prev) => ({ ...prev, loading: prev.messages.length === 0, error: '' }))
    try {
      const channelId = config.channelId || state.channelId || await resolveAiDevelopmentChannel(projectId)
      if (!channelId) throw new Error('没有找到可用于回放的 AI 开发频道')
      const messages = await loadAiDevelopmentTaskMessages(projectId, channelId)
      const selected = selectLiveSession(messages, config)
      setState((prev) => ({
        ...prev,
        loading: false,
        error: '',
        messages,
        projectId,
        channelId,
        conversationId: selected.conversationId,
        taskId: selected.taskId,
        refreshedAt: Date.now(),
      }))
    } catch (err) {
      const message = (err as { message?: string }).message || '加载真实会话失败'
      setState((prev) => ({ ...prev, loading: false, error: message }))
    }
  }, [config, state.channelId, state.projectId])

  useEffect(() => {
    void refresh()
  }, [refresh])

  useEffect(() => {
    if (!state.projectId || !state.channelId) return
    const token = getAuthToken()
    if (!token) return
    let closed = false
    const url = new URL(cloudWebSocketUrl('/ws/app'))
    url.searchParams.set('token', token)
    const ws = new WebSocket(url.toString())
    ws.onopen = () => {
      if (!closed) setState((prev) => ({ ...prev, connected: true }))
    }
    ws.onclose = () => {
      if (!closed) setState((prev) => ({ ...prev, connected: false }))
    }
    ws.onerror = () => ws.close()
    ws.onmessage = (event) => {
      if (liveRealtimeEventMatches(event.data, state)) void refresh()
    }
    return () => {
      closed = true
      ws.onopen = null
      ws.onclose = null
      ws.onerror = null
      ws.onmessage = null
      ws.close()
    }
  }, [refresh, state.channelId, state.conversationId, state.projectId, state.taskId])

  const sessionView = state.conversationId || state.taskId || null
  const taskMessagesById = useMemo(() => buildTaskProcessMessageMap([state.messages]), [state.messages])
  const displayMessages = useMemo(() => buildDisplayMessages({
    sessionView,
    channelMessages: state.messages,
    conversationMessages: [],
    conversationLoading: state.loading,
    taskMessagesById,
  }), [sessionView, state.messages, state.loading, taskMessagesById])
  const messageGroups = useMemo(() => buildMessageGroups(displayMessages, true), [displayMessages])
  const taskContext = useMemo(() => buildContext(displayMessages), [displayMessages])
  const running = useMemo(() => hasRunningTask(displayMessages), [displayMessages])

  const handleCancelTask = useCallback(async (taskId: string) => {
    if (!config.actionsEnabled) {
      window.alert('真实回放默认只读。需要执行取消、继续或审批时，在地址后加 actions=1。')
      return
    }
    await api.post(`/api/projects/${encodeURIComponent(state.projectId)}/channels/${encodeURIComponent(state.channelId)}/ai-tasks/${encodeURIComponent(taskId)}/cancel`, {})
    await refresh()
  }, [config.actionsEnabled, refresh, state.channelId, state.projectId])

  const handleContinueTask = useCallback(async () => {
    if (!config.actionsEnabled) {
      window.alert('真实回放默认只读。需要继续真实任务时，在地址后加 actions=1。')
      return
    }
    if (!state.projectId || !state.channelId) return
    await api.post(`/api/projects/${encodeURIComponent(state.projectId)}/channels/${encodeURIComponent(state.channelId)}/ai-tasks`, {
      content: '继续处理这个任务。',
      conversation_id: state.conversationId || undefined,
      conversation_title: state.conversationId ? undefined : '继续处理任务',
    })
    await refresh()
  }, [config.actionsEnabled, refresh, state.channelId, state.conversationId, state.projectId])

  const handleApproveTool = useCallback(async (taskId: string, approvalId: string, decision: 'approve' | 'deny') => {
    if (!config.actionsEnabled) {
      window.alert('真实回放默认只读。需要审批真实工具时，在地址后加 actions=1。')
      return
    }
    await api.post(
      `/api/projects/${encodeURIComponent(state.projectId)}/channels/${encodeURIComponent(state.channelId)}/ai-tasks/${encodeURIComponent(taskId)}/tool-approvals/${encodeURIComponent(approvalId)}/decision`,
      { decision },
    )
    await refresh()
  }, [config.actionsEnabled, refresh, state.channelId, state.projectId])

  return (
    <main className="previewPage livePreviewPage">
      <aside className="previewRail">
        <strong>真实任务回放</strong>
        <button
          type="button"
          className="expandToggle"
          data-active={expandAll ? 'true' : undefined}
          onClick={onToggleExpand}
        >
          {expandAll ? '全部展开中' : '按真实默认'}
        </button>
        <button type="button" className="expandToggle" onClick={() => void refresh()}>
          重新加载
        </button>
        <div className="liveFacts">
          <span>project {state.projectId || '-'}</span>
          <span>channel {shortLiveId(state.channelId)}</span>
          <span>conversation {shortLiveId(state.conversationId)}</span>
          <span>task {shortLiveId(state.taskId)}</span>
          <span>{state.connected ? 'WebSocket 已连接' : 'WebSocket 未连接'}</span>
          <span>{config.actionsEnabled ? '允许真实操作' : '只读回放'}</span>
        </div>
      </aside>
      <section className="previewSingle livePreviewStage">
        <article className="scenarioFrame liveScenarioFrame">
          <header>
            <strong>真实 PC Codex 生命周期</strong>
            <span>{running ? '运行中' : '已静止'}</span>
            {state.refreshedAt > 0 && <span>{new Date(state.refreshedAt).toLocaleTimeString('zh-CN', { hour12: false })}</span>}
          </header>
          <div className="conversationReplay liveConversationReplay">
            <div className="replayTopbar">
              <strong>AI 开发频道</strong>
              <span>{state.loading ? '正在加载真实消息' : `${displayMessages.length} 条可见消息`}</span>
            </div>
            {state.error ? (
              <div className="liveError">{state.error}</div>
            ) : (
              <ConversationFeed
                sessionView={sessionView || 'new'}
                feedRef={feedRef}
                feedLoading={state.loading}
                displayMessages={displayMessages}
                messageGroups={messageGroups}
                taskContext={taskContext}
                isDevChannel
                user={livePreviewUser}
                sendingMessage={false}
                onScroll={() => undefined}
                onCancelTask={handleCancelTask}
                onContinueTask={handleContinueTask}
                onApproveTool={handleApproveTool}
                debugExpandAll={expandAll}
              />
            )}
            <div className="replayComposer">
              <button type="button" aria-label="添加附件">+</button>
              <div className="replayInput">真实回放模式不会自动发送新任务...</div>
              <span>{config.actionsEnabled ? 'actions=1' : 'readonly'}</span>
              <button type="button" aria-label="发送">›</button>
            </div>
          </div>
        </article>
      </section>
    </main>
  )
}

async function resolveAiDevelopmentChannel(projectId: string): Promise<string> {
  const space = await api.get<ProjectSpace>(`/api/projects/${encodeURIComponent(projectId)}/space`)
  const channels = Array.isArray(space.channels) ? space.channels : []
  const devChannel = channels.find((channel) => channel.kind === 'ai_development')
    ?? channels.find((channel) => /ai|开发|codex/i.test(channel.name ?? ''))
    ?? channels[0]
  return clean(devChannel?.id ?? '')
}

function selectLiveSession(messages: Message[], config: LivePreviewConfig): { conversationId: string; taskId: string } {
  if (config.conversationId || config.taskId) {
    const scoped = config.taskId
      ? messages.filter((message) => messageTaskId(message) === config.taskId)
      : messages.filter((message) => messageConversationId(message) === config.conversationId)
    const latest = latestMessage(scoped) ?? latestMessage(messages)
    return {
      conversationId: config.conversationId || (latest ? messageConversationId(latest) : ''),
      taskId: config.taskId || (latest ? messageTaskId(latest) : ''),
    }
  }

  const latestTaskMessage = latestMessage(messages.filter((message) => messageTaskId(message)))
  const fallbackMessage = latestMessage(messages)
  return {
    conversationId: latestTaskMessage ? messageConversationId(latestTaskMessage) : fallbackMessage ? messageConversationId(fallbackMessage) : '',
    taskId: latestTaskMessage ? messageTaskId(latestTaskMessage) : '',
  }
}

function latestMessage(messages: Message[]): Message | undefined {
  return [...messages].sort((a, b) => timestampMs(b) - timestampMs(a))[0]
}

function timestampMs(message: Message | undefined): number {
  if (!message) return 0
  const value = Date.parse(String(message.created_at ?? ''))
  return Number.isFinite(value) ? value : 0
}

function liveRealtimeEventMatches(raw: string, state: LivePreviewState): boolean {
  try {
    const data = JSON.parse(raw) as Record<string, unknown>
    const type = clean(data.type)
    if (!/project_(message|task)|conversation|task/.test(type)) return false
    const projectId = clean(data.projectId ?? data.project_id ?? '')
    const channelId = clean(data.channelId ?? data.channel_id ?? '')
    const conversationId = clean(data.conversationId ?? data.conversation_id ?? '')
    const taskId = clean(data.taskId ?? data.task_id ?? '')
    if (projectId && state.projectId && projectId !== state.projectId) return false
    if (channelId && state.channelId && channelId !== state.channelId) return false
    if (conversationId && state.conversationId && conversationId !== state.conversationId) return false
    if (taskId && state.taskId && taskId !== state.taskId) return false
    return Boolean(projectId || channelId || conversationId || taskId)
  } catch {
    return false
  }
}

function shortLiveId(value: string): string {
  const text = clean(value)
  if (!text) return '-'
  return text.length > 14 ? `${text.slice(0, 8)}...` : text
}
