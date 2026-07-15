import { useLayoutEffect, useMemo, useRef } from 'react'
import ConversationFeed from '../features/conversation/ConversationFeed'
import { buildDisplayMessages, buildMessageGroups, buildTaskProcessMessageMap } from '../features/conversation/messageFlow'
import type { Message } from '../features/conversation/types'
import { buildContext } from '../features/dev/devTaskUtils'
import { replayMessagesAtFrame, type ReplayCapture, type ReplayFrame } from './model'

const replayUser = { nickname: '钱一龙', account: 'elon', avatar_data_url: null }

export function ReplayConversation({ capture, frame, expandTools, compact = false, onRendered }: {
  capture: ReplayCapture
  frame: ReplayFrame
  expandTools: boolean
  compact?: boolean
  onRendered?: (root: HTMLElement) => void
}) {
  const feedRef = useRef<HTMLDivElement>(null)
  const rootRef = useRef<HTMLDivElement>(null)
  const messages = useMemo(() => replayMessagesAtFrame(capture, frame), [capture, frame])
  const taskMessagesById = useMemo(() => buildTaskProcessMessageMap([messages]), [messages])
  const displayMessages = useMemo(() => buildDisplayMessages({
    sessionView: capture.conversationId,
    channelMessages: messages,
    conversationMessages: [],
    conversationLoading: false,
    taskMessagesById,
  }), [capture.conversationId, messages, taskMessagesById])
  const messageGroups = useMemo(() => buildMessageGroups(displayMessages, true), [displayMessages])
  const taskContext = useMemo(() => buildContext(displayMessages), [displayMessages])

  useLayoutEffect(() => {
    if (rootRef.current) onRendered?.(rootRef.current)
  }, [displayMessages, expandTools, onRendered])

  return (
    <div className={compact ? 'replayConversation replayConversationCompact' : 'replayConversation'} ref={rootRef} data-replay-conversation>
      <div data-replay-feed>
        <ConversationFeed
          sessionView={capture.conversationId}
          feedRef={feedRef}
          feedLoading={false}
          displayMessages={displayMessages}
          messageGroups={messageGroups}
          taskContext={taskContext}
          isDevChannel
          user={replayUser}
          sendingMessage={false}
          onScroll={() => undefined}
          onCancelTask={noopTaskAction}
          onContinueTask={noopTaskAction}
          onApproveTool={noopApprovalAction}
          debugExpandAll={expandTools}
        />
      </div>
      {!compact && (
        <div className="replayComposer replayWorkbenchComposer" data-replay-composer>
          <button type="button" aria-label="添加附件">+</button>
          <div className="replayInput">逐帧回放为只读模式</div>
          <span>readonly</span>
          <button type="button" aria-label="发送">›</button>
        </div>
      )}
    </div>
  )
}

async function noopTaskAction() {}
async function noopApprovalAction() {}

export function replayVisibleMessageCount(capture: ReplayCapture, frame: ReplayFrame): number {
  return replayMessagesAtFrame(capture, frame).filter((message: Message) => Boolean(message.content ?? message.text)).length
}
