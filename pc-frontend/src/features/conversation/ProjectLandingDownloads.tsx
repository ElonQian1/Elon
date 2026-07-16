import type { LucideIcon } from 'lucide-react'
import { Apple, Download, FileText, Globe2, Laptop, Monitor, Smartphone, Terminal } from 'lucide-react'
import type { ProjectLandingDownload } from './types'
import styles from './ProjectLandingDownloads.module.css'

const ACTIVE_STATUSES = new Set(['available', 'external', 'partial'])
const PASSIVE_STATUSES = new Set(['coming_soon', 'planned', 'pending', 'needs_configuration', 'not_deployed'])

const PLATFORM_META: Record<string, { label: string; short: string; icon: LucideIcon }> = {
  android: { label: 'Android APK', short: 'APK', icon: Smartphone },
  windows: { label: 'Windows 客户端', short: 'Win', icon: Monitor },
  web: { label: '网页端', short: 'Web', icon: Globe2 },
  ios: { label: 'iOS / PWA', short: 'iOS', icon: Apple },
  macos: { label: 'macOS', short: 'Mac', icon: Laptop },
  linux: { label: 'Linux', short: 'Linux', icon: Terminal },
}

export default function ProjectLandingDownloads({ downloads }: { downloads: ProjectLandingDownload[] }) {
  const availableCount = downloads.filter(isLandingDownloadEnabled).length

  return (
    <section id="project-landing-downloads" className={styles.downloadSection}>
      <div className={styles.sectionHeader}>
        <span className={styles.panelHeaderIcon}><Download size={17} aria-hidden="true" /></span>
        <div>
          <strong>下载与接入</strong>
          <small>{availableCount ? `${availableCount} 个入口现在可用` : '安装包生成后会自动出现在这里'}</small>
        </div>
        {downloads.length > 0 && <em>可用入口优先显示</em>}
      </div>

      {downloads.length === 0 ? (
        <div className={styles.downloadEmpty}>
          <Download size={22} aria-hidden="true" />
          <strong>还没有安装包</strong>
          <span>完成开发并生成安装包后，这里会展示手机端、PC 端和网页入口。</span>
        </div>
      ) : (
        <div className={styles.downloadGrid}>
          {[...downloads]
            .sort((left, right) => Number(isLandingDownloadEnabled(right)) - Number(isLandingDownloadEnabled(left)))
            .map((download, index) => (
              <DownloadCard key={`${download.platform ?? 'download'}-${index}`} download={download} />
            ))}
        </div>
      )}
    </section>
  )
}

function DownloadCard({ download }: { download: ProjectLandingDownload }) {
  const platform = normalizePlatform(download.platform)
  const meta = PLATFORM_META[platform] ?? {
    label: download.platform || '通用下载',
    short: download.short || 'Pkg',
    icon: FileText,
  }
  const Icon = meta.icon
  const status = normalizeStatus(download.status, download.url)
  const variants = download.variants ?? []
  const enabled = isLandingDownloadEnabled(download)

  if (variants.length > 0) {
    return (
      <article className={[styles.variantCard, styles[`status_${statusClass(status)}`] ?? ''].join(' ')}>
        <div className={styles.variantHeader}>
          <span className={styles.platformBadge}><Icon size={19} aria-hidden="true" /></span>
          <span className={styles.downloadCopy}>
            <strong>{download.label || meta.label}</strong>
            <small>{[download.version, downloadSizeLabel(download)].filter(Boolean).join(' · ') || download.short || meta.short}</small>
            {download.note && <em>{download.note}</em>}
          </span>
          <span className={styles.downloadStatus}>{statusLabel(status, enabled)}</span>
        </div>
        <div className={styles.variantList}>
          {variants.map((variant, index) => {
            const variantStatus = normalizeStatus(variant.status, variant.url)
            const variantEnabled = isVariantEnabled(variant)
            return (
              <div className={styles.variantRow} key={`${variant.label ?? variant.arch ?? 'variant'}-${index}`}>
                <span className={styles.variantCopy}>
                  <strong>{variant.label || variant.arch || `版本 ${index + 1}`}</strong>
                  <small>{[variant.arch, variant.version, variantSizeLabel(variant)].filter(Boolean).join(' · ') || statusLabel(variantStatus, variantEnabled)}</small>
                  {variant.note && <em>{variant.note}</em>}
                </span>
                <button
                  className={styles.variantAction}
                  type="button"
                  disabled={!variantEnabled}
                  onClick={() => variant.url && openUrl(variant.url)}
                >
                  {variantEnabled ? '下载' : statusLabel(variantStatus, false)}
                </button>
              </div>
            )
          })}
        </div>
      </article>
    )
  }

  return (
    <button
      className={[
        styles.downloadCard,
        styles[`status_${statusClass(status)}`] ?? '',
        enabled ? '' : styles.downloadDisabled,
      ].join(' ')}
      type="button"
      disabled={!enabled}
      onClick={() => download.url && openUrl(download.url)}
    >
      <span className={styles.platformBadge}><Icon size={19} aria-hidden="true" /></span>
      <span className={styles.downloadCopy}>
        <strong>{download.label || meta.label}</strong>
        <small>{[download.version, downloadSizeLabel(download)].filter(Boolean).join(' · ') || download.short || meta.short}</small>
        {download.note && <em>{download.note}</em>}
      </span>
      <span className={styles.downloadStatus}>{statusLabel(status, enabled)}</span>
    </button>
  )
}

