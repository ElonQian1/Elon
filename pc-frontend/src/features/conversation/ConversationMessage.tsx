import { clean } from '../../lib/utils'
import type { Message } from './types'
import MarkdownContent from '../markdown/MarkdownContent'
import { DevTaskMessage } from '../dev/DevTaskCard'
import { buildContext } from '../dev/devTaskUtils'
import { formatTime } from '../../lib/utils'
import styles from './ConversationPage.module.css'

export function MessageItem({ message, isDevChannel, taskContext, user, onCancel, onApprove, grouped }: {
  message: Message
  isDevChannel: boolean
  taskContext: ReturnType<typeof buildContext>
  user: { nickname?: string; account?: string } | null
  onCancel: (id: string) => Promise<void>
  onApprove: (taskId: string, approvalId: string, decision: 'approve' | 'deny') => Promise<void>
  grouped?: boolean
}) {
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
  const content = clean(message.content ?? message.text ?? '')
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

  // AI 消息：检测是否含 Markdown 特征，有则渲染 Markdown
  const hasMarkdown = isAi && /[#*`\[\]>|]/.test(content)

  return (
    <div className={[styles.messageRow, isOwn ? styles.ownRow : '', isAi ? styles.aiRow : '', grouped ? styles.grouped : ''].filter(Boolean).join(' ')}>
      <div className={styles.messageAvatar}>
        {isUserRole
          ? (displayName[0]?.toUpperCase() ?? '我')
          : 'AI'}
      </div>
      <div className={styles.messageBody}>
        <div className={styles.messageMeta}>
          <strong>{displayName}</strong>
          {time && <span>{time}</span>}
        </div>
        {hasMarkdown ? (
          <div className={[styles.messageContent, styles.aiContent, styles.markdownMsg].join(' ')}>
            <MarkdownContent content={content} copy />
          </div>
        ) : (
          <div className={[styles.messageContent, isAi ? styles.aiContent : ''].join(' ')}>
            {content}
          </div>
        )}
      </div>
    </div>
  )
}
