import { Bot, ShieldCheck, Stethoscope } from 'lucide-react'
import styles from './AiChatPage.module.css'

export default function AiPinnedTools({
  sending,
  onNewConversation,
  onOpenDoctor,
  onCodexVaultBackup,
}: {
  sending: boolean
  onNewConversation: () => void
  onOpenDoctor: () => void
  onCodexVaultBackup: () => void
}) {
  return (
    <div className={styles.pinnedTools}>
      <button className={[styles.pinnedTool, styles.pinnedToolPrimary].join(' ')} type="button" onClick={onNewConversation}>
        <span className={styles.pinnedToolIcon}>
          <Bot aria-hidden="true" size={18} strokeWidth={2.2} />
        </span>
        <span className={styles.pinnedToolCopy}>
          <strong>一龙 AI 对话</strong>
          <em>开始新的 AI 聊天</em>
        </span>
      </button>
      <button className={styles.pinnedTool} type="button" onClick={onOpenDoctor}>
        <span className={styles.pinnedToolIcon}>
          <Stethoscope aria-hidden="true" size={18} strokeWidth={2.2} />
        </span>
        <span className={styles.pinnedToolCopy}>
          <strong>电脑医生</strong>
          <em>诊断和修复本机问题</em>
        </span>
      </button>
      <button className={styles.pinnedTool} type="button" onClick={onCodexVaultBackup} disabled={sending}>
        <span className={styles.pinnedToolIcon}>
          <ShieldCheck aria-hidden="true" size={18} strokeWidth={2.2} />
        </span>
        <span className={styles.pinnedToolCopy}>
          <strong>Codex 账号</strong>
          <em>保存并分享 Codex 账号</em>
        </span>
      </button>
    </div>
  )
}
