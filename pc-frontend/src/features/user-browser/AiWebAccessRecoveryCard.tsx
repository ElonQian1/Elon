import { AlertTriangle, ExternalLink, RefreshCw } from 'lucide-react'
import type { AiWebChatBackend } from './useAiWebChatBackend'
import styles from './AiWebAccessRecoveryCard.module.css'

export default function AiWebAccessRecoveryCard({ web }: { web: AiWebChatBackend }) {
  const phase = web.userState.phase
  const blocked = phase === 'login_required' || phase === 'access_limited'
  const promptRetained = Boolean(web.controller.loginRecoveryPrompt.trim())
  if (!blocked && !promptRetained) return null

  const limited = phase === 'access_limited'
  const busy = Boolean(web.controller.busyAction)
  const canRetry = promptRetained && web.userState.canNewConversation

  return (
    <aside
      className={styles.card}
      data-testid="ai-web-access-recovery"
      data-phase={phase}
      role="alert"
      aria-live="assertive"
    >
      <div className={styles.icon}><AlertTriangle size={20} aria-hidden="true" /></div>
      <div className={styles.body}>
        <span className={styles.eyebrow}>{limited ? '官方请求受限' : blocked ? '官方要求登录' : '等待重新发送'}</span>
        <strong>{blocked ? web.userState.title : '上一条问题尚未得到回答'}</strong>
        <p>{blocked
          ? web.userState.detail
          : '登录或官方页面恢复后，可在一个新对话里重新发送；只有点击重试才会再次提交。'}</p>
        {promptRetained && <p className={styles.retained}>上一条问题已安全保留在本机，空白助手占位已移除。</p>}
        <div className={styles.actions}>
          {canRetry && (
            <button type="button" className={styles.primary} onClick={() => void web.controller.retryLoginBlockedPrompt()} disabled={busy}>
              <RefreshCw size={14} aria-hidden="true" />新对话重试
            </button>
          )}
          {!promptRetained && web.userState.canNewConversation && (
            <button type="button" className={styles.primary} onClick={() => void web.controller.run('new_conversation')} disabled={busy}>
              <RefreshCw size={14} aria-hidden="true" />新建游客对话
            </button>
          )}
          <button type="button" onClick={() => void web.controller.openOfficial()} disabled={busy}>
            <ExternalLink size={14} aria-hidden="true" />显示官方页
          </button>
          {promptRetained && (
            <button type="button" onClick={web.controller.dismissLoginRecovery} disabled={busy}>暂不重试</button>
          )}
        </div>
      </div>
    </aside>
  )
}
