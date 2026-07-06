import { memo } from 'react'
import { clean } from '../../lib/utils'
import { displayMessageContentOrAttachment } from '../../lib/messageDisplay'
import type { Message } from './types'
import MarkdownContent from '../markdown/MarkdownContent'
import { DevTaskMessage } from '../dev/DevTaskCard'
import { buildContext, taskResultDisplayText, taskResultTone } from '../dev/devTaskUtils'
import { formatTime } from '../../lib/utils'
import UserAvatar from '../shell/UserAvatar'
import styles from './ConversationPage.module.css'

interface MessageItemProps {
  message: Message
  isDevChannel: boolean
  taskContext: ReturnType<typeof buildContext>
  user: { nickname?: string; account?: string; avatar_data_url?: string | null } | null
  onCancel: (id: string) => Promise<void>
  onApprove: (taskId: string, approvalId: string, decision: 'approve' | 'deny') => Promise<void>
  grouped?: boolean
}

export const MessageItem = memo(function MessageItem({ message, isDevChannel, taskContext, user, onCancel, onApprove, grouped }: MessageItemProps) {
  const kind = clean(message.kind ?? message.role ?? '').toLowerCase()

  // Dev 任务消息用 DevTaskCard 渲染
  if (isDevChannel && ['ai_task', 'ai_progress', 'ai_result'].includes(kind)) {
    return (
      <div className={[styles.messageRow, styles.devTaskWrap].join(' ')}>
        <DevTaskMessage message={message} context={taskContext} onCancel={onCancel} onApprove={onApprove} />
      </div>
    )
  }

  const isAi = ['assistant', 'ai', 'bot', 'system', 'ai_task', 'ai_progress', 'ai_result'].includes(kind)
  const isUserRole = !isAi && (kind === 'user' || kind === 'human' || kind === 'discussion')
  const isOwn = isAi ? false : (typeof message.outgoing === 'boolean' ? message.outgoing : isUserRole)
  const terminalTask = isAi && isTerminalTaskStatus(message.task_status ?? message.taskStatus)
  const content = terminalTask
    ? taskResultDisplayText(message)
    : displayMessageContentOrAttachment(message.content ?? message.text ?? '')
  const taskTone = terminalTask ? taskResultTone(message.task_status ?? message.taskStatus, content) : null
  const taskStatusLabel = taskTone === 'failed' ? '任务失败' : taskTone === 'canceled' ? '任务已停止' : ''
  const contentClassName = [
    styles.messageContent,
    isAi ? styles.aiContent : '',
    taskTone === 'failed' ? styles.taskFailedContent : '',
    taskTone === 'canceled' ? styles.taskCanceledContent : '',
  ].filter(Boolean).join(' ')
  const time = message.created_at ? formatTime(message.created_at) : ''
  const senderName = clean(
    message.sender_name
      ?? (message as Record<string, unknown>).senderName
      ?? (message as Record<string, unknown>).sender_account
      ?? '',
  )
  const displayName = isAi
    ? 'AI'
    : senderName || (isOwn ? (user?.nickname ?? user?.account ?? '我') : '成员')

  // AI 回复默认支持 Markdown；用户消息至少支持图片、链接和代码片段。
  const hasMarkdown = isAi
    ? /[#*`\[\]>|]/.test(content)
    : /!\[[^\]]*]\([^)]+\)|\[[^\]]+]\([^)]+\)|`|^\s*https?:\/\/\S+?(?:\.(?:png|jpe?g|gif|webp)|\/(?:chat-)?attachments\/\S+)/im.test(content)

  return (
    <div className={[styles.messageRow, isOwn ? styles.ownRow : '', isAi ? styles.aiRow : '', grouped ? styles.grouped : ''].filter(Boolean).join(' ')}>
      <MessageAvatar message={message} isAi={isAi} isOwn={isOwn} displayName={displayName} user={user} />
      <div className={styles.messageBody}>
        <div className={styles.messageMeta}>
          <strong>{displayName}</strong>
          {time && <span>{time}</span>}
        </div>
        {hasMarkdown ? (
          <div className={[contentClassName, styles.markdownMsg].filter(Boolean).join(' ')}>
            {taskStatusLabel && <span className={styles.taskStatusLabel}>{taskStatusLabel}</span>}
            <MarkdownContent content={content} copy={isAi} />
          </div>
        ) : (
          <div className={contentClassName}>
            {taskStatusLabel && <span className={styles.taskStatusLabel}>{taskStatusLabel}</span>}
            {content}
          </div>
        )}
      </div>
    </div>
  )
}, areMessageItemPropsEqual)

function isTerminalTaskStatus(status: unknown): boolean {
  return ['done', 'completed', 'success', 'succeeded', 'finished', 'failed', 'error', 'canceled', 'cancelled', 'interrupted', 'stopped']
    .includes(clean(status ?? '').toLowerCase())
}

function MessageAvatar({
  message,
  isAi,
  isOwn,
  displayName,
  user,
}: {
  message: Message
  isAi: boolean
  isOwn: boolean
  displayName: string
  user: MessageItemProps['user']
}) {
  if (isAi) {
    return <div className={styles.messageAvatar}>AI</div>
  }
  const avatarUser = {
    id: clean(message.user_id ?? '') || user?.account || displayName,
    account: displayName,
    nickname: displayName,
    avatar_data_url: clean(
      message.sender_avatar_data_url
      ?? message.senderAvatarDataUrl
      ?? message.avatar_data_url
      ?? message.avatarDataUrl
      ?? (isOwn ? user?.avatar_data_url : '')
      ?? '',
    ) || null,
  }
  return <UserAvatar user={avatarUser} size="compact" className={styles.messageAvatar} />
}

function areMessageItemPropsEqual(
  prev: MessageItemProps,
  next: MessageItemProps,
): boolean {
  return prev.message === next.message
    && prev.isDevChannel === next.isDevChannel
    && prev.taskContext === next.taskContext
    && prev.grouped === next.grouped
    && prev.user?.nickname === next.user?.nickname
    && prev.user?.account === next.user?.account
    && prev.user?.avatar_data_url === next.user?.avatar_data_url
}
