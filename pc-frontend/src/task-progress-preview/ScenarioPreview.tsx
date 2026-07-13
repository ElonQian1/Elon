import { useMemo, useRef, useState } from 'react'
import ConversationFeed from '../features/conversation/ConversationFeed'
import { buildDisplayMessages, buildMessageGroups } from '../features/conversation/messageFlow'
import type { Message } from '../features/conversation/types'
import { buildContext } from '../features/dev/devTaskUtils'
import type { ChatMessage } from '../features/dev/types'

export const previewTaskId = 'tsk_preview_20260708'
export const previewStartedAt = '2026-07-08T10:17:00+08:00'

interface PreviewScenario {
  id: string
  label: string
  width: number
  messages: ChatMessage[]
  localNodeReady?: boolean
  localNodeRequired?: boolean
}

const previewUser = { nickname: '钱一龙', account: 'elon', avatar_data_url: null }

export function ScenarioPreview({ scenario, expandAll }: { scenario: PreviewScenario; expandAll: boolean }) {
  const feedRef = useRef<HTMLDivElement>(null)
  const [localNodeReady, setLocalNodeReady] = useState(scenario.localNodeReady ?? true)
  const [continuedCount, setContinuedCount] = useState(0)
  const conversationId = `preview-conversation-${scenario.id}`
  const openProcessInPlace = expandAll
  const taskMessages = useMemo(() => scenario.messages as Message[], [scenario])
  const conversationMessages = useMemo<Message[]>(() => [requestMessage(scenario, conversationId)], [conversationId, scenario])
  const taskMessagesById = useMemo(() => new Map([[previewTaskId, taskMessages]]), [taskMessages])
  const displayMessages = useMemo(() => buildDisplayMessages({
    sessionView: conversationId,
    channelMessages: [],
    conversationMessages,
    conversationLoading: false,
    taskMessagesById,
  }), [conversationId, conversationMessages, taskMessagesById])
  const messageGroups = useMemo(() => buildMessageGroups(displayMessages, true), [displayMessages])
  const taskContext = useMemo(() => buildContext(displayMessages), [displayMessages])

  return (
    <article className="scenarioFrame" style={{ maxWidth: scenario.width }}>
      <header>
        <strong>{scenario.label}</strong>
        <span>{scenario.width}px</span>
      </header>
      <div className="conversationReplay">
        <div className="replayTopbar">
          <strong>AI 开发频道</strong>
          <div className="replayStatus">
            <span>{scenario.label}</span>
            {scenario.localNodeRequired && (
              <button
                type="button"
                data-node-ready={localNodeReady ? 'true' : undefined}
                onClick={() => setLocalNodeReady(true)}
                disabled={localNodeReady}
              >
                {localNodeReady ? (continuedCount > 0 ? '节点在线 · 已续跑' : '节点在线') : '模拟节点重连'}
              </button>
            )}
          </div>
        </div>
        <ConversationFeed
          sessionView={conversationId}
          feedRef={feedRef}
          feedLoading={false}
          displayMessages={displayMessages}
          messageGroups={messageGroups}
          taskContext={taskContext}
          isDevChannel
          user={previewUser}
          sendingMessage={false}
          localNodeReady={localNodeReady}
          localNodeRequired={scenario.localNodeRequired}
          onScroll={() => undefined}
          onCancelTask={noopTaskAction}
          onContinueTask={async () => setContinuedCount((count) => count + 1)}
          onApproveTool={noopApprovalAction}
          debugOpenProcess={openProcessInPlace}
        />
        <div className="replayComposer">
          <button type="button" aria-label="添加附件">+</button>
          <div className="replayInput">以钱一龙的账号在 AI 开发频道发送消息...</div>
          <span>GPT-5.5</span>
          <button type="button" aria-label="发送">›</button>
        </div>
      </div>
    </article>
  )
}

function requestMessage(scenario: PreviewScenario, conversationId: string): Message {
  const request = scenario.messages.find((message) => message.kind === 'ai_task')?.content
    ?? scenario.messages[0]?.content
    ?? scenario.label
  return {
    id: `preview-user-${scenario.id}`,
    kind: 'user',
    role: 'user',
    task_id: previewTaskId,
    taskId: previewTaskId,
    conversation_id: conversationId,
    conversationId,
    sender_name: '钱一龙',
    user_id: 'preview-user',
    outgoing: true,
    content: request,
    text: request,
    created_at: previewStartedAt,
  }
}

async function noopTaskAction() {}

async function noopApprovalAction() {}
