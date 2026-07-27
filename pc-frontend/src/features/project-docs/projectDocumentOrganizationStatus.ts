export interface DocumentOrganizationTrackingRuntime {
  enabled: boolean
  adminUrl: string
  projectRoot: string
}

export interface DocumentOrganizationTraceEvent {
  stage: string
  status: string
  label: string
  detail: string
  at: number
}

export interface DocumentOrganizationTraceError {
  code: string
  message: string
  recovery: string
  at: number
}

export interface DocumentOrganizationTrace {
  version: 1
  operation_id: string
  status: 'pending' | 'running' | 'awaiting_review' | 'succeeded' | 'failed'
  current_stage: string
  created_at: number
  updated_at: number
  task_id?: string
  session_id?: string
  catalog_revision?: string
  manifest_revision?: string
  discussion_graph_revision?: string
  suggestions_revision?: string
  git_baseline_commit?: string
  git_result_commit?: string
  documents_cataloged: number
  ambiguous_documents: number
  documents_read: number
  estimated_tokens_used: number
  events: DocumentOrganizationTraceEvent[]
  error?: DocumentOrganizationTraceError
}

export interface DocumentOrganizationTraceResponse {
  ok: boolean
  trace: DocumentOrganizationTrace
}

export function parseDocumentOrganizationTrace(value: unknown): DocumentOrganizationTrace | null {
  if (!value || typeof value !== 'object') return null
  const candidate = value as Partial<DocumentOrganizationTrace>
  const operationId = clean(candidate.operation_id)
  const status = clean(candidate.status)
  if (!operationId || !['pending', 'running', 'awaiting_review', 'succeeded', 'failed'].includes(status)) {
    return null
  }
  const events = Array.isArray(candidate.events)
    ? candidate.events.slice(-40).flatMap((entry) => parseEvent(entry) ?? [])
    : []
  const error = parseError(candidate.error)
  return {
    version: 1,
    operation_id: operationId,
    status: status as DocumentOrganizationTrace['status'],
    current_stage: clean(candidate.current_stage) || 'requested',
    created_at: safeNumber(candidate.created_at),
    updated_at: safeNumber(candidate.updated_at),
    task_id: optionalText(candidate.task_id),
    session_id: optionalText(candidate.session_id),
    catalog_revision: optionalText(candidate.catalog_revision),
    manifest_revision: optionalText(candidate.manifest_revision),
    discussion_graph_revision: optionalText(candidate.discussion_graph_revision),
    suggestions_revision: optionalText(candidate.suggestions_revision),
    git_baseline_commit: optionalText(candidate.git_baseline_commit),
    git_result_commit: optionalText(candidate.git_result_commit),
    documents_cataloged: safeNumber(candidate.documents_cataloged),
    ambiguous_documents: safeNumber(candidate.ambiguous_documents),
    documents_read: safeNumber(candidate.documents_read),
    estimated_tokens_used: safeNumber(candidate.estimated_tokens_used),
    events,
    error: error ?? undefined,
  }
}

export function shouldPollDocumentOrganization(trace: DocumentOrganizationTrace | null): boolean {
  return !!trace && ['pending', 'running'].includes(trace.status)
}

export function organizationTraceStorageKey(projectId: string): string {
  return `elon_project_docs_operation:${projectId}`
}

export function newDocumentOrganizationOperationId(): string {
  const uuid = globalThis.crypto?.randomUUID?.().replace(/-/g, '')
  return `docs_${uuid || `${Date.now()}_${Math.random().toString(16).slice(2)}`}`
}

function parseEvent(value: unknown): DocumentOrganizationTraceEvent | null {
  if (!value || typeof value !== 'object') return null
  const event = value as Partial<DocumentOrganizationTraceEvent>
  const stage = clean(event.stage)
  const label = clean(event.label)
  if (!stage || !label) return null
  return {
    stage,
    status: clean(event.status),
    label,
    detail: clean(event.detail),
    at: safeNumber(event.at),
  }
}

function parseError(value: unknown): DocumentOrganizationTraceError | null {
  if (!value || typeof value !== 'object') return null
  const error = value as Partial<DocumentOrganizationTraceError>
  const code = clean(error.code)
  if (!code) return null
  return {
    code,
    message: clean(error.message),
    recovery: clean(error.recovery),
    at: safeNumber(error.at),
  }
}

function clean(value: unknown): string {
  return String(value ?? '').trim()
}

function optionalText(value: unknown): string | undefined {
  return clean(value) || undefined
}

function safeNumber(value: unknown): number {
  const number = Number(value)
  return Number.isFinite(number) && number >= 0 ? Math.floor(number) : 0
}
