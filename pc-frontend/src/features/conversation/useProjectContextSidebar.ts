import { useCallback, useEffect, useState } from 'react'
import { api } from '../../api/client'
import type { Project } from './types'

export type ProjectContextTab = 'project' | 'members'

export interface ProjectWorkspaceHealth {
  project?: {
    workspace_path?: string | null
    node_id?: string | null
  }
  node?: {
    node_id?: string | null
    device_name?: string | null
    online?: boolean
    cli_connected?: boolean
  }
  health_label?: string
  health_tone?: string
  recommended_action?: string
}

interface Options {
  project: Project | null
  onProjectChanged: () => Promise<void> | void
}

const LOGO_MAX_FILE_BYTES = 5 * 1024 * 1024
const LOGO_MAX_DATA_URL_LENGTH = 500_000
const PROJECT_TAB_STORAGE_PREFIX = 'elon.pc.projectContextTab.v1.'

export function useProjectContextSidebar({ project, onProjectChanged }: Options) {
  const projectId = project?.id ?? ''
  const [tab, setTabState] = useState<ProjectContextTab>(() => readStoredTab(projectId))
  const [health, setHealth] = useState<ProjectWorkspaceHealth | null>(null)
  const [healthLoading, setHealthLoading] = useState(false)
  const [healthError, setHealthError] = useState('')
  const [feedback, setFeedback] = useState('')
  const [logoBusy, setLogoBusy] = useState(false)

  useEffect(() => {
    setTabState(readStoredTab(projectId))
    setFeedback('')
  }, [projectId])

  const setTab = useCallback((next: ProjectContextTab) => {
    setTabState(next)
    if (!projectId || typeof window === 'undefined') return
    try {
      window.localStorage.setItem(`${PROJECT_TAB_STORAGE_PREFIX}${projectId}`, next)
    } catch {
      // 侧栏偏好仅做尽力保存。
    }
  }, [projectId])

  const loadHealth = useCallback(async () => {
    if (!projectId) {
      setHealth(null)
      setHealthError('')
      return
    }
    setHealthLoading(true)
    setHealthError('')
    try {
      const data = await api.get<ProjectWorkspaceHealth>(
        `/api/projects/${encodeURIComponent(projectId)}/workspace/health`,
      )
      setHealth(data)
    } catch (error) {
      setHealth(null)
      setHealthError(errorMessage(error, '暂时无法读取节点状态'))
    } finally {
      setHealthLoading(false)
    }
  }, [projectId])

  useEffect(() => {
    void loadHealth()
  }, [loadHealth, project?.updated_at])

  const copyText = useCallback(async (value: string, successMessage: string) => {
    const text = value.trim()
    if (!text) return
    try {
      await navigator.clipboard.writeText(text)
      setFeedback(successMessage)
    } catch {
      setFeedback('复制失败，请手动选择文本')
    }
  }, [])

  const updateLogo = useCallback(async (file: File) => {
    if (!projectId || logoBusy) return
    setLogoBusy(true)
    setFeedback('正在处理项目 Logo…')
    try {
      const iconDataUrl = await projectLogoDataUrl(file)
      await api.patch(`/api/projects/${encodeURIComponent(projectId)}/brand`, {
        icon_data_url: iconDataUrl,
      })
      await onProjectChanged()
      setFeedback('项目 Logo 已更新')
    } catch (error) {
      setFeedback(errorMessage(error, '项目 Logo 更新失败'))
    } finally {
      setLogoBusy(false)
    }
  }, [logoBusy, onProjectChanged, projectId])

  return {
    tab,
    setTab,
    health,
    healthLoading,
    healthError,
    feedback,
    logoBusy,
    copyText,
    updateLogo,
    reloadHealth: loadHealth,
  }
}

async function projectLogoDataUrl(file: File): Promise<string> {
  if (!['image/png', 'image/jpeg', 'image/webp'].includes(file.type)) {
    throw new Error('请选择 PNG、JPEG 或 WebP 图片')
  }
  if (file.size > LOGO_MAX_FILE_BYTES) throw new Error('Logo 图片不能超过 5 MB')

  const image = await loadImage(file)
  const sourceSize = Math.min(image.naturalWidth, image.naturalHeight)
  if (sourceSize < 64) throw new Error('Logo 图片至少需要 64×64 像素')
  const sourceX = Math.max(0, (image.naturalWidth - sourceSize) / 2)
  const sourceY = Math.max(0, (image.naturalHeight - sourceSize) / 2)
  const canvas = document.createElement('canvas')
  canvas.width = 320
  canvas.height = 320
  const context = canvas.getContext('2d')
  if (!context) throw new Error('当前浏览器无法处理图片')
  context.drawImage(image, sourceX, sourceY, sourceSize, sourceSize, 0, 0, 320, 320)

  for (const quality of [0.9, 0.82, 0.72, 0.62]) {
    const dataUrl = canvas.toDataURL('image/webp', quality)
    if (dataUrl.length <= LOGO_MAX_DATA_URL_LENGTH) return dataUrl
  }
  throw new Error('图片处理后仍然过大，请换一张更简单的图片')
}

function loadImage(file: File): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const objectUrl = URL.createObjectURL(file)
    const image = new Image()
    image.onload = () => {
      URL.revokeObjectURL(objectUrl)
      resolve(image)
    }
    image.onerror = () => {
      URL.revokeObjectURL(objectUrl)
      reject(new Error('无法读取这张图片'))
    }
    image.src = objectUrl
  })
}

function readStoredTab(projectId: string): ProjectContextTab {
  if (!projectId || typeof window === 'undefined') return 'project'
  try {
    return window.localStorage.getItem(`${PROJECT_TAB_STORAGE_PREFIX}${projectId}`) === 'members'
      ? 'members'
      : 'project'
  } catch {
    return 'project'
  }
}

function errorMessage(error: unknown, fallback: string): string {
  return error instanceof Error && error.message ? error.message : fallback
}
