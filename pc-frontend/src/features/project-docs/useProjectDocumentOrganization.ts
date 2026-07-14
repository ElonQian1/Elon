import { useCallback, useEffect, useState } from 'react'

import { api, type ApiError } from '../../api/client'
import { nodeApi } from '../node/localNodeApi'
import type { DocumentFile } from './projectDocumentModel'
import {
  newDocumentOrganizationOperationId,
  organizationTraceStorageKey,
  parseDocumentOrganizationTrace,
  shouldPollDocumentOrganization,
  type DocumentOrganizationTrace,
  type DocumentOrganizationTraceResponse,
  type DocumentOrganizationTrackingRuntime,
} from './projectDocumentOrganizationStatus'
import {
  createCustomSection,
  customSectionKey,
  EMPTY_SECTION_MANIFEST,
  ORGANIZATION_SUGGESTIONS_PATH,
  parseOrganizationSuggestions,
  parseSectionManifest,
  SECTION_CONFIG_PATH,
  serializeProjectDocumentJson,
  type DocumentOrganizationSuggestions,
  type DocumentSectionManifest,
} from './projectDocumentSections'

interface PersistedJson<T> {
  value: T
  revision?: string
}

interface AppliedOrganizationResponse {
  manifest: DocumentSectionManifest
  suggestions: DocumentOrganizationSuggestions
  manifest_revision?: string
  suggestions_revision?: string
}

export interface AppliedFileOperationResult {
  id: string
  kind: 'rename' | 'move'
  source_path: string
  target_path: string
  already_applied: boolean
}

interface AppliedFileOperationsResponse {
  operations: AppliedFileOperationResult[]
  manifest: DocumentSectionManifest
  suggestions: DocumentOrganizationSuggestions
  manifest_revision?: string
  suggestions_revision?: string
  catalog_revision: string
}

