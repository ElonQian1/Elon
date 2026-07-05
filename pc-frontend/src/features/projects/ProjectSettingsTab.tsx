import { useEffect, useMemo, useState } from 'react'
import { api } from '../../api/client'
import type { Project, ProjectSpace } from '../conversation/types'
import styles from './ProjectDetailPage.module.css'

interface Props {
  projectId: string
  project?: Project | null
  space?: ProjectSpace | null
  canEditProject: boolean
  canManageSettings: boolean
  canUpdateBrand: boolean
  canDeleteProject: boolean
  onChanged: () => Promise<void> | void
  onDeleted: () => void
}

export default function ProjectSettingsTab({
  projectId,
  project,
  space,
  canEditProject,
  canManageSettings,
  canUpdateBrand,
  canDeleteProject,
  onChanged,
  onDeleted,
}: Props) {
  const source = space?.project ?? project
  const [displayName, setDisplayName] = useState(source?.display_name ?? '')
  const [description, setDescription] = useState(source?.description ?? '')
  const [iconDataUrl, setIconDataUrl] = useState(source?.icon_data_url ?? '')
  const [galleryImages, setGalleryImages] = useState<string[]>(() => normalizeGallery(source?.gallery_images))
  const [isPublic, setIsPublic] = useState(project?.is_public ?? false)
  const [joinMode, setJoinMode] = useState(project?.join_mode ?? 'open')
  const [deleteConfirm, setDeleteConfirm] = useState('')
  const [busy, setBusy] = useState<'profile' | 'visibility' | 'delete' | ''>('')
  const [message, setMessage] = useState('')
  const [error, setError] = useState('')
  const projectName = project?.name ?? source?.name ?? projectId

  useEffect(() => {
    const next = space?.project ?? project
    setDisplayName(next?.display_name ?? '')
    setDescription(next?.description ?? '')
    setIconDataUrl(next?.icon_data_url ?? '')
    setGalleryImages(normalizeGallery(next?.gallery_images))
    setIsPublic(project?.is_public ?? false)
    setJoinMode(project?.join_mode ?? 'open')
  }, [project?.id, project?.updated_at, space?.project?.updated_at])

  const canSaveProfile = useMemo(() => canEditProject && !!projectId, [canEditProject, projectId])

  async function saveProfile(event: React.FormEvent) {
    event.preventDefault()
    if (!canSaveProfile) return
    setBusy('profile')
    setMessage('')
    setError('')
    try {
      await api.patch(`/api/projects/${encodeURIComponent(projectId)}/space/description`, {
        description,
      })
      const brandPatch: Record<string, string | null> = {}
      if (displayName !== (source?.display_name ?? '')) brandPatch.display_name = displayName.trim() || null
      if (iconDataUrl !== (source?.icon_data_url ?? '')) brandPatch.icon_data_url = iconDataUrl.trim() || null
      if (Object.keys(brandPatch).length > 0) {
        if (!canUpdateBrand) throw new Error('只有项目创建者可以修改展示名和图标')
        await api.patch(`/api/projects/${encodeURIComponent(projectId)}/brand`, brandPatch)
      }
      for (let slot = 0; slot < galleryImages.length; slot += 1) {
        const current = source?.gallery_images?.[slot] ?? ''
        const next = galleryImages[slot]?.trim() ?? ''
        if (next !== current) {
          await api.patch(`/api/projects/${encodeURIComponent(projectId)}/space/gallery-image`, {
            slot,
            image_url: next || null,
          })
        }
      }
      setMessage('项目资料已保存')
      await onChanged()
    } catch (err) {
      setError(errorMessage(err, '项目资料保存失败'))
    } finally {
      setBusy('')
    }
  }

  async function saveVisibility(event: React.FormEvent) {
    event.preventDefault()
    if (!canManageSettings) return
    setBusy('visibility')
    setMessage('')
    setError('')
    try {
      await api.patch(`/api/projects/${encodeURIComponent(projectId)}/visibility`, {
        is_public: isPublic,
        join_mode: joinMode,
      })
      setMessage('项目可见性已保存')
      await onChanged()
    } catch (err) {
      setError(errorMessage(err, '项目可见性保存失败'))
    } finally {
      setBusy('')
    }
  }

  async function deleteProject() {
    if (!canDeleteProject || deleteConfirm !== projectName) return
    if (!window.confirm(`确认永久删除项目“${projectName}”？此操作不可恢复。`)) return
    setBusy('delete')
    setMessage('')
    setError('')
    try {
      await api.delete(`/api/projects/${encodeURIComponent(projectId)}`)
      onDeleted()
    } catch (err) {
      setError(errorMessage(err, '删除项目失败'))
      setBusy('')
    }
  }

  return (
    <div className={styles.managementStack}>
      <section className={styles.panel}>
        <div className={styles.panelHeader}>
          <div>
            <strong>项目资料</strong>
            <span>这些内容用于项目首页、项目广场和 APK 展示。</span>
          </div>
        </div>
        <form className={styles.settingsForm} onSubmit={saveProfile}>
          <label className={styles.field}>
            <span>展示名</span>
            <input value={displayName} onChange={(event) => setDisplayName(event.target.value)} placeholder={projectName} disabled={!canSaveProfile || !canUpdateBrand || busy === 'profile'} />
          </label>
          <label className={styles.fieldWide}>
            <span>简介</span>
            <textarea value={description} onChange={(event) => setDescription(event.target.value)} placeholder="项目用途、用户和当前状态" disabled={!canSaveProfile || busy === 'profile'} />
          </label>
          <label className={styles.fieldWide}>
            <span>图标 Data URL</span>
            <textarea value={iconDataUrl} onChange={(event) => setIconDataUrl(event.target.value)} placeholder="data:image/png;base64,..." disabled={!canSaveProfile || !canUpdateBrand || busy === 'profile'} />
          </label>
          {!canUpdateBrand && <p className={styles.formHint}>展示名和图标只有项目创建者可以修改；简介和项目图片可由项目编辑者维护。</p>}
          <div className={styles.galleryGrid}>
            {galleryImages.map((image, index) => (
              <label className={styles.field} key={index}>
                <span>项目图片 {index + 1}</span>
                <input value={image} onChange={(event) => updateGalleryImage(setGalleryImages, index, event.target.value)} placeholder="https:// 或 data:image/ 地址" disabled={!canSaveProfile || busy === 'profile'} />
              </label>
            ))}
          </div>
          <button className={styles.primaryBtn} type="submit" disabled={!canSaveProfile || busy === 'profile'}>
            {busy === 'profile' ? '保存中' : '保存资料'}
          </button>
        </form>
      </section>

      <section className={styles.panel}>
        <div className={styles.panelHeader}>
          <div>
            <strong>可见性</strong>
            <span>控制项目是否进入项目广场，以及用户加入方式。</span>
          </div>
        </div>
        <form className={styles.managementForm} onSubmit={saveVisibility}>
          <label className={styles.toggleField}>
            <input type="checkbox" checked={isPublic} onChange={(event) => setIsPublic(event.target.checked)} disabled={!canManageSettings || busy === 'visibility'} />
            <span>公开到项目广场</span>
          </label>
          <label className={styles.field}>
            <span>加入方式</span>
            <select value={joinMode} onChange={(event) => setJoinMode(event.target.value)} disabled={!canManageSettings || busy === 'visibility'}>
              <option value="open">开放加入</option>
              <option value="approval">申请审批</option>
              <option value="invite">仅邀请</option>
              <option value="readonly">只读公开</option>
            </select>
          </label>
          <button className={styles.primaryBtn} type="submit" disabled={!canManageSettings || busy === 'visibility'}>
            {busy === 'visibility' ? '保存中' : '保存可见性'}
          </button>
        </form>
      </section>

      <section className={styles.dangerPanel}>
        <div className={styles.panelHeader}>
          <div>
            <strong>删除项目</strong>
            <span>会清理服务端记录和可控的工作区文件。</span>
          </div>
        </div>
        <label className={styles.field}>
          <span>输入项目名确认：{projectName}</span>
          <input value={deleteConfirm} onChange={(event) => setDeleteConfirm(event.target.value)} disabled={!canDeleteProject || busy === 'delete'} />
        </label>
        <button className={styles.textBtn} data-danger="true" type="button" disabled={!canDeleteProject || deleteConfirm !== projectName || busy === 'delete'} onClick={deleteProject}>
          {busy === 'delete' ? '删除中' : '永久删除项目'}
        </button>
        {!canDeleteProject && <p className={styles.formHint}>只有项目创建者可以删除项目。</p>}
      </section>
      {message && <p className={styles.formSuccess}>{message}</p>}
      {error && <p className={styles.formError}>{error}</p>}
    </div>
  )
}

function normalizeGallery(images?: string[]) {
  const values = [...(images ?? [])]
  while (values.length < 4) values.push('')
  return values.slice(0, 4)
}

function updateGalleryImage(setter: React.Dispatch<React.SetStateAction<string[]>>, index: number, value: string) {
  setter((current) => current.map((item, itemIndex) => (itemIndex === index ? value : item)))
}

function errorMessage(err: unknown, fallback: string) {
  return (err as { message?: string })?.message ?? fallback
}
