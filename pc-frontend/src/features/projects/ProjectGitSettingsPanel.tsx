import { useEffect, useState } from 'react'
import { api } from '../../api/client'
import type { ProjectGitStatus } from './projectManagementTypes'
import styles from './ProjectDetailPage.module.css'

interface Props {
  projectId: string
  currentUserId?: string
  canEdit: boolean
}

export default function ProjectGitSettingsPanel({ projectId, currentUserId, canEdit }: Props) {
  const [status, setStatus] = useState<ProjectGitStatus | null>(null)
  const [repoUrl, setRepoUrl] = useState('')
  const [branch, setBranch] = useState('main')
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState<'save' | 'key' | ''>('')
  const [message, setMessage] = useState('')
  const [error, setError] = useState('')

  async function loadStatus() {
    if (!projectId || !currentUserId) return
    setLoading(true)
    setError('')
    try {
      const data = await api.get<ProjectGitStatus>(gitPath(currentUserId, projectId, 'status'))
      setStatus(data)
      setRepoUrl(data.git?.origin ?? '')
      setBranch(data.git?.branch ?? 'main')
    } catch (err) {
      setError(errorMessage(err, 'Git 状态读取失败'))
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    loadStatus()
  }, [projectId, currentUserId])

  async function saveConfig(event: React.FormEvent) {
    event.preventDefault()
    if (!projectId || !currentUserId || !canEdit || !repoUrl.trim()) return
    setBusy('save')
    setMessage('')
    setError('')
    try {
      const data = await api.post<ProjectGitStatus>(gitPath(currentUserId, projectId, 'config'), {
        repo_url: repoUrl.trim(),
        branch: branch.trim() || 'main',
        auth_type: 'deploy_key',
      })
      setStatus(data)
      setMessage('Git 仓库设置已保存')
    } catch (err) {
      setError(errorMessage(err, '保存 Git 设置失败'))
    } finally {
      setBusy('')
    }
  }

  async function createDeployKey() {
    if (!projectId || !currentUserId || !canEdit) return
    setBusy('key')
    setMessage('')
    setError('')
    try {
      const data = await api.post<{ public_key?: string; status?: ProjectGitStatus }>(
        gitPath(currentUserId, projectId, 'deploy-key'),
        {},
      )
      if (data.status) setStatus(data.status)
      setMessage('Deploy Key 已生成，可复制到 Git 托管平台')
    } catch (err) {
      setError(errorMessage(err, '生成 Deploy Key 失败'))
    } finally {
      setBusy('')
    }
  }

  return (
    <section className={styles.panel}>
      <div className={styles.panelHeader}>
        <div>
          <strong>Git 设置</strong>
          <span>配置远端仓库、默认分支和每项目 Deploy Key。</span>
        </div>
        <button className={styles.textBtn} type="button" onClick={loadStatus} disabled={loading || !currentUserId}>
          {loading ? '读取中' : '刷新'}
        </button>
      </div>

      <form className={styles.managementForm} onSubmit={saveConfig}>
        <label className={styles.field}>
          <span>远端仓库</span>
          <input value={repoUrl} onChange={(event) => setRepoUrl(event.target.value)} placeholder="git@github.com:owner/repo.git" disabled={!canEdit || busy === 'save'} />
        </label>
        <label className={styles.field}>
          <span>默认分支</span>
          <input value={branch} onChange={(event) => setBranch(event.target.value)} placeholder="main" disabled={!canEdit || busy === 'save'} />
        </label>
        <button className={styles.primaryBtn} type="submit" disabled={!canEdit || busy === 'save' || !repoUrl.trim()}>
          {busy === 'save' ? '保存中' : '保存 Git'}
        </button>
      </form>

      <div className={styles.overviewGrid}>
        <div className={styles.kv}><span>工作区</span><strong>{status?.workspace ?? '-'}</strong></div>
        <div className={styles.kv}><span>Git 状态</span><strong>{status?.git?.has_git ? '已配置' : '未配置'}</strong></div>
        <div className={styles.kv}><span>远端检查</span><strong>{status?.git?.remote_check ?? '-'}</strong></div>
        <div className={styles.kv}><span>Deploy Key</span><strong>{status?.deploy_key?.exists ? '已生成' : '未生成'}</strong></div>
      </div>

      <div className={styles.keyBlock}>
        <div className={styles.panelHeader}>
          <div>
            <strong>Deploy Key 公钥</strong>
            <span>复制到 GitHub/GitLab 仓库的 Deploy Keys 后，服务器或节点即可拉取代码。</span>
          </div>
          <button className={styles.textBtn} type="button" onClick={createDeployKey} disabled={!canEdit || busy === 'key'}>
            {busy === 'key' ? '生成中' : '生成/刷新'}
          </button>
        </div>
        <textarea className={styles.monoArea} readOnly value={status?.deploy_key?.public_key ?? ''} placeholder="还没有生成 Deploy Key" />
        {status?.deploy_key?.github_deploy_keys_url && (
          <a className={styles.inlineLink} href={status.deploy_key.github_deploy_keys_url} target="_blank" rel="noreferrer">打开 Git 托管平台 Deploy Keys</a>
        )}
      </div>
      {!canEdit && <p className={styles.formHint}>当前角色可查看 Git 状态；配置仓库和生成密钥需要项目编辑权限。</p>}
      {message && <p className={styles.formSuccess}>{message}</p>}
      {error && <p className={styles.formError}>{error}</p>}
    </section>
  )
}

function gitPath(userId: string, projectId: string, action: 'status' | 'config' | 'deploy-key') {
  return `/api/user/${encodeURIComponent(userId)}/projects/${encodeURIComponent(projectId)}/git/${action}`
}

function errorMessage(err: unknown, fallback: string) {
  return (err as { message?: string })?.message ?? fallback
}