export function useProjectDocumentOrganization(
  projectId: string,
  trackingRuntime: DocumentOrganizationTrackingRuntime,
) {
  const [manifestFile, setManifestFile] = useState<PersistedJson<DocumentSectionManifest>>({
    value: EMPTY_SECTION_MANIFEST,
  })
  const [suggestionsFile, setSuggestionsFile] = useState<PersistedJson<DocumentOrganizationSuggestions | null>>({
    value: null,
  })
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [trace, setTrace] = useState<DocumentOrganizationTrace | null>(null)
  const [trackingError, setTrackingError] = useState('')

  const load = useCallback(async () => {
    setLoading(true)
    setError('')
    const [manifest, suggestions] = await Promise.all([
      readJsonFile(projectId, SECTION_CONFIG_PATH),
      readJsonFile(projectId, ORGANIZATION_SUGGESTIONS_PATH),
    ])
    setManifestFile({
      value: manifest ? parseSectionManifest(manifest.content) : { version: 1, sections: [], assignments: {} },
      revision: manifest?.revision,
    })
    setSuggestionsFile({
      value: suggestions ? parseOrganizationSuggestions(suggestions.content) : null,
      revision: suggestions?.revision,
    })
    setLoading(false)
  }, [projectId])

  useEffect(() => {
    load().catch((reason) => {
      setError(errorMessage(reason, '读取项目分区配置失败'))
      setLoading(false)
    })
  }, [load])

  const trackingAvailable = trackingRuntime.enabled && !!trackingRuntime.projectRoot.trim()
  const trackingRequest = useCallback(async (
    path: string,
    body: Record<string, unknown>,
  ) => {
    const response = await nodeApi<DocumentOrganizationTraceResponse>(
      trackingRuntime.adminUrl,
      path,
      { method: 'POST', body: JSON.stringify({ project_root: trackingRuntime.projectRoot, ...body }) },
    )
    const parsed = parseDocumentOrganizationTrace(response.trace)
    if (!parsed) throw new Error('本机节点返回了无效的文档整理状态')
    setTrace(parsed)
    setTrackingError('')
    return parsed
  }, [trackingRuntime.adminUrl, trackingRuntime.projectRoot])

  const loadStatus = useCallback(async () => {
    if (!trackingAvailable) return null
    const operationId = readStoredOperation(projectId)
    if (!operationId) return null
    try {
      return await trackingRequest('/api/project-docs/organization/status', { operation_id: operationId })
    } catch (reason) {
      setTrackingError(errorMessage(reason, '读取 MCP 整理进度失败'))
      return null
    }
  }, [projectId, trackingAvailable, trackingRequest])

  useEffect(() => {
    loadStatus()
  }, [loadStatus])

  useEffect(() => {
    if (!shouldPollDocumentOrganization(trace)) return
    const timer = window.setInterval(loadStatus, 1_500)
    return () => window.clearInterval(timer)
  }, [loadStatus, trace])

  useEffect(() => {
    if (trace?.current_stage === 'awaiting_review' || trace?.current_stage === 'applied') {
      load().catch((reason) => setError(errorMessage(reason, '刷新 AI 整理建议失败')))
    }
  }, [load, trace?.current_stage])

  const startRun = useCallback(async () => {
    if (!trackingAvailable) return undefined
    const operationId = newDocumentOrganizationOperationId()
    const started = await trackingRequest('/api/project-docs/organization/start', {
      operation_id: operationId,
    })
    storeOperation(projectId, started.operation_id)
    return started.operation_id
  }, [projectId, trackingAvailable, trackingRequest])

  const markDispatched = useCallback(async (operationId?: string, taskId?: string) => {
    if (!trackingAvailable || !operationId) return
    try {
      await trackingRequest('/api/project-docs/organization/dispatched', {
        operation_id: operationId,
        task_id: taskId,
      })
    } catch (reason) {
      setTrackingError(errorMessage(reason, '记录 AI 任务发送状态失败'))
    }
  }, [trackingAvailable, trackingRequest])

  const markFailed = useCallback(async (operationId: string | undefined, message: string) => {
    if (!trackingAvailable || !operationId) return
    try {
      await trackingRequest('/api/project-docs/organization/fail', {
        operation_id: operationId,
        error_code: 'dispatch_failed',
        message,
        recovery: '确认本机节点、项目目录和 AI 开发频道可用后重试。',
      })
    } catch (reason) {
      setTrackingError(errorMessage(reason, '记录 AI 整理失败状态失败'))
    }
  }, [trackingAvailable, trackingRequest])

  const saveManifest = useCallback(async (value: DocumentSectionManifest) => {
    const saved = await writeJsonFile(projectId, SECTION_CONFIG_PATH, value, manifestFile.revision)
    setManifestFile({ value, revision: saved.revision })
    return value
  }, [manifestFile.revision, projectId])

  const addSection = useCallback(async (label: string) => {
    const section = createCustomSection(label, manifestFile.value.sections)
    await saveManifest({
      ...manifestFile.value,
      sections: [...manifestFile.value.sections, section],
    })
    return customSectionKey(section.id)
  }, [manifestFile.value, saveManifest])

  const removeSection = useCallback(async (sectionKey: string) => {
    const id = sectionKey.replace(/^custom:/, '')
    const assignments = Object.fromEntries(
      Object.entries(manifestFile.value.assignments).filter(([, assigned]) => assigned !== sectionKey),
    )
    await saveManifest({
      version: 1,
      sections: manifestFile.value.sections.filter((section) => section.id !== id),
      assignments,
    })
  }, [manifestFile.value, saveManifest])

  const assignDocument = useCallback(async (path: string, sectionKey: string) => {
    const normalized = normalizePath(path)
    const assignments = { ...manifestFile.value.assignments }
    if (sectionKey) assignments[normalized] = sectionKey
    else delete assignments[normalized]
    const nextManifest = {
      ...manifestFile.value,
      assignments,
    }
    await saveManifest(nextManifest)
    return nextManifest
  }, [manifestFile.value, saveManifest])

  const applySuggestions = useCallback(async (catalogRevision: string) => {
    const suggestions = suggestionsFile.value
    if (!suggestions || suggestions.status !== 'ready') return manifestFile.value
    const result = await api.post<AppliedOrganizationResponse>(
      `/api/projects/${encodeURIComponent(projectId)}/docs/organization/apply`,
      {
        reviewed: true,
        expected_catalog_revision: catalogRevision,
        expected_manifest_revision: manifestFile.revision,
        expected_suggestions_revision: suggestionsFile.revision,
      },
    )
    setManifestFile({ value: result.manifest, revision: result.manifest_revision })
    setSuggestionsFile({ value: result.suggestions, revision: result.suggestions_revision })
    const operationId = trace?.operation_id
    if (trackingAvailable && operationId) {
      try {
        await trackingRequest('/api/project-docs/organization/applied', {
          operation_id: operationId,
          manifest_revision: result.manifest_revision,
          suggestions_revision: result.suggestions_revision,
        })
      } catch (reason) {
        setTrackingError(errorMessage(reason, '分区已应用，但记录观测状态失败'))
      }
    }
    return result.manifest
  }, [manifestFile.revision, manifestFile.value, projectId, suggestionsFile.revision, suggestionsFile.value, trace?.operation_id, trackingAvailable, trackingRequest])

  const applyFileOperations = useCallback(async (input: {
    catalogRevision: string
    operationIds: string[]
    allowRename: boolean
    allowMove: boolean
  }) => {
    if (!trackingAvailable) throw new Error('实体文档整理必须连接项目本机节点')
    setError('')
    try {
      const result = await nodeApi<AppliedFileOperationsResponse>(
        trackingRuntime.adminUrl,
        '/api/project-docs/organization/apply-files',
        {
          method: 'POST',
          body: JSON.stringify({
            project_root: trackingRuntime.projectRoot,
            reviewed: true,
            operation_ids: input.operationIds,
            allow_rename: input.allowRename,
            allow_move: input.allowMove,
            expected_catalog_revision: input.catalogRevision,
            expected_manifest_revision: manifestFile.revision,
            expected_suggestions_revision: suggestionsFile.revision,
          }),
        },
      )
      setManifestFile({ value: result.manifest, revision: result.manifest_revision })
      setSuggestionsFile({ value: result.suggestions, revision: result.suggestions_revision })
      return result
    } catch (reason) {
      setError(errorMessage(reason, '实体文档整理失败'))
      throw reason
    }
  }, [manifestFile.revision, suggestionsFile.revision, trackingAvailable, trackingRuntime.adminUrl, trackingRuntime.projectRoot])

  const reload = useCallback(async () => {
    await Promise.all([load(), loadStatus()])
  }, [load, loadStatus])

  return {
    manifest: manifestFile.value,
    suggestions: suggestionsFile.value,
    loading,
    error,
    trace,
    trackingAvailable,
    trackingError,
    reload,
    startRun,
    markDispatched,
    markFailed,
    addSection,
    removeSection,
    assignDocument,
    applySuggestions,
    applyFileOperations,
  }
}

