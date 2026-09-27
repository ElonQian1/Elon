import { useEffect, useState } from 'react'
import { RefreshCw, ExternalLink } from 'lucide-react'
import type { AiWebChatBackend } from './useAiWebChatBackend'
import styles from './AiWebConversationReadStatus.module.css'

export default function AiWebConversationReadStatus({ web }: { web: AiWebChatBackend }) {
  const target = web.controller.readTarget
  const [slow, setSlow] = useState(false)
  const [attempt, setAttempt] = useState(0)
  useEffect(() => {
    setSlow(false)
    const timer = window.setTimeout(() => setSlow(true), 8000)
    return () => window.clearTimeout(timer)
  }, [target, attempt])
  if (!target || web.messages.length) return null
  return <section className={styles.status} role="status" aria-live="polite">
    <p>{slow ? '尚未读取到会话内容' : '正在读取所选会话…'}</p>
    {slow && <div>
      <button type="button" disabled={Boolean(web.controller.busyAction)}
        onClick={() => { setSlow(false); setAttempt(value => value + 1); void web.controller.openCachedConversation(target) }}>
        <RefreshCw size={16} />重新读取
      </button>
      <button type="button" onClick={() => void web.controller.openOfficial()}><ExternalLink size={16} />查看官网</button>
    </div>}
  </section>
}
