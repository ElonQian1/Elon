import { useState } from 'react'
import { Send } from 'lucide-react'
import SocialDialog from '../SocialDialog'
import { messageText } from '../socialChatOperations'
import type { SocialMessage } from '../socialMessageTypes'
import useLocalAiOwnerIdentity from '../../user-browser/useLocalAiOwnerIdentity'
import { getDesktopInvoke } from '../../shell/desktopShell'
import { startGroupAi } from './groupAiStore'
import type { GroupAiInput } from './groupAiTask'
import styles from './GroupAi.module.css'

interface Props { owner: string; group: string; title: string; messages: SocialMessage[]; onClose: () => void }

export default function GroupAiSelectionDialog(props: Props) {
  const [question, setQuestion] = useState('请结合所选消息，分析并回答。')
  const [provider, setProvider] = useState<GroupAiInput['provider']>('chatgpt')
  const [error, setError] = useState('')
  const identity = useLocalAiOwnerIdentity()
  const unavailable = !getDesktopInvoke() ? '请在一龙 Windows 客户端使用网页 AI'
    : identity.source === 'conflict' ? identity.detail : ''
  function submit() {
    try {
      if (unavailable) throw new Error(unavailable)
      startGroupAi({
        owner: props.owner, group: props.group, title: props.title, source: props.messages[0].id, provider,
        selection: {
          message_ids: props.messages.map(m => m.id),
          message_revisions: Object.fromEntries(props.messages.map(m => [m.id, m.revision ?? 1])),
          question: question.trim(),
        },
      })
      props.onClose()
    } catch (failure) { setError((failure as Error).message) }
  }
  return <SocialDialog title="AI 回复到群聊" onClose={props.onClose} footer={<>
    <button type="button" onClick={props.onClose}>取消</button>
    <button type="button" className={styles.submit} disabled={!!unavailable || !question.trim() || !props.messages.length} onClick={submit}>
      <Send size={16} aria-hidden="true" />分析并发送到群聊
    </button>
  </>}>
    <p className={styles.destination}>发送到：<strong>{props.title}</strong></p>
    <label>回答来源<select value={provider} onChange={event => setProvider(event.target.value as GroupAiInput['provider'])}>
      <option value="chatgpt">ChatGPT</option><option value="google-ai-mode">Google AI 模式</option>
    </select></label>
    <details className={styles.context}><summary>已选择 {props.messages.length} 条消息</summary>
      {props.messages.map(message => <article key={message.id}><strong>{message.sender_name || '群成员'}</strong><pre>{messageText(message)}</pre></article>)}
    </details>
    {props.messages.some(m => m.attachments?.length) && <p className={styles.warning}>本次发送消息文字和附件说明，不包含附件原文件。</p>}
    <label className={styles.question}>想问什么<textarea rows={4} maxLength={2000} value={question} onChange={event => setQuestion(event.target.value)} /></label>
    {(error || unavailable) && <p className={styles.warning} role="alert">{error || unavailable}</p>}
  </SocialDialog>
}
