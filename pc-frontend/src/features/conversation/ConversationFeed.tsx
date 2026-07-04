import type { RefObject } from 'react'
import DevTaskGroup from '../dev/DevTaskGroup'
import type { TaskContext } from '../dev/types'
import { MessageItem } from './ConversationMessage'
import type { Message } from './types'
import type { MessageGroup } from './messageFlow'
import styles from './ConversationPage.module.css'

interface ConversationFeedProps {
  sessionView: string | 'new'
  feedRef: RefObject<HTMLDivElement>
  feedLoading: boolean
  displayMessages: Message[]
  messageGroups: MessageGroup[]
  taskContext: TaskContext
  isDevChannel: boolean
  user: { nickname?: string; account?: string; avatar_data_url?: string | null } | null
  sendingMessage: boolean
  onScroll: () => void
  onCancelTask: (id: string) => Promise<void>
  onApproveTool: (taskId: string, approvalId: string, decision: 'approve' | 'deny') => Promise<void>
}

export default function ConversationFeed({
  sessionView,
  feedRef,
  feedLoading,
  displayMessages,
  messageGroups,
  taskContext,
  isDevChannel,
  user,
  sendingMessage,
  onScroll,
  onCancelTask,
  onApproveTool,
}: ConversationFeedProps) {
  return (
    <div className={styles.messageList} ref={feedRef} onScroll={onScroll}>
      {feedLoading && displayMessages.length === 0 && (
        <div className={styles.emptyState} style={{ marginTop: '4vh' }}>
          <p>正在打开会话…</p>
        </div>
      )}
      {!feedLoading && displayMessages.length === 0 && (
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
              messages={group.messages as Parameters<typeof DevTaskGroup>[0]['messages']}
              taskContext={taskContext}
              user={user}
              onCancel={onCancelTask}
              onApprove={onApproveTool}
            />
          </div>
        ) : (
          <MessageItem
            key={group.key}
            message={group.message}
            isDevChannel={isDevChannel}
            taskContext={taskContext}
            user={user}
            onCancel={onCancelTask}
            onApprove={onApproveTool}
            grouped={group.grouped}
          />
        ),
      )}
      {sendingMessage && (
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
  )
}
