/**
 * ProjectLanding — 项目首页组件（Discord 式 landing page）
 * 与旧版 pc_project_landing.js 功能对等：
 *   - Hero：图标 + 名称 + 一句话 tagline + 最近更新
 *   - 建议第一步：3 步引导（AI 开发 → 打包 → 下载）
 *   - 项目描述
 *   - 多端下载卡片
 *   - 项目亮点 / 适用人群标签
 *   - 项目入口：频道快捷按钮 + 资源链接
 */
import type { Project, Channel, ProjectLanding as ProjectLandingData } from './types'
import { formatTime } from '../../lib/utils'
import styles from './ProjectLanding.module.css'

interface Props {
  project: Project
  channels: Channel[]
  landing: ProjectLandingData | null
  onSelectChannel: (id: string) => void
}

const ACTIVE_STATUSES = new Set(['available', 'external'])

export default function ProjectLanding({ project, channels, landing, onSelectChannel }: Props) {
  const devCh = channels.find((c) => c.kind === 'ai_development')
  const buildCh = channels.find((c) => c.kind === 'builds')
  const downloads = landing?.downloads ?? []
  const hasDownload = downloads.some((d) => ACTIVE_STATUSES.has(d.status ?? '') || (d.url && !d.status))
  const highlights = landing?.highlights ?? []
  const targetUsers = landing?.target_users ?? []
  const resources = landing?.resources ?? []
  const tagline = landing?.tagline || project.description || '项目首页'
  const description = landing?.summary || landing?.description

  function scrollToDownloads() {
    document.getElementById('landing-downloads')?.scrollIntoView({ behavior: 'smooth', block: 'start' })
  }

  return (
    <div className={styles.landing}>

      {/* ── Hero ── */}
      <div className={styles.hero}>
        {(project.icon_data_url || project.icon)
          ? <img src={project.icon_data_url || project.icon} alt="" className={styles.heroLogo} />
          : <div className={styles.heroLogoFallback}>{(project.name?.[0] ?? '项').toUpperCase()}</div>
        }
        <div className={styles.heroCopy}>
          {project.updated_at && (
            <div className={styles.heroKicker}>最近更新 {formatTime(project.updated_at)}</div>
          )}
          <h2 className={styles.heroName}>{project.name}</h2>
          <p className={styles.heroTagline}>{tagline}</p>
        </div>
      </div>

      {/* ── 建议第一步 ── */}
      <div className={styles.startSection}>
        <div className={styles.startLabel}>建议第一步</div>
        <div className={styles.steps}>
          <button
            className={[styles.step, devCh ? styles.stepActive : styles.stepDisabled].join(' ')}
            type="button"
            onClick={() => devCh && onSelectChannel(devCh.id)}
            disabled={!devCh}
          >
            <span className={styles.stepNum}>1</span>
            <span className={styles.stepCopy}>
              <strong>开始做应用</strong>
              <small>{devCh ? '描述需求或要改的功能' : 'AI 开发频道未配置'}</small>
            </span>
          </button>
          <button
            className={[styles.step, !devCh && buildCh ? styles.stepActive : (buildCh ? '' : styles.stepDisabled)].join(' ')}
            type="button"
            onClick={() => buildCh && onSelectChannel(buildCh.id)}
            disabled={!buildCh}
          >
            <span className={styles.stepNum}>2</span>
            <span className={styles.stepCopy}>
              <strong>生成安装包</strong>
              <small>{buildCh ? '需求做完后打包交付' : '打包频道未配置'}</small>
            </span>
          </button>
          <button
            className={[styles.step, !devCh && !buildCh && hasDownload ? styles.stepActive : (hasDownload ? '' : styles.stepDisabled)].join(' ')}
            type="button"
            onClick={hasDownload ? scrollToDownloads : undefined}
            disabled={!hasDownload}
          >
            <span className={styles.stepNum}>3</span>
            <span className={styles.stepCopy}>
              <strong>安装使用</strong>
              <small>{hasDownload ? '下载可用客户端' : '安装包生成后可下载'}</small>
            </span>
          </button>
        </div>
      </div>

      {/* ── 项目描述 ── */}
      {description && <p className={styles.description}>{description}</p>}

      {/* ── 多端下载 ── */}
      {downloads.length > 0 && (
        <div id="landing-downloads" className={styles.section}>
          <div className={styles.sectionTitle}>下载安装</div>
          <div className={styles.downloadGrid}>
            {downloads.map((d, i) => {
              const enabled = ACTIVE_STATUSES.has(d.status ?? '') || (!!d.url && !d.status)
              const statusLabel = d.status === 'coming_soon' ? '即将支持'
                : d.status === 'planned' ? '计划中'
                : d.status === 'unavailable' ? '暂不可用'
                : enabled ? '下载' : '待配置'
              return (
                <button
                  key={d.platform ?? i}
                  className={[styles.downloadCard, enabled ? '' : styles.downloadDisabled].join(' ')}
                  type="button"
                  disabled={!enabled}
                  onClick={() => enabled && d.url && window.open(d.url, '_blank', 'noopener')}
                >
                  <strong>{d.short || d.platform || '通用'}</strong>
                  <span>{d.label || d.platform || '下载'}</span>
                  {(d.version || d.size) && <small>{[d.version, d.size].filter(Boolean).join(' · ')}</small>}
                  {d.note && <small className={styles.downloadNote}>{d.note}</small>}
                  <em>{statusLabel}</em>
                </button>
              )
            })}
          </div>
        </div>
      )}

      {/* ── 项目亮点 ── */}
      {highlights.length > 0 && (
        <div className={styles.section}>
          <div className={styles.sectionTitle}>项目亮点</div>
          <div className={styles.tagList}>
            {highlights.map((h, i) => <span key={i} className={styles.tag}>{h}</span>)}
          </div>
        </div>
      )}

      {/* ── 适用人群 ── */}
      {targetUsers.length > 0 && (
        <div className={styles.section}>
          <div className={styles.sectionTitle}>适用人群</div>
          <div className={styles.tagList}>
            {targetUsers.map((t, i) => <span key={i} className={[styles.tag, styles.tagAudience].join(' ')}>{t}</span>)}
          </div>
        </div>
      )}

      {/* ── 项目入口 ── */}
      {(channels.length > 0 || resources.length > 0 || landing?.custom_landing_url || landing?.web_url) && (
        <div className={styles.section}>
          <div className={styles.sectionTitle}>项目入口</div>
          <div className={styles.entrances}>
            {channels.map((c) => (
              <button
                key={c.id}
                className={styles.channelBtn}
                type="button"
                onClick={() => onSelectChannel(c.id)}
              >
                <span>{c.kind === 'ai_development' ? '🛠️' : '#'}</span>
                <strong>{c.name}</strong>
                {c.description && <span>{c.description}</span>}
              </button>
            ))}
            {(landing?.custom_landing_url || landing?.web_url) && (
              <button
                className={styles.resourceBtn}
                type="button"
                onClick={() => window.open((landing.custom_landing_url || landing.web_url)!, '_blank', 'noopener')}
              >
                完整介绍 ↗
              </button>
            )}
            {resources.map((r, i) => r.url ? (
              <button
                key={i}
                className={styles.resourceBtn}
                type="button"
                onClick={() => window.open(r.url!, '_blank', 'noopener')}
              >
                {r.label || '相关链接'} ↗
              </button>
            ) : null)}
          </div>
        </div>
      )}

    </div>
  )
}
