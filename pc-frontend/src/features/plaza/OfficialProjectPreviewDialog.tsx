import { useCallback, useEffect, useState } from 'react'
import { AlertTriangle, CheckCircle2, Clock3, Info, Loader2, RefreshCw, ShieldCheck, X } from 'lucide-react'
import { api } from '../../api/client'
import type { PlazaProject } from './ProjectPlazaView'
import styles from './OfficialProjectPreviewDialog.module.css'

interface OfficialProjectPreviewDownload {
  platform: string
  kind?: string
  label?: string
  status: string
  note?: string
}

interface OfficialProjectPaperPreview {
  schema: string
  mode: string
  simulated: boolean
  funds_moved: boolean
  target_is_guaranteed: boolean
  label?: string
  description?: string
}

interface OfficialProjectPreview {
  schema: 'yilong.official_project_preview.v1'
  project_id: string
  title: string
  tagline?: string
  summary: string
  description?: string
  highlights: string[]
  target_users: string[]
  recent_updates: string[]
  privacy_notes: string[]
  system_requirements: string[]
  downloads: OfficialProjectPreviewDownload[]
  paper_launch?: OfficialProjectPaperPreview
}

interface Props {
  project: PlazaProject
  onClose: () => void
}

export default function OfficialProjectPreviewDialog({ project, onClose }: Props) {
  const [preview, setPreview] = useState<OfficialProjectPreview | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')

  const loadPreview = useCallback(async () => {
    setLoading(true)
    setError('')
    try {
      const result = await api.get<{ preview: OfficialProjectPreview }>(
        `/api/store/projects/${encodeURIComponent(project.id)}/preview`,
      )
      if (result.preview?.schema !== 'yilong.official_project_preview.v1') {
        throw new Error('项目详情格式暂不受支持')
      }
      setPreview(result.preview)
    } catch (caught) {
      setPreview(null)
      setError((caught as { message?: string }).message ?? '项目详情加载失败')
    } finally {
      setLoading(false)
    }
  }, [project.id])

  useEffect(() => {
    void loadPreview()
  }, [loadPreview])

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [onClose])

  const title = preview?.title || project.display_name || project.name

  return (
    <div className={styles.backdrop} role="presentation" onMouseDown={onClose}>
      <section
        className={styles.dialog}
        role="dialog"
        aria-modal="true"
        aria-labelledby="official-project-preview-title"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <header className={styles.header}>
          <span className={styles.icon}><Info size={20} aria-hidden="true" /></span>
          <div>
            <span className={styles.eyebrow}>了解项目详情</span>
            <strong id="official-project-preview-title">{title}</strong>
            <span>{preview?.tagline || '一龙官方项目'}</span>
          </div>
          <button className={styles.close} type="button" aria-label="关闭项目详情" autoFocus onClick={onClose}>
            <X size={18} aria-hidden="true" />
          </button>
        </header>

        <div className={styles.body}>
          {loading && (
            <div className={styles.feedback} aria-live="polite">
              <Loader2 className={styles.spinner} size={22} aria-hidden="true" />
              <strong>正在读取官方项目说明…</strong>
              <span>这里只读取公开信息，不会自动加入项目。</span>
            </div>
          )}

          {!loading && error && (
            <div className={styles.feedback} role="alert">
              <AlertTriangle size={22} aria-hidden="true" />
              <strong>项目详情暂时没有加载出来</strong>
              <span>{error}</span>
              <button className={styles.retry} type="button" onClick={() => void loadPreview()}>
                <RefreshCw size={14} aria-hidden="true" />重新加载
              </button>
            </div>
          )}

          {!loading && preview && (
            <>
              <section className={styles.summary}>
                <p>{preview.summary}</p>
                {preview.description && preview.description !== preview.summary && <span>{preview.description}</span>}
              </section>

              {preview.paper_launch && (
                <section className={styles.paperNotice} aria-label="Paper 模拟边界">
                  <ShieldCheck size={20} aria-hidden="true" />
                  <div>
                    <strong>{preview.paper_launch.label || 'Paper 模拟测试'}</strong>
                    <span>{preview.paper_launch.description || '当前只提供模拟能力。'}</span>
                    <small>
                      {preview.paper_launch.simulated ? '模拟环境' : '状态未知'} ·
                      {preview.paper_launch.funds_moved ? ' 涉及资金移动' : ' 不移动真实资金'} ·
                      {preview.paper_launch.target_is_guaranteed ? ' 保证目标' : ' 不保证收益目标'}
                    </small>
                  </div>
                </section>
              )}

              <PreviewList title="当前能力" items={preview.highlights} />
              <PreviewList title="适合谁" items={preview.target_users} />
              <PreviewList title="最近更新" items={preview.recent_updates} compact />

              {preview.downloads.length > 0 && (
                <section className={styles.section}>
                  <h3>客户端计划</h3>
                  <div className={styles.downloads}>
                    {preview.downloads.map((download) => (
                      <article key={`${download.platform}-${download.label || ''}`}>
                        <span className={styles.downloadIcon}><Clock3 size={16} aria-hidden="true" /></span>
                        <div>
                          <strong>{download.label || platformLabel(download.platform)}</strong>
                          <span>{download.note || '尚未提供公开下载'}</span>
                        </div>
                        <em data-status={download.status}>{statusLabel(download.status)}</em>
                      </article>
                    ))}
                  </div>
                </section>
              )}

              <PreviewList title="隐私与风险说明" items={preview.privacy_notes} warning />
              <PreviewList title="系统要求" items={preview.system_requirements} compact />
            </>
          )}
        </div>

        <footer className={styles.footer}>
          <span><CheckCircle2 size={14} aria-hidden="true" />官方目录只读信息</span>
          <button type="button" onClick={onClose}>我知道了</button>
        </footer>
      </section>
    </div>
  )
}

function PreviewList({
  title,
  items,
  compact = false,
  warning = false,
}: {
  title: string
  items: string[]
  compact?: boolean
  warning?: boolean
}) {
  if (!items.length) return null
  return (
    <section className={styles.section} data-compact={compact || undefined} data-warning={warning || undefined}>
      <h3>{title}</h3>
      <ul>{items.map((item) => <li key={item}>{item}</li>)}</ul>
    </section>
  )
}

function platformLabel(platform: string) {
  if (platform === 'web') return 'Web 技术测试面板'
  if (platform === 'windows') return 'Windows 桌面端'
  if (platform === 'android') return 'Android 客户端'
  return platform || '客户端'
}

function statusLabel(status: string) {
  if (status === 'available') return '可用'
  if (status === 'partial') return '部分可用'
  if (status === 'planned' || status === 'coming_soon') return '规划中'
  if (status === 'pending') return '准备中'
  return '暂不可用'
}
