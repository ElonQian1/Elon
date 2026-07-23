import type {
  CrossPlatformTarget,
  CrossPlatformWritebackReceipt,
  PlatformWritebackResult,
} from './crossPlatformWritebackReceipt'
import styles from './CrossPlatformWritebackReceiptPanel.module.css'

interface Props {
  receipt: CrossPlatformWritebackReceipt | null
}

const PLATFORM_LABEL: Record<CrossPlatformTarget, string> = {
  pwa: 'PWA',
  apk: 'APK',
}

const STATUS_LABEL: Record<PlatformWritebackResult['status'], string> = {
  PREVIEW: 'preview',
  AI_WRITING: 'AI writing',
  SAVED: 'saved',
  BUILD_VERIFYING: 'build verifying',
  BUILD_VERIFIED: 'build-verified',
  FAILED: 'failed',
  EVIDENCE_MISSING: 'evidence missing',
}

export function CrossPlatformWritebackReceiptPanel({ receipt }: Props) {
  if (!receipt) {
    return (
      <section className={styles.empty} data-testid="cross-platform-writeback-receipt-empty">
        <strong>双端机器回执</strong>
        <span>尚未开始写回；当前两端均为草稿 preview。</span>
        <div className={styles.platformGrid}>
          {(['pwa', 'apk'] as const).map((platform) => (
            <article key={platform} className={styles.platform} data-platform={platform} data-platform-status="PREVIEW">
              <div><strong>{PLATFORM_LABEL[platform]}</strong><span className={styles.status}>preview</span></div>
              <small>等待写回 · 0 个文件</small>
              <span>{platform === 'pwa' ? '等待真实重载证据' : '等待真实 Renderer 证据'}</span>
            </article>
          ))}
        </div>
      </section>
    )
  }
  return (
    <section className={styles.panel} data-testid="cross-platform-writeback-receipt" data-receipt-status={receipt.status}>
      <header>
        <div>
          <strong>双端机器回执 · {receipt.status}</strong>
          <span>{receipt.complete ? 'PWA 与 APK 均已验证' : '缺证据的端不会显示完成'}</span>
        </div>
        <code title={receipt.receiptId}>{receipt.receiptId.slice(0, 18)}</code>
      </header>
      <div className={styles.platformGrid}>
        {receipt.targetPlatforms.map((platform) => (
          <PlatformReceiptCard key={platform} platform={platform} result={receipt.platformResults[platform]} />
        ))}
      </div>
      <dl className={styles.proof}>
        <div><dt>sourceRevision</dt><dd title={receipt.sourceRevision}>{shortHash(receipt.sourceRevision)}</dd></div>
        <div><dt>sourceHash</dt><dd title={receipt.sourceHash}>{shortHash(receipt.sourceHash)}</dd></div>
        <div><dt>changedFiles</dt><dd>{receipt.changedFiles.length}</dd></div>
        <div><dt>targetPlatforms</dt><dd>{receipt.targetPlatforms.join(' + ')}</dd></div>
      </dl>
      {receipt.changedFiles.length > 0 && (
        <details>
          <summary>查看 {receipt.changedFiles.length} 个源码文件与 SHA-256</summary>
          <ul>{receipt.changedFiles.map((file) => (
            <li key={file}><code>{file}</code><span>{shortHash(receipt.sourceHashes[file])}</span></li>
          ))}</ul>
        </details>
      )}
      {receipt.diagnostics.length > 0 && (
        <ul className={styles.diagnostics}>
          {receipt.diagnostics.map((diagnostic) => <li key={diagnostic}>{diagnostic}</li>)}
        </ul>
      )}
    </section>
  )
}

function PlatformReceiptCard({
  platform,
  result,
}: {
  platform: CrossPlatformTarget
  result: PlatformWritebackResult
}) {
  return (
    <article className={styles.platform} data-platform={platform} data-platform-status={result.status}>
      <div>
        <strong>{PLATFORM_LABEL[platform]}</strong>
        <span className={styles.status}>{STATUS_LABEL[result.status]}</span>
      </div>
      <small>{methodLabel(result.method)} · {result.changedFiles.length} 个文件</small>
      <span>{result.evidenceComplete ? '构建/运行证据完整' : platform === 'pwa' ? '等待真实重载证据' : '等待真实 Renderer 证据'}</span>
      {result.error && <p>{result.error}</p>}
    </article>
  )
}

function methodLabel(method: PlatformWritebackResult['method']) {
  if (method === 'DETERMINISTIC') return '确定性写回'
  if (method === 'CODEX') return 'Codex 结构写回'
  if (method === 'MIXED') return '确定性 + Codex'
  return '等待写回'
}

function shortHash(value?: string) {
  if (!value) return '—'
  return value.length > 22 ? `${value.slice(0, 18)}…` : value
}
