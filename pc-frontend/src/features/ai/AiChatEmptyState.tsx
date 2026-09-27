import type { ComponentProps } from 'react'
import AiChatWelcome from './AiChatWelcome'
import AiWebConversationReadStatus from '../user-browser/AiWebConversationReadStatus'
import styles from './AiChatPage.module.css'

type Props = ComponentProps<typeof AiChatWelcome> & { hasMessages: boolean; loading: boolean }
export default function AiChatEmptyState({ hasMessages, loading, ...welcome }: Props) {
  if (welcome.chatMode && welcome.web.controller.readTarget) {
    return <AiWebConversationReadStatus web={welcome.web} />
  }
  if (loading) return <p className={styles.hint}>{welcome.chatMode ? '正在连接本地网页 AI…' : '读取消息…'}</p>
  return hasMessages ? null : <AiChatWelcome {...welcome} />
}
