import { useEffect, useRef, useState } from 'react'
import { useAuthStore } from '../../store/auth'
import type { ActiveConversation } from './socialMessageTypes'
import type { SavedSocialMessage } from './socialLocalState'
import { messageText, sendSocialMessage, SocialRequestError } from './socialChatOperations'
import { conversationId } from './socialChatCache'
import SocialDialog from './SocialDialog'
import styles from './SocialTools.module.css'

export interface SocialTarget extends ActiveConversation { title: string }
export default function SocialForwardDialog({ messages, targets, onClose, onSent }: {
  messages: SavedSocialMessage[]; targets: SocialTarget[]; onClose: () => void; onSent: () => void
}) {
  const [targetId, setTargetId] = useState('')
  const [done, setDone] = useState<string[]>([])
  const [busy, setBusy] = useState(false)
  const [started, setStarted] = useState(false)
  const [error, setError] = useState('')
  const [uncertain, setUncertain] = useState(false)
  const alive = useRef(true)
  const controller = useRef<AbortController | null>(null)
  useEffect(() => { alive.current = true; return () => { alive.current = false; controller.current?.abort() } }, [])
  const target = targets.find(t => conversationId(t) === targetId)
  const remaining = messages.filter(item => !done.includes(item.message.id))
  async function forward() {
    if (busy || !target || uncertain || !remaining.length) return
    const session = useAuthStore.getState().token
    controller.current = new AbortController()
    setBusy(true); setStarted(true); setError('')
    for (const item of remaining) {
      if (!alive.current || useAuthStore.getState().token !== session) break
      try {
        const content = item.message.content ? `转发自 ${item.message.sender_name || item.title}\n${item.message.content}` : ''
        if (Array.from(content).length > 4000) throw new Error('转发文字超过 4000 字，请复制后精简再发送')
        const result = await sendSocialMessage(target, { content, attachments: item.message.attachments }, controller.current.signal)
        if (!alive.current) break
        if (!result.message?.id) throw new SocialRequestError('该条发送结果未确认，请核对目标会话', true)
        setDone(old => [...old, item.message.id])
      } catch (failure) {
        if (!alive.current) break
        setError((failure as Error).message)
        setUncertain(failure instanceof SocialRequestError && failure.uncertain)
        break
      }
    }
    if (alive.current) { setBusy(false); onSent() }
  }
  return <SocialDialog title="转发消息" onClose={onClose} busy={busy} footer={<>
    <span>{done.length} / {messages.length} 条已确认发送</span>
    <button type="button" disabled={busy || !target || uncertain || !remaining.length} onClick={() => void forward()}>{busy ? '正在转发…' : done.length ? `继续发送剩余 ${remaining.length} 条` : `确认转发 ${messages.length} 条`}</button>
  </>}>
    <label>发送到<select aria-label="转发目标" value={targetId} onChange={event => setTargetId(event.target.value)} disabled={started}>
      <option value="">请选择好友或群聊</option>{targets.map(t => <option key={conversationId(t)} value={conversationId(t)}>{t.kind === 'group' ? '群聊' : '好友'} · {t.title}</option>)}
    </select></label>
    <p className={styles.hint}>确认后会以你的账号发送新消息。成功的条目不会再次发送；转发后不会随原消息编辑而变化。</p>
    {messages.map(item => <article className={styles.item} key={item.message.id}><strong>{item.title} · {item.message.sender_name || '原发送者'}</strong><pre>{messageText(item.message)}</pre><small>{done.includes(item.message.id) ? '已发送' : '待发送'}</small></article>)}
    {error && <p className={styles.error} role="alert">{error}</p>}
    {uncertain && <button type="button" onClick={() => { setUncertain(false); setError('已按你的核对结果允许手动重试') }}>我已核对：该条未送达，允许重试</button>}
  </SocialDialog>
}
