import { useEffect, useMemo, useState } from 'react'
import { getAuthToken } from '../../api/client'
import { resolveApiUrl } from '../../api/runtime'
import type { ProjectRelease } from './projectManagementTypes'
import styles from './ProjectDetailPage.module.css'

interface Props {
  projectId: string
  canEdit: boolean
}

const OFFICIAL_QUANT_PROJECT_ID = 'yilong-quant'
const OFFICIAL_QUANT_PACKAGE_NAME = 'com.elon.quant'
const OFFICIAL_QUANT_CHANNEL = 'paper'
const OFFICIAL_QUANT_MIN_VERSION_CODE = 5
const OFFICIAL_QUANT_MIN_VERSION_NAME = '0.5.0'
const SOURCE_GIT_SHA_PATTERN = /^[0-9a-f]{40}$/
const INTEGER_VERSION_CODE_PATTERN = /^(0|[1-9]\d*)$/
const CANONICAL_VERSION_NAME_PATTERN = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/

export default function ProjectReleasesTab({ projectId, canEdit }: Props) {
  const isOfficialQuant = projectId === OFFICIAL_QUANT_PROJECT_ID
  const [releases, setReleases] = useState<ProjectRelease[]>([])
  const [loading, setLoading] = useState(false)
  const [file, setFile] = useState<File | null>(null)
  const [versionName, setVersionName] = useState('')
  const [versionCode, setVersionCode] = useState('')
  const [packageName, setPackageName] = useState(isOfficialQuant ? OFFICIAL_QUANT_PACKAGE_NAME : '')
  const [channel, setChannel] = useState(isOfficialQuant ? OFFICIAL_QUANT_CHANNEL : 'stable')
  const [sourceGitSha, setSourceGitSha] = useState('')
  const [changelog, setChangelog] = useState('')
  const [busy, setBusy] = useState(false)
  const [downloadingReleaseId, setDownloadingReleaseId] = useState('')
  const [message, setMessage] = useState('')
  const [error, setError] = useState('')
  const latest = isOfficialQuant
    ? releases.find((release) => release.installable === true)
    : releases[0]

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

  useEffect(() => {
    if (isOfficialQuant) {
      setPackageName(OFFICIAL_QUANT_PACKAGE_NAME)
      setChannel(OFFICIAL_QUANT_CHANNEL)
    } else {
      setPackageName((current) => current === OFFICIAL_QUANT_PACKAGE_NAME ? '' : current)
      setChannel((current) => current === OFFICIAL_QUANT_CHANNEL ? 'stable' : current)
    }
    setVersionCode('')
    setSourceGitSha('')
  }, [isOfficialQuant])

  const uploadDisabled = useMemo(() => !canEdit || busy || !file, [busy, canEdit, file])

  async function uploadRelease(event: React.FormEvent) {
    event.preventDefault()
    if (!projectId || !file || uploadDisabled) return
    setMessage('')
    setError('')
    if (isOfficialQuant) {
      const validationError = officialQuantUploadError({
        versionName,
        versionCode,
        packageName,
        channel,
        sourceGitSha,
      })
      if (validationError) {
        setError(validationError)
        return
      }
    }
    setBusy(true)
    try {
      const params = new URLSearchParams()
      params.set('file_name', file.name)
      if (versionName.trim()) params.set('version_name', versionName.trim())
      if (versionCode.trim()) params.set('version_code', versionCode.trim())
      if (packageName.trim()) params.set('package_name', packageName.trim())
      if (channel.trim()) params.set('channel', channel.trim())
      if (sourceGitSha.trim()) params.set('source_git_sha', sourceGitSha.trim())
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
      setVersionCode('')
      setPackageName(isOfficialQuant ? OFFICIAL_QUANT_PACKAGE_NAME : '')
      setSourceGitSha('')
      setChangelog('')
      setMessage('APK 已上传并记录发布历史')
      await loadReleases()
    } catch (err) {
      setError(errorMessage(err, 'APK 上传失败'))
    } finally {
      setBusy(false)
    }
  }

  async function downloadRelease(release: ProjectRelease) {
    const token = getAuthToken()
    if (!token) {
      setError('请先登录后下载 APK')
      return
    }
    setMessage('')
    setError('')
    setDownloadingReleaseId(release.id)
    try {
      const response = await fetch(releaseDownloadUrl(projectId, release.id), {
        headers: { Authorization: `Bearer ${token}` },
      })
      if (!response.ok) throw new Error(await responseText(response, 'APK 下载失败'))
      const blob = await response.blob()
      const downloadUrl = URL.createObjectURL(blob)
      const anchor = document.createElement('a')
      anchor.href = downloadUrl
      anchor.download = release.file_name?.trim() || 'app-release.apk'
      document.body.appendChild(anchor)
      anchor.click()
      anchor.remove()
      window.setTimeout(() => URL.revokeObjectURL(downloadUrl), 1_000)
    } catch (err) {
      setError(errorMessage(err, 'APK 下载失败'))
    } finally {
      setDownloadingReleaseId('')
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
            <input value={versionName} onChange={(event) => setVersionName(event.target.value)} placeholder={isOfficialQuant ? OFFICIAL_QUANT_MIN_VERSION_NAME : '1.0.0'} disabled={!canEdit || busy} />
          </label>
          <label className={styles.field}>
            <span>版本号（versionCode）</span>
            <input type="number" min={isOfficialQuant ? OFFICIAL_QUANT_MIN_VERSION_CODE : undefined} step="1" inputMode="numeric" value={versionCode} onChange={(event) => setVersionCode(event.target.value)} placeholder={isOfficialQuant ? String(OFFICIAL_QUANT_MIN_VERSION_CODE) : '可选'} disabled={!canEdit || busy} />
          </label>
          <label className={styles.field}>
            <span>包名</span>
            <input value={packageName} onChange={(event) => setPackageName(event.target.value)} placeholder="com.example.app" readOnly={isOfficialQuant} disabled={!canEdit || busy} />
          </label>
          <label className={styles.field}>
            <span>渠道</span>
            <select value={channel} onChange={(event) => setChannel(event.target.value)} disabled={!canEdit || busy || isOfficialQuant}>
              {isOfficialQuant ? (
                <option value={OFFICIAL_QUANT_CHANNEL}>{OFFICIAL_QUANT_CHANNEL}</option>
              ) : (
                <>
                  <option value="stable">stable</option>
                  <option value="beta">beta</option>
                  <option value="internal">internal</option>
                </>
              )}
            </select>
          </label>
          <label className={styles.fieldWide}>
            <span>源码 Git SHA</span>
            <input value={sourceGitSha} onChange={(event) => setSourceGitSha(event.target.value)} placeholder={isOfficialQuant ? '40 位小写十六进制提交 SHA' : '可选'} spellCheck={false} disabled={!canEdit || busy} />
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
        {canEdit && isOfficialQuant && <p className={styles.formHint}>一龙量化只接受 com.elon.quant、paper 渠道、0.5.0（versionCode 5）及以上正式版本；APK 摘要由服务器计算。</p>}
        {message && <p className={styles.formSuccess}>{message}</p>}
        {error && <p className={styles.formError}>{error}</p>}
      </section>

      <section className={styles.panel}>
        <div className={styles.panelHeader}>
          <div>
            <strong>发布历史</strong>
            <span>{latest
              ? `最新版本 ${releaseVersionLabel(latest)}`
              : isOfficialQuant
                ? '暂无可安装新版'
                : '暂无发布记录'}</span>
          </div>
          <span>{releases.length} 条</span>
        </div>
        <div className={styles.rowList}>
          {releases.map((release) => (
            <div className={styles.rowItem} key={release.id}>
              <div className={styles.rowMain}>
                <strong>{releaseVersionLabel(release)}</strong>
                <span>
                  {release.package_name || '未记录包名'} · {release.channel || 'stable'} · versionCode {release.version_code ?? '未记录'} · {formatBytes(release.size_bytes)} · {formatDate(release.created_at)}
                </span>
                {release.source_git_sha && <span title={release.source_git_sha}>源码 Git SHA：{release.source_git_sha}</span>}
                {release.sha256 && <span title={release.sha256}>服务器 SHA-256：{release.sha256}</span>}
                {release.changelog && <em>{release.changelog}</em>}
              </div>
              <div className={styles.rowActions}>
                {isOfficialQuant && release.installable !== true ? (
                  <span className={styles.formHint}>审计记录，不可安装</span>
                ) : (
                  <button
                    className={styles.textBtn}
                    type="button"
                    onClick={() => downloadRelease(release)}
                    disabled={downloadingReleaseId === release.id}
                  >
                    {downloadingReleaseId === release.id ? '下载中' : '下载'}
                  </button>
                )}
              </div>
            </div>
          ))}
          {releases.length === 0 && <p className={styles.empty}>暂无发布历史</p>}
        </div>
      </section>
    </div>
  )
}

interface OfficialQuantUploadFields {
  versionName: string
  versionCode: string
  packageName: string
  channel: string
  sourceGitSha: string
}

function officialQuantUploadError(fields: OfficialQuantUploadFields) {
  if (fields.packageName.trim() !== OFFICIAL_QUANT_PACKAGE_NAME || fields.channel.trim() !== OFFICIAL_QUANT_CHANNEL) {
    return `一龙量化只允许 ${OFFICIAL_QUANT_PACKAGE_NAME} 的 ${OFFICIAL_QUANT_CHANNEL} 渠道发布`
  }
  const parsedVersionName = parseCanonicalVersionName(fields.versionName.trim())
  const minimumVersionName = parseCanonicalVersionName(OFFICIAL_QUANT_MIN_VERSION_NAME)
  if (!parsedVersionName || !minimumVersionName || compareVersionNames(parsedVersionName, minimumVersionName) < 0) {
    return `一龙量化版本名必须使用 x.y.z 格式，且不低于 ${OFFICIAL_QUANT_MIN_VERSION_NAME}`
  }
  const rawVersionCode = fields.versionCode.trim()
  const parsedVersionCode = Number(rawVersionCode)
  if (!INTEGER_VERSION_CODE_PATTERN.test(rawVersionCode) || !Number.isSafeInteger(parsedVersionCode) || parsedVersionCode < OFFICIAL_QUANT_MIN_VERSION_CODE) {
    return `一龙量化 versionCode 必须是至少 ${OFFICIAL_QUANT_MIN_VERSION_CODE} 的整数`
  }
  if (!SOURCE_GIT_SHA_PATTERN.test(fields.sourceGitSha.trim())) {
    return '一龙量化源码 Git SHA 必须是 40 位小写十六进制字符'
  }
  return ''
}

type ParsedVersionName = [number, number, number]

function parseCanonicalVersionName(value: string): ParsedVersionName | null {
  const match = value.match(CANONICAL_VERSION_NAME_PATTERN)
  if (!match) return null
  const parts = match.slice(1).map(Number)
  if (parts.some((part) => !Number.isSafeInteger(part))) return null
  return parts as ParsedVersionName
}

function compareVersionNames(left: ParsedVersionName, right: ParsedVersionName) {
  for (let index = 0; index < left.length; index += 1) {
    if (left[index] !== right[index]) return left[index] - right[index]
  }
  return 0
}

function releaseVersionLabel(release: ProjectRelease) {
  const name = release.version_name || release.file_name || release.id
  return release.version_code == null ? name : `${name} (${release.version_code})`
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
