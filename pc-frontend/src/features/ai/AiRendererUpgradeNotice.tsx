import { AlertTriangle, ExternalLink, RefreshCw } from 'lucide-react'
import type { LocalAiRendererCompatibilityNotice } from '../user-browser/localAiRendererCompatibility'
import styles from './AiChatPage.module.css'

export default function AiRendererUpgradeNotice({
  compatibility,
  onOpenOfficial,
  onCheckUpdates,
}: {
  compatibility?: LocalAiRendererCompatibilityNotice
  onOpenOfficial?: () => void
  onCheckUpdates?: () => void
}) {
  if (!compatibility) return null
  return (
    <aside className={styles.rendererUpgradeNotice} role="status">
      <AlertTriangle size={17} aria-hidden="true" />
      <div>
        <strong>官网富内容结构已变化</strong>
        <span>正文和已识别内容已保留；当前 Win 渲染器尚不能完整复现这部分卡片或交互内容。</span>
        <div>
          {onOpenOfficial && <button type="button" onClick={onOpenOfficial}><ExternalLink size={13} />查看官网完整内容</button>}
          {onCheckUpdates && <button type="button" onClick={onCheckUpdates}><RefreshCw size={13} />检查 Win 端更新</button>}
        </div>
      </div>
    </aside>
  )
}
