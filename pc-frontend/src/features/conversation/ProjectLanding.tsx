import type { LucideIcon } from 'lucide-react'
import {
  Activity,
  Apple,
  Bot,
  ChevronRight,
  CircleCheck,
  Download,
  ExternalLink,
  FileText,
  Globe2,
  Hash,
  Laptop,
  Link2,
  Megaphone,
  MessageSquare,
  Monitor,
  PackageCheck,
  Rocket,
  Smartphone,
  Sparkles,
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
  const announcementChannel = channels.find((channel) => channel.kind === 'announce')
  const chatChannels = channels.filter((channel) => !['ai_development', 'builds', 'announce'].includes(channel.kind ?? ''))
  const downloads = landing?.downloads ?? []
  const availableDownloads = downloads.filter((download) => isDownloadEnabled(download))
  const firstDownload = availableDownloads[0]
  const resources = landing?.resources?.filter((resource) => resource.url) ?? []
  const externalUrl = landing?.custom_landing_url || landing?.web_url || resources[0]?.url
  const tagline = landing?.tagline || project.description || '项目空间'
  const description = landing?.summary || landing?.description || project.description
  const updatedAt = project.updated_at ? formatTime(project.updated_at) : ''
  const quickActions = buildQuickActions({
    firstDownload,
    externalUrl,
    buildChannel,
    resources,
    onSelectChannel,
  })
  const primaryAction = buildPrimaryAction({
    devChannel,
    buildChannel,
    firstDownload,
    externalUrl,
    onSelectChannel,
  })
  const highlightTiles = buildHighlightTiles(landing, project, availableDownloads.length, channels.length)
  const activityItems = buildActivityItems(project, updatedAt, channels.length, availableDownloads.length)

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

          {quickActions.length > 0 && (
            <div className={styles.quickActions}>
              {quickActions.map((action) => (
                <button key={action.key} className={styles.quickAction} type="button" onClick={action.onClick}>
                  <action.icon size={15} aria-hidden="true" />
                  <span>{action.label}</span>
                </button>
              ))}
            </div>
          )}
        </div>
      </section>

      <section className={styles.startRail} aria-label="项目流程">
        <StepButton
          icon={Bot}
          title="需求开发"
          detail={devChannel ? devChannel.name : '待配置 AI 频道'}
          active={!!devChannel}
          disabled={!devChannel}
          onClick={() => devChannel && onSelectChannel(devChannel.id)}
        />
        <StepButton
          icon={PackageCheck}
          title="构建交付"
          detail={buildChannel ? buildChannel.name : '交付频道待配置'}
          active={!devChannel && !!buildChannel}
          disabled={!buildChannel}
          onClick={() => buildChannel && onSelectChannel(buildChannel.id)}
        />
        <StepButton
          icon={Download}
          title="安装使用"
          detail={firstDownload ? downloadLabel(firstDownload) : '生成后显示下载'}
          active={!devChannel && !buildChannel && !!firstDownload}
          disabled={!firstDownload}
          onClick={() => firstDownload && openDownload(firstDownload)}
        />
      </section>

      <div className={styles.contentGrid}>
        <main className={styles.mainStack}>
          {downloads.length > 0 && (
            <section id="landing-downloads" className={styles.section}>
              <SectionHeader icon={Download} title="下载安装" note={availableDownloads.length ? '可用入口优先显示' : '等待发布'} />
              <div className={styles.downloadGrid}>
                {downloads.map((download, index) => (
                  <DownloadCard key={`${download.platform ?? 'download'}-${index}`} download={download} />
                ))}
              </div>
            </section>
          )}

          <section className={styles.section}>
            <SectionHeader icon={Sparkles} title="项目预览" note="项目能力与适用场景" />
            <div className={styles.previewGrid}>
              {highlightTiles.map((tile) => (
                <div key={tile.title} className={styles.previewTile}>
                  <tile.icon size={18} aria-hidden="true" />
                  <strong>{tile.title}</strong>
                  <span>{tile.detail}</span>
                </div>
              ))}
            </div>
          </section>
        </main>

        <aside className={styles.sideStack}>
          {(channels.length > 0 || resources.length > 0 || externalUrl) && (
            <section className={styles.sideSection}>
              <SectionHeader icon={Hash} title="项目入口" />
              <div className={styles.entryList}>
                {channels.map((channel) => (
                  <button
                    key={channel.id}
                    className={styles.entryRow}
                    type="button"
                    onClick={() => onSelectChannel(channel.id)}
                  >
                    <span className={styles.entryIcon}>
                      {channelIcon(channel.kind)}
                    </span>
                    <span className={styles.entryCopy}>
                      <strong>{channel.name}</strong>
                      {channel.description && <small>{channel.description}</small>}
                    </span>
                    <ChevronRight size={14} aria-hidden="true" />
                  </button>
                ))}
                {externalUrl && (
                  <button className={styles.entryRow} type="button" onClick={() => openUrl(externalUrl)}>
                    <span className={styles.entryIcon}><ExternalLink size={14} aria-hidden="true" /></span>
                    <span className={styles.entryCopy}>
                      <strong>完整介绍</strong>
                      <small>打开项目主页或外部页面</small>
                    </span>
                    <ChevronRight size={14} aria-hidden="true" />
                  </button>
                )}
                {resources.map((resource) => (
                  <button
                    key={resource.url}
                    className={styles.entryRow}
                    type="button"
                    onClick={() => resource.url && openUrl(resource.url)}
                  >
                    <span className={styles.entryIcon}><Link2 size={14} aria-hidden="true" /></span>
                    <span className={styles.entryCopy}>
                      <strong>{resource.label || '相关链接'}</strong>
                      <small>{resource.url}</small>
                    </span>
                    <ChevronRight size={14} aria-hidden="true" />
                  </button>
                ))}
              </div>
            </section>
          )}

          <section className={styles.sideSection}>
            <SectionHeader icon={Activity} title="最近动态" />
            <div className={styles.activityList}>
              {announcementChannel && (
                <button className={styles.activityItem} type="button" onClick={() => onSelectChannel(announcementChannel.id)}>
                  <Megaphone size={15} aria-hidden="true" />
                  <span>查看 {announcementChannel.name}</span>
                </button>
              )}
              {activityItems.map((item) => (
                <div key={item} className={styles.activityItem}>
                  <Activity size={15} aria-hidden="true" />
                  <span>{item}</span>
                </div>
              ))}
              {chatChannels.slice(0, 2).map((channel) => (
                <button key={channel.id} className={styles.activityItem} type="button" onClick={() => onSelectChannel(channel.id)}>
                  <MessageSquare size={15} aria-hidden="true" />
                  <span>{channel.name}</span>
                </button>
              ))}
            </div>
          </section>
        </aside>
      </div>
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

function StepButton({
  icon: Icon,
  title,
  detail,
  active,
  disabled,
  onClick,
}: {
  icon: LucideIcon
  title: string
  detail: string
  active?: boolean
  disabled?: boolean
  onClick: () => void
}) {
  return (
    <button
      className={[styles.stepButton, active ? styles.stepActive : '', disabled ? styles.stepDisabled : ''].join(' ')}
      type="button"
      disabled={disabled}
      onClick={onClick}
    >
      <Icon size={18} aria-hidden="true" />
      <span>
        <strong>{title}</strong>
        <small>{detail}</small>
      </span>
    </button>
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
        <small>{[download.version, download.size].filter(Boolean).join(' · ') || download.short || meta.short}</small>
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

function buildQuickActions({
  firstDownload,
  externalUrl,
  buildChannel,
  resources,
  onSelectChannel,
}: {
  firstDownload?: ProjectLandingDownload
  externalUrl?: string
  buildChannel?: Channel
  resources: Array<{ label?: string; url?: string }>
  onSelectChannel: (id: string) => void
}) {
  const actions: Array<{ key: string; icon: LucideIcon; label: string; onClick: () => void }> = []
  if (firstDownload) actions.push({ key: 'download', icon: Download, label: '下载', onClick: () => openDownload(firstDownload) })
  if (buildChannel) actions.push({ key: 'build', icon: PackageCheck, label: '交付记录', onClick: () => onSelectChannel(buildChannel.id) })
  if (externalUrl) actions.push({ key: 'external', icon: ExternalLink, label: '主页', onClick: () => openUrl(externalUrl) })
  const firstResource = resources.find((resource) => resource.url && resource.url !== externalUrl)
  if (firstResource?.url) actions.push({ key: 'resource', icon: Link2, label: firstResource.label || '链接', onClick: () => openUrl(firstResource.url!) })
  return actions.slice(0, 3)
}

function buildHighlightTiles(
  landing: ProjectLandingData | null,
  project: Project,
  availableDownloadCount: number,
  channelCount: number,
) {
  const highlights = landing?.highlights ?? []
  const targetUsers = landing?.target_users ?? []
  const base = [
    {
      icon: Sparkles,
      title: highlights[0] || '项目定位',
      detail: landing?.summary || project.description || '围绕这个项目持续开发、沟通和交付',
    },
    {
      icon: Download,
      title: availableDownloadCount ? '可安装交付' : '交付准备中',
      detail: availableDownloadCount ? `${availableDownloadCount} 个入口可用` : '安装包和网页入口会在发布后出现',
    },
    {
      icon: UsersRound,
      title: targetUsers[0] || '协作空间',
      detail: `${channelCount} 个频道承载需求、讨论和构建记录`,
    },
  ]
  return base
}

function buildActivityItems(project: Project, updatedAt: string, channelCount: number, downloadCount: number) {
  return [
    updatedAt ? `项目最近更新于 ${updatedAt}` : '项目资料已同步到工作台',
    channelCount ? `已配置 ${channelCount} 个项目频道` : '频道配置待补充',
    downloadCount ? `${downloadCount} 个下载或外部入口可用` : '暂无可用下载入口',
    project.unread_count ? `${project.unread_count} 条未读动态` : '',
  ].filter(Boolean)
}

function channelIcon(kind?: string) {
  if (kind === 'ai_development') return <Bot size={14} aria-hidden="true" />
  if (kind === 'builds') return <PackageCheck size={14} aria-hidden="true" />
  if (kind === 'announce') return <Megaphone size={14} aria-hidden="true" />
  return <Hash size={14} aria-hidden="true" />
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
