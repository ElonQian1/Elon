import { useCallback, useEffect, useMemo, useState, type KeyboardEvent } from 'react'
import { GitBranch, MessageSquareText, RefreshCw, Send, X } from 'lucide-react'
import { api } from '../../api/client'
import { clean } from '../../lib/utils'
import { useAuthStore } from '../../store/auth'
import ConversationFeed from '../conversation/ConversationFeed'
import { loadAiDevelopmentTaskMessages } from '../conversation/conversationPageHelpers'
import { listMemberConversationMessages, targetFromUser } from '../conversation/memberConversationApi'
import {
  buildDisplayMessages,
  buildMessageGroups,
  buildTaskProcessMessageMap,
  hasRunningTask,
} from '../conversation/messageFlow'
import { useConversationAutoScroll } from '../conversation/useConversationAutoScroll'
import { useProjectStore } from '../conversation/useProjectStore'
import type { Channel, Message, Project } from '../conversation/types'
import { buildContext } from '../dev/devTaskUtils'
import type { UiTunerCodexContextPack } from './contextPack'
import type { UiTunerProjectSessionRecord } from './projectSessions'
import {
  uiTunerConversationSeed,
  uiTunerElementLabel,
  uiTunerSendDisabledReason,
  uiTunerSessionLabel,
  type UiTunerConversationMode,
} from './uiTunerConversation'
import panelStyles from './UiTunerPanels.module.css'

interface UiTunerConversationDrawerProps {
  open: boolean
  onClose: () => void
  pack: UiTunerCodexContextPack
  intent: string
  activeProject?: Project | null
  aiChannel?: Channel | null
  workspacePath: string
  canStart: boolean
  localNodeReady: boolean
  localNodeStatusText: string
  selectedSession: UiTunerProjectSessionRecord | null
  visibleSessions: UiTunerProjectSessionRecord[]
  status: string
  onSelectSession: (sessionId: string) => void
  onStartSession: (
    mode: UiTunerConversationMode,
    overrideIntent?: string,
  ) => Promise<UiTunerProjectSessionRecord | null>
}

