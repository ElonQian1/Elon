import type { LucideIcon } from 'lucide-react'
import { useState } from 'react'
import {
  ArrowRight,
  CircleCheck,
  Download,
  ExternalLink,
  FileText,
  Hash,
  PackageCheck,
  Rocket,
  UsersRound,
  Wrench,
} from 'lucide-react'
import { formatTime } from '../../lib/utils'
import type { Channel, Project, ProjectLanding as ProjectLandingData, ProjectLandingDownload } from './types'
import ProjectLandingDownloads, {
  firstLandingDownload,
  isLandingDownloadEnabled,
} from './ProjectLandingDownloads'
import QuantPaperLaunch from './QuantPaperLaunch'
import ProjectIntroduction from './ProjectIntroduction'
import { resolveProjectLandingContent } from './projectLandingContent'
import { openProjectWeb, projectWebUrl } from './projectWebEntry'
import styles from './ProjectLanding.module.css'

interface Props {
  project: Project
  channels: Channel[]
  landing: ProjectLandingData | null
  onSelectChannel: (id: string) => void
  onOpenMembers?: () => void
}

interface PrimaryAction {
  icon: LucideIcon
  title: string
  detail: string
  label: string
  disabled?: boolean
  onClick: () => void
}

interface WorkflowItem {
  number: string
  title: string
  detail: string
  action: string
  current?: boolean
  disabled?: boolean
  onClick: () => void
}

export default function ProjectLanding({ project, channels, landing: configuredLanding, onSelectChannel, onOpenMembers }: Props) {
  const landing = resolveProjectLandingContent(project.id, configuredLanding)
  const webUrl = projectWebUrl(landing?.web_url)
  const [webError, setWebError] = useState('')
  const openWeb = () => {
    if (!webUrl) return
    setWebError('')
    try { openProjectWeb(webUrl, project.name, project.id) }
    catch (error) { setWebError(error instanceof Error ? error.message : '网页端暂时无法打开，请重试。') }
  }
  const devChannel = channels.find((channel) => channel.kind === 'ai_development')
  const buildChannel = channels.find((channel) => channel.kind === 'builds')
  const downloads = landing?.downloads ?? []
  const availableDownloads = downloads.filter(isLandingDownloadEnabled)
  const firstDownload = firstLandingDownload(downloads)
  const resources = projectResources(landing)
  const quickChannels = channels
    .filter((channel) => channel.id !== devChannel?.id && channel.id !== buildChannel?.id)
    .slice(0, 6)
  const tagline = landing?.tagline || project.description || '项目空间'
  const description = landing?.summary || landing?.description || project.description || '项目尚未提供介绍，可从现有频道和资料了解当前内容。'
  const updatedAt = project.updated_at ? formatTime(project.updated_at) : ''
  const primaryAction: PrimaryAction = webUrl
    ? { icon: ExternalLink, title: '打开网页端', detail: '使用项目网页应用', label: '打开', onClick: openWeb }
    : buildPrimaryAction({ project, devChannel, buildChannel, firstDownload, resources, onSelectChannel, onOpenMembers })
  const workflow = buildWorkflow({ devChannel, buildChannel, availableDownloads, onSelectChannel })
  const quantPaperLaunch = project.id === 'yilong-quant'
    && landing?.paper_launch?.schema === 'yilong.quant.paper_launch.v1'
    ? landing.paper_launch
    : null
  const windowsWebviewLaunch = landing?.windows_webview?.schema === 'yilong.windows_webview_launch.v1'
    ? landing.windows_webview
    : null

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
            {description !== tagline && <span className={styles.summary}>{description}</span>}
            <div className={styles.metaRow}>
              <MetaPill icon={UsersRound} label={`我的身份：${projectRoleLabel(project.role || project.my_role)}`} />
              <MetaPill icon={UsersRound} label={project.member_count ? `${project.member_count} 位成员` : '项目成员'} />
              <MetaPill icon={Hash} label={`${channels.length} 个频道`} />
              <MetaPill icon={Download} label={availableDownloads.length ? `${availableDownloads.length} 个可用入口` : '等待交付'} />
            </div>
          </div>
        </div>

        <div className={styles.actionPanel}>
          {webError && <p role="alert">{webError}</p>}
          <button
            className={styles.primaryAction}
            type="button"
            disabled={primaryAction.disabled}
            onClick={primaryAction.onClick}
          >
            <span className={styles.primaryIcon}><primaryAction.icon size={23} aria-hidden="true" /></span>
            <span className={styles.primaryCopy}>
              <strong>{primaryAction.title}</strong>
              <small>{primaryAction.detail}</small>
            </span>
            <em>{primaryAction.label}</em>
          </button>
        </div>
      </section>

      {quantPaperLaunch && <QuantPaperLaunch integration={quantPaperLaunch} />}

      <ProjectIntroduction project={project} channels={channels} landing={landing} onSelectChannel={onSelectChannel} onOpenMembers={onOpenMembers} />

      {workflow.length > 0 && <section className={styles.startSection} aria-label="项目工作流程">
        <div className={styles.startHeader}>
          <span className={styles.sectionEyebrow}>从这里继续</span>
          <strong>从参与项目到获取交付，找到下一步</strong>
          <p>第一次进入可以按顺序完成；回来继续时，直接点击当前可用步骤。</p>
        </div>
        <div className={styles.workflowGrid}>
          {workflow.map((item) => <WorkflowStep key={item.number} item={item} />)}
        </div>
      </section>}

      <div className={styles.overviewGrid}>
        <section className={styles.infoPanel}>
          <PanelHeader icon={Hash} title="资料与项目入口" note="快速前往常用频道与资料" />
          {quickChannels.length > 0 && (
            <div className={styles.quickGrid}>
              {quickChannels.map((channel) => (
                <button className={styles.quickChannel} type="button" key={channel.id} onClick={() => onSelectChannel(channel.id)}>
                  <span className={styles.quickIcon}><Hash size={14} aria-hidden="true" /></span>
                  <span>
                    <strong>{channel.name}</strong>
                    <small>{channel.description || channelKindLabel(channel.kind)}</small>
                  </span>
                  <ArrowRight size={15} aria-hidden="true" />
                </button>
              ))}
            </div>
          )}
          {resources.length > 0 && (
            <div className={styles.resourceList}>
              {resources.map((resource) => (
                <button className={styles.resourceLink} type="button" key={`${resource.label}-${resource.url}`} onClick={() => webUrl && projectWebUrl(resource.url) === webUrl ? openWeb() : openUrl(resource.url)}>
                  <FileText size={14} aria-hidden="true" />
                  <span>{resource.label}</span>
                  <ExternalLink size={13} aria-hidden="true" />
                </button>
              ))}
            </div>
          )}
          {quickChannels.length === 0 && resources.length === 0 && (
            <p className={styles.emptyCopy}>项目入口正在配置；你仍可以从左侧频道开始工作。</p>
          )}
        </section>
      </div>

      <ProjectLandingDownloads
        downloads={downloads}
        projectId={project.id}
        projectRole={project.role}
        windowsWebview={windowsWebviewLaunch}
      />
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
  return <span className={styles.metaPill}><Icon size={14} aria-hidden="true" />{label}</span>
}

