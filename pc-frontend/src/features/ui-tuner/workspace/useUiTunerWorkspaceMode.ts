import { useCallback, useState } from 'react'
import type { SourcePreviewMode } from '../source-preview/types'
import { WORKSPACE_MODE_STORAGE_KEY } from './uiTunerWorkspaceState'

const MODES = new Set<SourcePreviewMode>(['headless', 'source', 'evidence'])

export function useUiTunerWorkspaceMode() {
  const [workspaceMode, setWorkspaceMode] = useState<SourcePreviewMode>(() => {
    const remembered = window.localStorage.getItem(WORKSPACE_MODE_STORAGE_KEY) as SourcePreviewMode | null
    return remembered && MODES.has(remembered) ? remembered : 'headless'
  })
  const changeWorkspaceMode = useCallback((mode: SourcePreviewMode) => {
    setWorkspaceMode(mode)
    window.localStorage.setItem(WORKSPACE_MODE_STORAGE_KEY, mode)
  }, [])
  return { workspaceMode, changeWorkspaceMode }
}
