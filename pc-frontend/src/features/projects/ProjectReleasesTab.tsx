import { useEffect, useMemo, useState } from 'react'
import { getAuthToken } from '../../api/client'
import { resolveApiUrl } from '../../api/runtime'
import type { ProjectRelease } from './projectManagementTypes'
import styles from './ProjectDetailPage.module.css'

interface Props {
  projectId: string
  canEdit: boolean
}

export default function ProjectReleasesTab({ projectId, canEdit }: Props) {
  const [releases, setReleases] = useState<ProjectRelease[]>([])
  const [loading, setLoading] = useState(false)
  const [file, setFile] = useState<File | null>(null)
  const [versionName, setVersionName] = useState('')
  const [packageName, setPackageName] = useState('')
  const [channel, setChannel] = useState('stable')
  const [changelog, setChangelog] = useState('')
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')
  const [error, setError] = useState('')
  const latest = releases[0]

  async function loadReleases() {
    if (!projectId) return
    setLoading(true)
    setError('')
    try {
      const response = await fetch(resolveApiUrl(`/api/projects/${encodeURIComponent(projectId)}/releases`), {
        headers: authHeaders(),
      })
      if (!response.ok) throw new Error(await responseText(response, '发布历史读取失败'))
      const data = await response.json() as { releases?: ProjectRelease[] }
      setReleases(data.releases ?? [])
    } catch (err) {
      setError(errorMessage(err, '发布历史读取失败'))
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    loadReleases()
  }, [projectId])

  const uploadDisabled = useMemo(() => !canEdit || busy || !file, [busy, canEdit, file])

  async function uploadRelease(event: React.FormEvent) {
    event.preventDefault()
    if (!projectId || !file || uploadDisabled) return
    setBusy(true)
    setMessage('')
    setError('')
    try {
      const params = new URLSearchParams()
      params.set('file_name', file.name)
      if (versionName.trim()) params.set('version_name', versionName.trim())
      if (packageName.trim()) params.set('package_name', packageName.trim())
      if (channel.trim()) params.set('channel', channel.trim())
      if (changelog.trim()) params.set('changelog', changelog.trim())
      const response = await fetch(resolveApiUrl(`/api/projects/${encodeURIComponent(projectId)}/releases?${params}`), {
        method: 'POST',
        headers: {
          ...authHeaders(),
          'Content-Type': file.type || 'application/vnd.android.package-archive',
        },
        body: file,
      })
      if (!response.ok) throw new Error(await responseText(response, 'APK 上传失败'))
      setFile(null)
      setVersionName('')
      setPackageName('')
      setChangelog('')
      setMessage('APK 已上传并记录发布历史')
      await loadReleases()
    } catch (err) {
      setError(errorMessage(err, 'APK 上传失败'))
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className={styles.managementStack}>
      <section className={styles.panel}>
        <div className={styles.panelHeader}>
          <div>
            <strong>APK 管理</strong>
            <span>上传正式 APK，生成下载记录，并同步到项目发布历史。</span>
          </div>
          <button className={styles.textBtn} type="button" onClick={loadReleases} disabled={loading}>
            {loading ? '读取中' : '刷新'}
          </button>
        </div>
        <form className={styles.managementForm} onSubmit={uploadRelease}>
          <label className={styles.field}>
            <span>APK 文件</span>
            <input type="file" accept=".apk,application/vnd.android.package-archive" onChange={(event) => setFile(event.target.files?.[0] ?? null)} disabled={!canEdit || busy} />
          </label>
          <label className={styles.field}>
            <span>版本名</span>
            <input value={versionName} onChange={(event) => setVersionName(event.target.value)} placeholder="1.0.0" disabled={!canEdit || busy} />
          </label>
          <label className={styles.field}>
            <span>包名</span>
            <input value={packageName} onChange={(event) => setPackageName(event.target.value)} placeholder="com.example.app" disabled={!canEdit || busy} />
          </label>
          <label className={styles.field}>
            <span>渠道</span>
            <select value={channel} onChange={(event) => setChannel(event.target.value)} disabled={!canEdit || busy}>
              <option value="stable">stable</option>
              <option value="beta">beta</option>
              <option value="internal">internal</option>
            </select>
          </label>
          <label className={styles.fieldWide}>
            <span>发布说明</span>
            <textarea value={changelog} onChange={(event) => setChangelog(event.target.value)} placeholder="本次更新内容" disabled={!canEdit || busy} />
          </label>
          <button className={styles.primaryBtn} type="submit" disabled={uploadDisabled}>
            {busy ? '上传中' : '上传 APK'}
          </button>
        </form>
        {!canEdit && <p className={styles.formHint}>当前角色可查看发布历史；上传 APK 需要项目编辑权限。</p>}
        {message && <p className={styles.formSuccess}>{message}</p>}
        {error && <p className={styles.formError}>{error}</p>}
      </section>

      <section className={styles.panel}>
        <div className={styles.panelHeader}>
          <div>
            <strong>发布历史</strong>
            <span>{latest ? `最新版本 ${latest.version_name || latest.file_name || latest.id}` : '暂无发布记录'}</span>
          </div>
          <span>{releases.length} 条</span>
        </div>
        <div className={styles.rowList}>
          {releases.map((release) => (
            <div className={styles.rowItem} key={release.id}>
              <div className={styles.rowMain}>
                <strong>{release.version_name || release.file_name || release.id}</strong>
                <span>
                  {release.package_name || '未记录包名'} · {release.channel || 'stable'} · {formatBytes(release.size_bytes)} · {formatDate(release.created_at)}
                </span>
                {release.changelog && <em>{release.changelog}</em>}
              </div>
              <div className={styles.rowActions}>
                <a className={styles.textBtn} href={releaseDownloadUrl(projectId, release.id)} target="_blank" rel="noreferrer">下载</a>
              </div>
            </div>
          ))}
          {releases.length === 0 && <p className={styles.empty}>暂无发布历史</p>}
        </div>
      </section>
    </div>
  )
}

function authHeaders(): Record<string, string> {
  const token = getAuthToken()
  if (!token) return {}
  return { Authorization: `Bearer ${token}` }
}

async function responseText(response: Response, fallback: string) {
  try {
    const body = await response.json()
    return body?.error || body?.message || fallback
  } catch {
    return response.statusText || fallback
  }
}

function releaseDownloadUrl(projectId: string, releaseId: string) {
  return resolveApiUrl(`/api/projects/${encodeURIComponent(projectId)}/releases/${encodeURIComponent(releaseId)}/download.apk`)
}

function formatBytes(value?: number | null) {
  if (!value || value <= 0) return '未知大小'
  if (value >= 1024 * 1024) return `${(value / 1024 / 1024).toFixed(1)} MB`
  return `${Math.round(value / 1024)} KB`
}

function formatDate(value?: string) {
  if (!value) return '-'
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString('zh-CN')
}

function errorMessage(err: unknown, fallback: string) {
  return (err as { message?: string })?.message ?? fallback
}