function PanelHeader({ icon: Icon, title, note }: { icon: LucideIcon; title: string; note: string }) {
  return (
    <div className={styles.panelHeader}>
      <span className={styles.panelHeaderIcon}><Icon size={17} aria-hidden="true" /></span>
      <div><strong>{title}</strong><small>{note}</small></div>
    </div>
  )
}

function WorkflowStep({ item }: { item: WorkflowItem }) {
  return (
    <button
      className={styles.workflowStep}
      data-current={item.current ? 'true' : undefined}
      type="button"
      disabled={item.disabled}
      onClick={item.onClick}
    >
      <span className={styles.workflowNumber}>{item.number}</span>
      <span className={styles.workflowCopy}><strong>{item.title}</strong><small>{item.detail}</small></span>
      <em className={styles.workflowAction}>{item.action}<ArrowRight size={13} aria-hidden="true" /></em>
    </button>
  )
}

function buildWorkflow({
  devChannel,
  buildChannel,
  availableDownloads,
  onSelectChannel,
}: {
  devChannel?: Channel
  buildChannel?: Channel
  availableDownloads: ProjectLandingDownload[]
  onSelectChannel: (id: string) => void
}): WorkflowItem[] {
  const hasDownload = availableDownloads.length > 0
  return [
    {
      number: '1',
      title: '参与功能开发',
      detail: devChannel ? '描述需求、修复问题或继续上次开发。' : '项目还没有配置 AI 开发频道。',
      action: devChannel ? '开始' : '未配置',
      current: !!devChannel,
      disabled: !devChannel,
      onClick: () => devChannel && onSelectChannel(devChannel.id),
    },
    {
      number: '2',
      title: '查看构建与交付',
      detail: buildChannel ? '查看构建、发布和交付进度。' : '项目还没有配置构建频道。',
      action: buildChannel ? '查看' : '未配置',
      disabled: !buildChannel,
      onClick: () => buildChannel && onSelectChannel(buildChannel.id),
    },
    {
      number: '3',
      title: '安装与接入',
      detail: hasDownload ? `${availableDownloads.length} 个入口可直接使用。` : '生成安装包后会在这里出现。',
      action: hasDownload ? '前往' : '等待',
      disabled: !hasDownload,
      onClick: () => document.getElementById('project-landing-downloads')?.scrollIntoView({ behavior: 'smooth', block: 'start' }),
    },
  ].filter(item => !item.disabled).map((item, index) => ({ ...item, number: String(index + 1) }))
}

