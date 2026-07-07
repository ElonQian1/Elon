import type { LucideIcon } from 'lucide-react'
import {
  Apple,
  CircleCheck,
  Download,
  ExternalLink,
  FileText,
  Globe2,
  Hash,
  Laptop,
  Monitor,
  PackageCheck,
  Rocket,
  Smartphone,
  Terminal,
  UsersRound,
  Wrench,
} from 'lucide-react'

import { formatTime } from '../../lib/utils'
import type { Channel, Project, ProjectLanding as ProjectLandingData, ProjectLandingDownload } from './types'
import styles from './ProjectLanding.module.css'

interface Props {
  project: Project
  channels: Channel[]
  landing: ProjectLandingData | null
  onSelectChannel: (id: string) => void
}

interface PrimaryAction {
  icon: LucideIcon
  title: string
  detail: string
  label: string
  disabled?: boolean
  onClick: () => void
}

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

export default function ProjectLanding({ project, channels, landing, onSelectChannel }: Props) {
  const devChannel = channels.find((channel) => channel.kind === 'ai_development')
  const buildChannel = channels.find((channel) => channel.kind === 'builds')
  const downloads = landing?.downloads ?? []
  const availableDownloads = downloads.filter((download) => isDownloadEnabled(download))
  const firstDownload = availableDownloads[0]
  const resources = landing?.resources?.filter((resource) => resource.url) ?? []
  const externalUrl = landing?.custom_landing_url || landing?.web_url || resources[0]?.url
  const tagline = landing?.tagline || project.description || '项目空间'
  const description = landing?.summary || landing?.description || project.description
  const updatedAt = project.updated_at ? formatTime(project.updated_at) : ''
  const primaryAction = buildPrimaryAction({
    devChannel,
    buildChannel,
    firstDownload,
    externalUrl,
    onSelectChannel,
  })

  return (
    <div className={styles.landing}>
      <section className={styles.heroBand}>
        <div className={styles.identity}>
          <ProjectIcon project={project} />
          <div className={styles.identityCopy}>
            <div className={styles.kicker}>
              <CircleCheck size={14} aria-hidden="true" />
              <span>{landing?.source?.status === 'fallback' ? '默认项目主页' : '项目空间'}</span>
              {updatedAt && <span>最近更新 {updatedAt}</span>}
            </div>
            <h2>{project.name}</h2>
            <p>{tagline}</p>
            {description && description !== tagline && <span className={styles.summary}>{description}</span>}
            <div className={styles.metaRow}>
              <MetaPill icon={UsersRound} label={project.member_count ? `${project.member_count} 位成员` : '项目成员'} />
              <MetaPill icon={Hash} label={`${channels.length} 个频道`} />
              <MetaPill icon={Download} label={availableDownloads.length ? `${availableDownloads.length} 个可用入口` : '等待交付'} />
            </div>
          </div>
        </div>

        <div className={styles.actionPanel}>
          <button
            className={styles.primaryAction}
            type="button"
            disabled={primaryAction.disabled}
            onClick={primaryAction.onClick}
          >
            <span className={styles.primaryIcon}>
              <primaryAction.icon size={22} aria-hidden="true" />
            </span>
            <span className={styles.primaryCopy}>
              <strong>{primaryAction.title}</strong>
              <small>{primaryAction.detail}</small>
            </span>
            <em>{primaryAction.label}</em>
          </button>
        </div>
      </section>

      {downloads.length > 0 && (
        <section id="landing-downloads" className={styles.downloadSection}>
          <SectionHeader icon={Download} title="下载安装" note={availableDownloads.length ? '可用入口优先显示' : '等待发布'} />
          <div className={styles.downloadGrid}>
            {downloads.map((download, index) => (
              <DownloadCard key={`${download.platform ?? 'download'}-${index}`} download={download} />
            ))}
          </div>
        </section>
      )}
    </div>
  )
}

function ProjectIcon({ project }: { project: Project }) {
  if (project.icon_data_url || project.icon) {
    return <img src={project.icon_data_url || project.icon} alt="" className={styles.projectIcon} />
  }
  return <div className={styles.projectIconFallback}>{(project.name?.[0] ?? '项').toUpperCase()}</div>
}

function MetaPill({ icon: Icon, label }: { icon: LucideIcon; label: string }) {
  return (
    <span className={styles.metaPill}>
      <Icon size={14} aria-hidden="true" />
      {label}
    </span>
  )
}

function SectionHeader({ icon: Icon, title, note }: { icon: LucideIcon; title: string; note?: string }) {
  return (
    <div className={styles.sectionHeader}>
      <Icon size={16} aria-hidden="true" />
      <strong>{title}</strong>
      {note && <span>{note}</span>}
    </div>
  )
}

