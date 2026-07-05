import { ChevronLeft, ChevronRight } from 'lucide-react'
import styles from './AiChatPage.module.css'

export default function AiChatTopbar({
  title,
  userPanelCollapsed,
  modelButtonCopy,
  sending,
  onToggleUserPanel,
  onCodexVaultBackup,
}: {
  title: string
  userPanelCollapsed: boolean
  modelButtonCopy: { source: string; detail: string }
  sending: boolean
  onToggleUserPanel: () => void
  onCodexVaultBackup: () => void
}) {
  return (
    <header className={styles.topbar}>
      <span className={styles.topbarTitle}>{title}</span>
      <div className={styles.topbarRight}>
        <button
          className={[styles.topbarBtn, styles.panelToggleBtn].join(' ')}
          type="button"
          title={userPanelCollapsed ? '展开右侧用户栏' : '收起右侧用户栏'}
          aria-label={userPanelCollapsed ? '展开右侧用户栏' : '收起右侧用户栏'}
          aria-pressed={!userPanelCollapsed}
          onClick={onToggleUserPanel}
        >
          {userPanelCollapsed
            ? <ChevronLeft size={14} aria-hidden="true" />
            : <ChevronRight size={14} aria-hidden="true" />}
        </button>
        <span className={styles.modelBadge}>{modelButtonCopy.source} · {modelButtonCopy.detail}</span>
        <button className={styles.topbarBtn} type="button" title="分享这台电脑的算力" onClick={() => { window.location.href = '/pc/node' }}>
          分享算力
        </button>
        <button
          className={styles.topbarBtn}
          type="button"
          title="把本机 Codex auth.json 备份到云端保险箱"
          onClick={onCodexVaultBackup}
          disabled={sending}
        >
          备份 auth.json
        </button>
        <button className={styles.topbarBtn} type="button" title="打开移动端入口" onClick={() => window.open('/app/download', '_blank', 'noopener')}>
          打开移动端
        </button>
        <button className={styles.topbarBtn} type="button" title="切换到旧版" onClick={openLegacyPc}>
          旧版
        </button>
      </div>
    </header>
  )
}

function openLegacyPc() {
  try {
    const raw = localStorage.getItem('elon_auth')
    if (raw) {
      const tok = JSON.parse(raw)?.state?.token
      if (tok) {
        localStorage.setItem('lodex_token', tok)
        localStorage.setItem('elon_token', tok)
      }
    }
  } catch {}
  window.open('/pc-legacy', '_blank', 'noopener')
}
