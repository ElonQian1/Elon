import type { AiWebChatBackend } from './useAiWebChatBackend'
import styles from './AiWebSendFailure.module.css'

export default function AiWebSendFailure({ web }: { web: AiWebChatBackend }) {
  const diagnostics = web.controller.sessionState?.diagnostics
  if (web.controller.busyAction || diagnostics?.lastCommandAction !== 'send_prompt'
    || diagnostics.lastCommandOk !== false || !web.message) return null
  const unsupported = web.message.includes('runtime_not_observed')
  return <div className={styles.error} role="alert">
    <span>{unsupported
      ? '当前 ChatGPT 官网版本尚未兼容，未确认发送。请检查保留的草稿，或打开官网继续。'
      : web.message}</span>
    <button type="button" onClick={() => void web.controller.openOfficial()}>打开官网</button>
  </div>
}
