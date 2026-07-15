import { useCallback, useEffect, useState } from 'react'

import type { DocumentAutomationMode } from './projectDocumentSections'

export const DEFAULT_DOCUMENT_AUTOMATION_MODE: DocumentAutomationMode = 'git_backed_full'

export const DOCUMENT_AUTOMATION_OPTIONS: Array<{
  value: DocumentAutomationMode
  label: string
  detail: string
}> = [
  {
    value: 'git_backed_full',
    label: 'Git 备份后 AI 完全整理（默认）',
    detail: '整理前备份、自动分类和调整路径、整理后提交',
  },
  {
    value: 'review_all',
    label: '每次审核',
    detail: 'AI 只准备操作，用户确认后才应用',
  },
  {
    value: 'suggestions_only',
    label: '仅生成建议',
    detail: '不应用分区，也不移动任何文件',
  },
]

export function useProjectDocumentAutomationPolicy(projectId: string) {
  const storageKey = `elon:project-docs:automation-mode:${projectId}`
  const [mode, setModeState] = useState<DocumentAutomationMode>(() => readMode(storageKey))

  useEffect(() => setModeState(readMode(storageKey)), [storageKey])

  const setMode = useCallback((value: DocumentAutomationMode) => {
    setModeState(value)
    try {
      window.localStorage.setItem(storageKey, value)
    } catch {
      // The selected mode remains active for this page when storage is unavailable.
    }
  }, [storageKey])

  return { mode, setMode }
}

function readMode(storageKey: string): DocumentAutomationMode {
  try {
    const value = window.localStorage.getItem(storageKey)
    if (value === 'git_backed_full' || value === 'review_all' || value === 'suggestions_only') return value
  } catch {
    // Use the trusted reversible default when browser storage is unavailable.
  }
  return DEFAULT_DOCUMENT_AUTOMATION_MODE
}