export function isLandingDownloadEnabled(download: ProjectLandingDownload) {
  const status = normalizeStatus(download.status, download.url)
  if (download.url && (ACTIVE_STATUSES.has(status) || !PASSIVE_STATUSES.has(status))) return true
  return !!landingDownloadUrl(download)
}

export function firstLandingDownload(downloads: ProjectLandingDownload[]) {
  return downloads.find(isLandingDownloadEnabled)
}

export function landingDownloadUrl(download: ProjectLandingDownload) {
  const status = normalizeStatus(download.status, download.url)
  if (download.url && (ACTIVE_STATUSES.has(status) || !PASSIVE_STATUSES.has(status))) return download.url
  return download.variants?.find(isVariantEnabled)?.url || ''
}

function isVariantEnabled(variant: NonNullable<ProjectLandingDownload['variants']>[number]) {
  const status = normalizeStatus(variant.status, variant.url)
  return !!variant.url && (ACTIVE_STATUSES.has(status) || !PASSIVE_STATUSES.has(status))
}

function normalizePlatform(platform?: string) {
  const raw = String(platform ?? '').toLowerCase().replace(/[\s_-]+/g, '')
  if (raw === 'apk' || raw === 'androidapk') return 'android'
  if (raw === 'win' || raw === 'windowsclient') return 'windows'
  if (raw === 'mac' || raw === 'osx' || raw === 'darwin') return 'macos'
  if (raw === 'browser' || raw === 'h5' || raw === 'website') return 'web'
  return raw
}

function normalizeStatus(status: string | undefined, url: string | undefined) {
  if (status) return status.toLowerCase()
  return url ? 'available' : 'planned'
}

function statusLabel(status: string, enabled: boolean) {
  if (enabled) return status === 'external' ? '外部入口' : '可用'
  if (status === 'coming_soon') return '即将支持'
  if (status === 'planned') return '计划中'
  if (status === 'unavailable') return '暂不可用'
  if (PASSIVE_STATUSES.has(status)) return '待发布'
  return '待配置'
}

function statusClass(status: string) {
  return status.replace(/[^a-z0-9]+/g, '_')
}

function downloadSizeLabel(download: ProjectLandingDownload) {
  return download.size || download.size_label || download.sizeLabel || formatDownloadBytes(download.size_bytes ?? download.sizeBytes)
}

function variantSizeLabel(variant: NonNullable<ProjectLandingDownload['variants']>[number]) {
  return variant.size || variant.size_label || variant.sizeLabel || formatDownloadBytes(variant.size_bytes ?? variant.sizeBytes)
}

function formatDownloadBytes(value: string | number | undefined) {
  const bytes = typeof value === 'number' ? value : Number(value)
  if (!Number.isFinite(bytes) || bytes <= 0) return ''
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${bytes} B`
}

function openUrl(url: string) {
  window.open(url, '_blank', 'noopener')
}
