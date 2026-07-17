import { useCallback, useEffect, useState } from 'react'

import { api, type ApiError } from '../../api/client'
import { nodeApi } from '../node/localNodeApi'
import type { DocumentFile } from './projectDocumentModel'
import {
  assignDocuments,
  createSectionInManifest,
  recordManifestChange,
  removeSectionTree,
} from './projectDocumentCommands'
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
  EMPTY_SECTION_MANIFEST,
  ORGANIZATION_SUGGESTIONS_PATH,
  parseOrganizationSuggestions,
  parseSectionManifest,
  SECTION_CONFIG_PATH,
  serializeProjectDocumentJson,
  type DocumentOrganizationSuggestions,
  type DocumentAutomationMode,
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
  git_baseline_commit?: string
  git_result_commit?: string
  git_document_transaction_complete?: boolean
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
  git_baseline_commit?: string
  git_result_commit?: string
  git_document_transaction_complete?: boolean
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
  const [undoStack, setUndoStack] = useState<Array<{ manifest: DocumentSectionManifest; label: string }>>([])

  useEffect(() => setUndoStack([]), [projectId])

  const load = useCallback(async () => {
    setLoading(true)
    setError('')
    const [manifest, suggestions] = await Promise.all([
      readJsonFile(projectId, SECTION_CONFIG_PATH),
      readJsonFile(projectId, ORGANIZATION_SUGGESTIONS_PATH),
    ])
    setManifestFile({
      value: manifest ? parseSectionManifest(manifest.content) : parseSectionManifest(''),
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

  const saveManifest = useCallback(async (value: DocumentSectionManifest, undoLabel = '更新项目知识架构') => {
    const saved = await writeJsonFile(projectId, SECTION_CONFIG_PATH, value, manifestFile.revision)
    setUndoStack((current) => [...current, { manifest: manifestFile.value, label: undoLabel }].slice(-20))
    setManifestFile({ value, revision: saved.revision })
    return value
  }, [manifestFile.revision, manifestFile.value, projectId])

  const applyManifest = useCallback(async (value: DocumentSectionManifest, undoLabel?: string) => {
    const label = undoLabel ?? value.audit_log[value.audit_log.length - 1]?.summary ?? '更新项目知识架构'
    return saveManifest(value, label)
  }, [saveManifest])

  const undoLastChange = useCallback(async () => {
    const previous = undoStack[undoStack.length - 1]
    if (!previous) return null
    const restored = recordManifestChange(
      { ...previous.manifest, audit_log: manifestFile.value.audit_log },
      'history.undo',
      SECTION_CONFIG_PATH,
      `撤销：${previous.label}`,
    )
    const saved = await writeJsonFile(projectId, SECTION_CONFIG_PATH, restored, manifestFile.revision)
    setManifestFile({ value: restored, revision: saved.revision })
    setUndoStack((current) => current.slice(0, -1))
    return restored
  }, [manifestFile.revision, projectId, undoStack])

  const addSection = useCallback(async (
    label: string,
    parentId = '',
    appearance?: { detail?: string; color?: string; icon?: string },
  ) => {
    const created = createSectionInManifest(manifestFile.value, label, parentId, appearance)
    await saveManifest(created.manifest, created.manifest.audit_log[created.manifest.audit_log.length - 1]?.summary)
    return created.key
  }, [manifestFile.value, saveManifest])

  const removeSection = useCallback(async (sectionKey: string) => {
    await applyManifest(removeSectionTree(manifestFile.value, sectionKey))
  }, [applyManifest, manifestFile.value])

  const assignDocument = useCallback(async (
    path: string,
    sectionKey: string,
    facet: 'knowledge' | 'governance',
  ) => {
    const nextManifest = assignDocuments(manifestFile.value, [path], sectionKey, facet)
    await applyManifest(nextManifest)
    return nextManifest
  }, [applyManifest, manifestFile.value])

  const assignManyDocuments = useCallback(async (
    paths: string[],
    sectionKey: string,
    facet: 'knowledge' | 'governance',
  ) => applyManifest(assignDocuments(manifestFile.value, paths, sectionKey, facet)), [applyManifest, manifestFile.value])

  const setProfile = useCallback(async (profile: string) => saveManifest(recordManifestChange({
    ...manifestFile.value,
    profile,
  }, 'profile.set', profile, `切换项目知识模板为 ${profile}`), '切换项目知识模板'), [manifestFile.value, saveManifest])

  const applySuggestions = useCallback(async (
    catalogRevision: string,
    authorizationMode: DocumentAutomationMode,
  ) => {
    const suggestions = suggestionsFile.value
    if (!suggestions || suggestions.status !== 'ready') return null
    if (authorizationMode === 'git_backed_full' && !trackingAvailable) {
      throw new Error('Git 备份后完全整理必须连接项目本机节点')
    }
    const request = {
      authorization_mode: authorizationMode,
      reviewed: authorizationMode === 'review_all',
      expected_catalog_revision: authorizationMode === 'git_backed_full'
        ? trace?.catalog_revision || catalogRevision
        : catalogRevision,
      expected_manifest_revision: manifestFile.revision,
      expected_suggestions_revision: suggestionsFile.revision,
    }
    const result = authorizationMode === 'git_backed_full'
      ? await nodeApi<AppliedOrganizationResponse>(
        trackingRuntime.adminUrl,
        '/api/project-docs/organization/apply-suggestions',
        { method: 'POST', body: JSON.stringify({ project_root: trackingRuntime.projectRoot, ...request }) },
      )
      : await api.post<AppliedOrganizationResponse>(
        `/api/projects/${encodeURIComponent(projectId)}/docs/organization/apply`,
        request,
      )
    setManifestFile({ value: result.manifest, revision: result.manifest_revision })
    setSuggestionsFile({ value: result.suggestions, revision: result.suggestions_revision })
    const operationId = trace?.operation_id
    if (authorizationMode !== 'git_backed_full' && trackingAvailable && operationId) {
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
    if (authorizationMode === 'git_backed_full') await loadStatus()
    return result
  }, [loadStatus, manifestFile.revision, projectId, suggestionsFile.revision, suggestionsFile.value, trace?.catalog_revision, trace?.operation_id, trackingAvailable, trackingRequest, trackingRuntime.adminUrl, trackingRuntime.projectRoot])

  const applyFileOperations = useCallback(async (input: {
    catalogRevision: string
    authorizationMode: DocumentAutomationMode
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
            authorization_mode: input.authorizationMode,
            reviewed: input.authorizationMode === 'review_all',
            operation_ids: input.operationIds,
            allow_rename: input.authorizationMode === 'review_all' && input.allowRename,
            allow_move: input.authorizationMode === 'review_all' && input.allowMove,
            expected_catalog_revision: input.catalogRevision,
            expected_manifest_revision: manifestFile.revision,
            expected_suggestions_revision: suggestionsFile.revision,
            git_baseline_commit: input.authorizationMode === 'git_backed_full'
              ? trace?.git_baseline_commit
              : undefined,
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
  }, [manifestFile.revision, suggestionsFile.revision, trace?.git_baseline_commit, trackingAvailable, trackingRuntime.adminUrl, trackingRuntime.projectRoot])

  const reload = useCallback(async () => {
    await Promise.all([load(), loadStatus()])
  }, [load, loadStatus])

  return {
    manifest: manifestFile.value,
    manifestRevision: manifestFile.revision,
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
    applyManifest,
    undoLastChange,
    canUndo: undoStack.length > 0,
    lastUndoLabel: undoStack[undoStack.length - 1]?.label ?? '',
    addSection,
    removeSection,
    assignDocument,
    assignManyDocuments,
    setProfile,
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

function errorMessage(error: unknown, fallback: string) {
  return (error as { message?: string })?.message ?? fallback
}