function DownloadCard({ download }: { download: ProjectLandingDownload }) {
  const enabled = isDownloadEnabled(download)
  const platform = normalizePlatform(download.platform)
  const meta = PLATFORM_META[platform] ?? { label: download.platform || '通用下载', short: download.short || 'Pkg', icon: FileText }
  const Icon = meta.icon
  const status = normalizeStatus(download.status, download)
  const variantCount = download.variants?.filter((variant) => isVariantEnabled(variant)).length ?? 0

  return (
    <button
      className={[
        styles.downloadCard,
        styles[`status_${statusClass(status)}`] ?? '',
        enabled ? '' : styles.downloadDisabled,
      ].join(' ')}
      type="button"
      disabled={!enabled}
      onClick={() => openDownload(download)}
    >
      <span className={styles.platformBadge}>
        <Icon size={18} aria-hidden="true" />
      </span>
      <span className={styles.downloadCopy}>
        <strong>{download.label || meta.label}</strong>
        <small>{[download.version, downloadSizeLabel(download)].filter(Boolean).join(' / ') || download.short || meta.short}</small>
        {download.note && <em>{download.note}</em>}
      </span>
      <span className={styles.downloadStatus}>{statusLabel(status, enabled)}</span>
      {variantCount > 0 && <span className={styles.variantHint}>{variantCount} 个版本</span>}
    </button>
  )
}

function buildPrimaryAction({
  devChannel,
  buildChannel,
  firstDownload,
  externalUrl,
  onSelectChannel,
}: {
  devChannel?: Channel
  buildChannel?: Channel
  firstDownload?: ProjectLandingDownload
  externalUrl?: string
  onSelectChannel: (id: string) => void
}): PrimaryAction {
  if (devChannel) {
    return {
      icon: Rocket,
      title: '继续开发',
      detail: devChannel.description || '进入 AI 开发频道',
      label: devChannel.name,
      onClick: () => onSelectChannel(devChannel.id),
    }
  }
  if (buildChannel) {
    return {
      icon: PackageCheck,
      title: '查看交付',
      detail: buildChannel.description || '进入构建与安装包频道',
      label: buildChannel.name,
      onClick: () => onSelectChannel(buildChannel.id),
    }
  }
  if (firstDownload) {
    return {
      icon: Download,
      title: '安装使用',
      detail: downloadLabel(firstDownload),
      label: '下载',
      onClick: () => openDownload(firstDownload),
    }
  }
  if (externalUrl) {
    return {
      icon: ExternalLink,
      title: '打开项目',
      detail: '查看项目主页或外部入口',
      label: '打开',
      onClick: () => openUrl(externalUrl),
    }
  }
  return {
    icon: Wrench,
    title: '等待配置',
    detail: '项目入口会在频道或交付配置后出现',
    label: '未就绪',
    disabled: true,
    onClick: () => undefined,
  }
}

function normalizePlatform(platform?: string) {
  const raw = String(platform ?? '').toLowerCase().replace(/[\s_-]+/g, '')
  if (raw === 'apk' || raw === 'androidapk') return 'android'
  if (raw === 'win' || raw === 'windowsclient') return 'windows'
  if (raw === 'mac' || raw === 'osx' || raw === 'darwin') return 'macos'
  if (raw === 'browser' || raw === 'h5' || raw === 'website') return 'web'
  return raw
}

function normalizeStatus(status: string | undefined, download: ProjectLandingDownload) {
  if (status) return status.toLowerCase()
  return downloadUrl(download) ? 'available' : 'planned'
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

function downloadLabel(download: ProjectLandingDownload) {
  const platform = normalizePlatform(download.platform)
  const meta = PLATFORM_META[platform]
  return download.label || meta?.label || download.platform || '下载项目'
}

function downloadSizeLabel(download: ProjectLandingDownload) {
  if (download.size) return download.size
  if (download.size_label) return download.size_label
  if (download.sizeLabel) return download.sizeLabel
  return formatDownloadBytes(download.size_bytes ?? download.sizeBytes)
}

function formatDownloadBytes(value: string | number | undefined) {
  const bytes = typeof value === 'number' ? value : Number(value)
  if (!Number.isFinite(bytes) || bytes <= 0) return ''
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${bytes} B`
}

function isDownloadEnabled(download: ProjectLandingDownload) {
  const status = normalizeStatus(download.status, download)
  return ACTIVE_STATUSES.has(status) || (!!downloadUrl(download) && !PASSIVE_STATUSES.has(status))
}

function isVariantEnabled(variant: NonNullable<ProjectLandingDownload['variants']>[number]) {
  const status = String(variant.status ?? '').toLowerCase()
  return ACTIVE_STATUSES.has(status) || (!!variant.url && !PASSIVE_STATUSES.has(status))
}

function downloadUrl(download: ProjectLandingDownload) {
  if (download.url) return download.url
  return download.variants?.find((variant) => isVariantEnabled(variant) && variant.url)?.url
}

function openDownload(download: ProjectLandingDownload) {
  const url = downloadUrl(download)
  if (url) openUrl(url)
}

function openUrl(url: string) {
  window.open(url, '_blank', 'noopener')
}