export default function UiTunerConversationDrawer({
  open,
  onClose,
  pack,
  intent,
  activeProject,
  aiChannel,
  workspacePath,
  canStart,
  localNodeReady,
  localNodeStatusText,
  selectedSession,
  visibleSessions,
  status,
  onSelectSession,
  onStartSession,
}: UiTunerConversationDrawerProps) {
  const user = useAuthStore((state) => state.user)
  const messages = useProjectStore((state) => state.messages)
  const sendingMessage = useProjectStore((state) => state.sendingMessage)
  const loadMessages = useProjectStore((state) => state.loadMessages)
  const [draft, setDraft] = useState(() => uiTunerConversationSeed(pack, intent))
  const [feedLoading, setFeedLoading] = useState(false)
  const [conversationMessages, setConversationMessages] = useState<Message[]>([])
  const [sessionTaskMessages, setSessionTaskMessages] = useState<Message[]>([])
  const [drawerError, setDrawerError] = useState('')

  const projectId = activeProject?.id ?? ''
  const channelId = aiChannel?.id ?? ''
  const target = targetFromUser(user)
  const targetUserId = target?.userId ?? ''
  const sessionView = selectedSession?.conversationId || 'new'
  const elementLabel = uiTunerElementLabel(pack)
  const selectedSource = clean(pack.runtimeBinding.sourceFile)
  const selectedResource = clean(pack.runtimeBinding.resourceId)
  const disabledReason = uiTunerSendDisabledReason({
    hasProject: !!projectId,
    hasChannel: !!channelId,
    canStart,
    localNodeStatusText,
    workspacePath,
  })
  const cannotSend = !!disabledReason || sendingMessage || !draft.trim()

  const loadConversation = useCallback(async (
    session: UiTunerProjectSessionRecord | null = selectedSession,
    showSpinner = true,
  ) => {
    if (!open || !projectId || !channelId || !targetUserId || !session?.conversationId) {
      setConversationMessages([])
      setSessionTaskMessages([])
      return
    }
    if (showSpinner) setFeedLoading(true)
    setDrawerError('')
    try {
      const [conversation, taskMessages] = await Promise.all([
        listMemberConversationMessages(projectId, targetUserId, session.conversationId),
        loadAiDevelopmentTaskMessages(projectId, channelId),
        loadMessages(projectId, channelId),
      ])
      setConversationMessages(conversation as Message[])
      setSessionTaskMessages(taskMessages)
    } catch (error) {
      setDrawerError((error as { message?: string }).message ?? '项目会话加载失败')
    } finally {
      if (showSpinner) setFeedLoading(false)
    }
  }, [channelId, loadMessages, open, projectId, selectedSession, targetUserId])

  useEffect(() => {
    if (!open) return
    setDraft(uiTunerConversationSeed(pack, intent))
    void loadConversation(selectedSession)
  }, [intent, loadConversation, open, pack, selectedSession])

  const taskMessagesById = useMemo(
    () => buildTaskProcessMessageMap([messages, sessionTaskMessages]),
    [messages, sessionTaskMessages],
  )
  const displayMessages = useMemo(
    () => buildDisplayMessages({
      sessionView,
      channelMessages: messages,
      conversationMessages,
      conversationLoading: feedLoading,
      taskMessagesById,
    }),
    [conversationMessages, feedLoading, messages, sessionView, taskMessagesById],
  )
  const messageGroups = useMemo(() => buildMessageGroups(displayMessages, true), [displayMessages])
  const taskContext = useMemo(
    () => buildContext(displayMessages as Parameters<typeof buildContext>[0]),
    [displayMessages],
  )
  const taskRunning = useMemo(() => hasRunningTask(displayMessages), [displayMessages])
  const {
    feedRef,
    handleFeedScroll,
    requestFeedAutoFollow,
    showNewMsg,
  } = useConversationAutoScroll({
    messages,
    convMessages: conversationMessages,
    sessionTaskMessages,
    sessionView,
    sendingMessage,
    sendingMemberDiscussion: false,
  })

  useEffect(() => {
    if (!open || !taskRunning) return
    const timer = window.setInterval(() => {
      void loadConversation(selectedSession, false)
    }, 4000)
    return () => window.clearInterval(timer)
  }, [loadConversation, open, selectedSession, taskRunning])

  const submit = useCallback(async (mode: UiTunerConversationMode) => {
    const content = draft.trim()
    if (!content || cannotSend) return
    const session = await onStartSession(mode, content)
    if (!session) return
    setDraft('')
    requestFeedAutoFollow()
    window.setTimeout(() => {
      void loadConversation(session, false)
    }, 600)
  }, [cannotSend, draft, loadConversation, onStartSession, requestFeedAutoFollow])

  const handleComposerKeyDown = useCallback((event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key !== 'Enter' || event.shiftKey) return
    event.preventDefault()
    void submit('continue')
  }, [submit])

  const cancelTask = useCallback(async (taskId: string) => {
    if (!projectId || !channelId) return
    await api.post(
      `/api/projects/${encodeURIComponent(projectId)}/channels/${encodeURIComponent(channelId)}/ai-tasks/${encodeURIComponent(taskId)}/cancel`,
      {},
    )
    await loadConversation(selectedSession, false)
  }, [channelId, loadConversation, projectId, selectedSession])

  const continueTask = useCallback(async () => {
    const session = await onStartSession(
      'continue',
      '继续处理这个任务。先检查当前 task 和本机 sidecar 最新状态，再从中断点推进。',
    )
    if (session) await loadConversation(session, false)
  }, [loadConversation, onStartSession])

  const approveTool = useCallback(async (
    taskId: string,
    approvalId: string,
    decision: 'approve' | 'deny',
  ) => {
    if (!projectId || !channelId) return
    await api.post(
      `/api/projects/${encodeURIComponent(projectId)}/channels/${encodeURIComponent(channelId)}/ai-tasks/${encodeURIComponent(taskId)}/tool-approvals/${encodeURIComponent(approvalId)}/decision`,
      { decision },
    )
    await loadConversation(selectedSession, false)
  }, [channelId, loadConversation, projectId, selectedSession])

  if (!open) return null

  return (
    <>
      <button type="button" className={panelStyles.conversationDrawerBackdrop} onClick={onClose} aria-label="关闭项目会话" />
      <aside
        className={panelStyles.conversationDrawer}
        data-ui-tuner-conversation-drawer="open"
        data-ui-tuner-context-element={elementLabel}
      >
        <header className={panelStyles.conversationHeader}>
          <div>
            <span>项目 Codex 会话</span>
            <strong>{uiTunerSessionLabel(selectedSession)}</strong>
          </div>
          <div className={panelStyles.conversationHeaderActions}>
            <button type="button" title="刷新会话" onClick={() => void loadConversation(selectedSession)}>
              <RefreshCw size={15} aria-hidden="true" />
            </button>
            <button type="button" title="关闭" onClick={onClose}>
              <X size={15} aria-hidden="true" />
            </button>
          </div>
        </header>

        <section className={panelStyles.conversationElement}>
          <MessageSquareText size={16} aria-hidden="true" />
          <div>
            <strong>{elementLabel}</strong>
            <span>{selectedResource || selectedSource || '当前 context pack 会随消息发送'}</span>
          </div>
        </section>

        <div className={panelStyles.conversationSessionBar}>
          <select
            value={selectedSession?.id ?? ''}
            onChange={(event) => onSelectSession(event.currentTarget.value)}
            disabled={visibleSessions.length === 0}
          >
            <option value="">新建 ui-tuner 会话</option>
            {visibleSessions.map((session) => (
              <option key={session.id} value={session.id}>{uiTunerSessionLabel(session)}</option>
            ))}
          </select>
          <span>{localNodeReady ? 'Codex CLI 可用' : localNodeStatusText}</span>
        </div>

        <div className={panelStyles.conversationFeedShell}>
          <ConversationFeed
            sessionView={sessionView}
            feedRef={feedRef}
            feedLoading={feedLoading}
            displayMessages={displayMessages}
            messageGroups={messageGroups}
            taskContext={taskContext}
            isDevChannel
            user={user}
            sendingMessage={sendingMessage}
            onScroll={handleFeedScroll}
            onCancelTask={cancelTask}
            onContinueTask={continueTask}
            onApproveTool={approveTool}
            debugOpenProcess
          />
          {showNewMsg && (
            <button type="button" className={panelStyles.conversationNewMsg} onClick={requestFeedAutoFollow}>
              查看最新消息
            </button>
          )}
        </div>

        <footer className={panelStyles.conversationComposer}>
          {(drawerError || status || disabledReason) && (
            <p className={panelStyles.conversationHint}>{drawerError || status || disabledReason}</p>
          )}
          <textarea
            value={draft}
            onChange={(event) => setDraft(event.currentTarget.value)}
            onKeyDown={handleComposerKeyDown}
            placeholder="告诉 Codex 你想怎么调整当前 APK 元素"
            className={panelStyles.conversationTextarea}
          />
          <div className={panelStyles.conversationActions}>
            <button type="button" disabled={cannotSend} onClick={() => void submit('fork')}>
              <GitBranch size={14} aria-hidden="true" />
              分叉
            </button>
            <button type="button" disabled={cannotSend} onClick={() => void submit('continue')}>
              <Send size={14} aria-hidden="true" />
              发送到 Codex
            </button>
          </div>
        </footer>
      </aside>
    </>
  )
}
