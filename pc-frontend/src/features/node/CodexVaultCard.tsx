import { DownloadCloud, ShieldCheck, Trash2, UploadCloud } from 'lucide-react'
import type { CodexVaultLocalStatus, CodexVaultStatusResponse } from './types'
import styles from './NodePage.module.css'

function formatVaultTime(value?: string | null): string {
  if (!value) return '无'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return date.toLocaleString()
}

function authStateText(auth?: CodexVaultLocalStatus['default_auth']): string {
  if (!auth?.present) return '未找到'
  if (auth.problem) return '读取异常'
  if (auth.auth_mode && auth.auth_mode !== 'chatgpt') return auth.auth_mode
  return auth.has_refresh_token ? 'ChatGPT / Pro' : '缺少 refresh_token'
}

export default function CodexVaultCard({
  status,
  cloud,
  busy,
  onBackup,
  onRestore,
  onClear,
  onDeleteCloud,
  onRefresh,
}: {
  status: CodexVaultLocalStatus | null
  cloud?: CodexVaultStatusResponse['cloud']
  busy: boolean
  onBackup: () => void
  onRestore: () => void
  onClear: () => void
  onDeleteCloud: () => void
  onRefresh: () => void
}) {
  const vault = cloud?.vault
  const cloudReady = !!vault?.configured
  const bound = !!vault?.bound
  const defaultAuth = status?.default_auth
  const managedAuth = status?.managed_auth
  const activeManaged = !!status?.active_home_managed
  const canBackup = !!defaultAuth?.present
    && defaultAuth.auth_mode !== 'api_key'
    && !!defaultAuth.has_refresh_token
  const canRestore = cloudReady && bound
  const canClear = activeManaged || !!managedAuth?.present
  const state = cloud?.error
    ? '云端不可用'
    : !cloud
      ? '读取中'
      : !cloudReady
        ? '服务器未配置'
        : bound
          ? `已备份 v${vault?.credential_version ?? 1}`
          : '未备份'
  const stateTone = cloudReady && bound ? styles.vaultOnline : cloud?.error ? styles.vaultOffline : styles.vaultChecking
  return (
    <section className={styles.vaultCard}>
      <div className={styles.vaultHead}>
        <div>
          <span className={styles.codexLabel}>Codex Pro 保险箱</span>
          <h4>临时登录到陌生电脑</h4>
        </div>
        <span className={[styles.vaultState, stateTone].join(' ')}>{state}</span>
      </div>
      <div className={styles.vaultGrid}>
        <div>
          <span>默认 auth.json</span>
          <strong>{authStateText(defaultAuth)}</strong>
        </div>
        <div>
          <span>托管 CODEX_HOME</span>
          <strong>{activeManaged ? '当前生效' : managedAuth?.present ? '已写入' : '未写入'}</strong>
        </div>
        <div>
          <span>最近备份</span>
          <strong>{formatVaultTime(vault?.last_backup_at)}</strong>
        </div>
        <div>
          <span>最近租用</span>
          <strong>{formatVaultTime(vault?.last_lease_at)}</strong>
        </div>
      </div>
      {cloud?.error && <p className={styles.codexFixHint}>{cloud.error}</p>}
      {defaultAuth?.problem && <p className={styles.codexFixHint}>{defaultAuth.problem}</p>}
      {status?.managed_home && <code className={styles.codexPath}>{status.managed_home}</code>}
      <div className={styles.vaultActions}>
        <button
          className={[styles.btn, styles.primary, styles.iconBtn].join(' ')}
          onClick={onBackup}
          disabled={busy || !canBackup}
          title="把本机默认 Codex Pro 登录态加密备份到云端保险箱"
        >
          <UploadCloud size={15} strokeWidth={2.2} aria-hidden="true" />
          备份本机登录
        </button>
        <button
          className={[styles.btn, styles.iconBtn].join(' ')}
          onClick={onRestore}
          disabled={busy || !canRestore}
          title="把云端保险箱凭据写入本机节点托管的临时 CODEX_HOME"
        >
          <DownloadCloud size={15} strokeWidth={2.2} aria-hidden="true" />
          临时恢复
        </button>
        <button
          className={[styles.btn, styles.iconBtn].join(' ')}
          onClick={onClear}
          disabled={busy || !canClear}
          title="删除本机节点托管的临时 CODEX_HOME，不影响默认 auth.json"
        >
          <Trash2 size={15} strokeWidth={2.2} aria-hidden="true" />
          清理本机
        </button>
        <button
          className={[styles.btn, styles.iconBtn].join(' ')}
          onClick={onDeleteCloud}
          disabled={busy || !bound}
          title="删除云端保险箱中的 Codex Pro 备份，不影响本机文件"
        >
          <Trash2 size={15} strokeWidth={2.2} aria-hidden="true" />
          删除云端
        </button>
        <button
          className={[styles.btn, styles.iconBtn].join(' ')}
          onClick={onRefresh}
          disabled={busy}
          title="刷新 Codex Pro 保险箱状态"
        >
          <ShieldCheck size={15} strokeWidth={2.2} aria-hidden="true" />
          刷新
        </button>
      </div>
    </section>
  )
}
