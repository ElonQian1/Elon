import { AlertTriangle, Download, ExternalLink, RefreshCw } from 'lucide-react'
import { WIN_CLIENT_DOWNLOAD_URL } from '../node/launchWinClient'
import { localAiResearchCompatibilityNotice } from '../user-browser/localAiResearchCompatibilityNotice'
import type { AiWebChatBackend } from '../user-browser/useAiWebChatBackend'
import styles from './AiWebClientUpgradeNotice.module.css'

export default function AiWebClientUpgradeNotice({ web }: { web: AiWebChatBackend }) {
  const clientUpgrade = web.capability.state === 'upgrade_required'
  const researchNotice = localAiResearchCompatibilityNotice(web.researchStatus)
  if (!clientUpgrade && !researchNotice) return null

  return (
    <section className={styles.notice} role="status" aria-live="polite" data-testid="ai-web-client-upgrade-notice">
      <AlertTriangle className={styles.icon} size={20} aria-hidden="true" />
      <div className={styles.copy}>
        <strong>{clientUpgrade ? '当前 Win 客户端较旧，已暂停网页 AI 操作' : researchNotice!.title}</strong>
        <span>
          {clientUpgrade
            ? web.capability.message || '官网适配器已经升级，请先更新 Win 客户端，避免新会话卡住或富内容显示不完整。'
            : researchNotice!.detail}
        </span>
      </div>
      <div className={styles.actions}>
        {!clientUpgrade && web.ready && (
          <button type="button" onClick={() => void web.controller.openOfficial()}>
            <ExternalLink size={15} aria-hidden="true" />查看官网完整内容
          </button>
        )}
        <a href={WIN_CLIENT_DOWNLOAD_URL} download>
          <Download size={15} aria-hidden="true" />下载新版
        </a>
        {clientUpgrade && (
          <button type="button" onClick={() => void web.capability.refresh()}>
            <RefreshCw size={15} aria-hidden="true" />更新后重检
          </button>
        )}
      </div>
    </section>
  )
}