function buildPrimaryAction({
  project,
  devChannel,
  buildChannel,
  firstDownload,
  resources,
  onSelectChannel,
  onOpenMembers,
}: {
  project: Project
  devChannel?: Channel
  buildChannel?: Channel
  firstDownload?: ProjectLandingDownload
  resources: Array<{ label: string; url: string }>
  onSelectChannel: (id: string) => void
  onOpenMembers?: () => void
}): PrimaryAction {
  const role = project.role || project.my_role
  if (!role || role === 'visitor' || role === 'guest') return {
    icon: UsersRound, title: '了解加入方式', detail: project.join_mode === 'readonly' ? '当前项目提供只读体验' : '查看团队与项目加入规则',
    label: '查看', onClick: () => document.getElementById('project-collaboration')?.scrollIntoView({ block: 'start' }),
  }
  if (devChannel) return { icon: Rocket, title: ['viewer', 'observer'].includes(role) ? '查看开发进度' : '继续开发', detail: devChannel.description || '进入 AI 开发频道', label: devChannel.name, onClick: () => onSelectChannel(devChannel.id) }
  if (onOpenMembers && !buildChannel && !firstDownload && !resources.length) return { icon: UsersRound, title: '查看团队成员', detail: '了解同一项目中的成员', label: '查看', onClick: onOpenMembers }
  if (buildChannel) return { icon: PackageCheck, title: '查看交付', detail: buildChannel.description || '进入构建与安装包频道', label: buildChannel.name, onClick: () => onSelectChannel(buildChannel.id) }
  if (firstDownload) return { icon: Download, title: '安装使用', detail: firstDownload.label || '下载可用客户端', label: '下载', onClick: () => document.getElementById('project-landing-downloads')?.scrollIntoView({ behavior: 'smooth', block: 'start' }) }
  if (resources[0]) return { icon: ExternalLink, title: '打开项目', detail: '查看项目主页或外部入口', label: '打开', onClick: () => openUrl(resources[0].url) }
  return { icon: Wrench, title: '等待配置', detail: '项目入口会在频道或交付配置后出现', label: '未就绪', disabled: true, onClick: () => undefined }
}

function projectRoleLabel(role?: string) {
  return ({ owner: '项目所有者', admin: '管理员', editor: '开发成员', viewer: '只读成员', member: '项目成员' } as Record<string, string>)[role || ''] || '访客'
}

function projectResources(landing: ProjectLandingData | null) {
  const candidates = [
    landing?.custom_landing_url ? { label: '完整项目介绍', url: landing.custom_landing_url } : null,
    landing?.web_url ? { label: '打开网页端', url: landing.web_url } : null,
    ...(landing?.resources ?? []).map((resource) => ({ label: resource.label || '相关资料', url: resource.url || '' })),
  ].filter((resource): resource is { label: string; url: string } => !!resource?.url)
  return candidates.filter((resource, index) => candidates.findIndex((candidate) => candidate.url === resource.url) === index)
}

function channelKindLabel(kind?: string) {
  if (kind === 'announcements' || kind === 'announcement') return '项目公告'
  if (kind === 'docs') return '项目文档'
  if (kind === 'discussion') return '项目讨论'
  if (kind === 'issues') return '问题与反馈'
  return '项目频道'
}

function openUrl(url: string) {
  if (url) window.open(url, '_blank', 'noopener')
}