function readStoredOperation(projectId: string) {
  try {
    return window.localStorage.getItem(organizationTraceStorageKey(projectId)) ?? ''
  } catch {
    return ''
  }
}

function storeOperation(projectId: string, operationId: string) {
  try {
    window.localStorage.setItem(organizationTraceStorageKey(projectId), operationId)
  } catch {
    // The current page still keeps the trace even when storage is blocked.
  }
}

async function readJsonFile(projectId: string, path: string) {
  try {
    return await api.get<DocumentFile>(documentFileUrl(projectId, path))
  } catch (error) {
    if ((error as ApiError)?.status === 404) return null
    throw error
  }
}

async function writeJsonFile(
  projectId: string,
  path: string,
  value: unknown,
  expectedRevision?: string,
) {
  return api.put<{ revision: string; byte_len: number }>(
    `/api/projects/${encodeURIComponent(projectId)}/docs/file`,
    {
      path,
      content: serializeProjectDocumentJson(value),
      expected_revision: expectedRevision,
    },
  )
}

function documentFileUrl(projectId: string, path: string) {
  return `/api/projects/${encodeURIComponent(projectId)}/docs/file?path=${encodeURIComponent(path)}`
}

function normalizePath(path: string) {
  return path.trim().replace(/\\/g, '/')
}

function errorMessage(error: unknown, fallback: string) {
  return (error as { message?: string })?.message ?? fallback
}
