import { nodeApi } from '../node/localNodeApi'

export type NativeContextCandidateStatus = 'pending' | 'reviewed' | 'rejected' | 'applied'
export type NativeContextReviewAction = 'accept' | 'reject' | 'restore'
export type NativeContextAuthorizationMode = 'git_backed_full' | 'trusted_reversible' | 'review_all' | 'suggestions_only'

export interface ProjectContextEvidence {
  path: string
  content_hash: string
  locator: string
  evidence_kind: 'source' | 'test' | 'document' | 'configuration'
}

export interface ProjectContextMemory {
  candidate_id: string
  summary: string
  topics: string[]
  evidence: ProjectContextEvidence[]
  reviewed_at: string
}

export interface NativeContextCandidate extends ProjectContextMemory {
  status: NativeContextCandidateStatus
  producer: string
  created_at_ms: number
  updated_at_ms: number
  evidence_current: boolean
}

export interface NativeContextCandidatePage {
  status: NativeContextCandidateStatus | 'all'
  counts: Record<NativeContextCandidateStatus, number>
  pagination: {
    offset: number
    limit: number
    returned: number
    total: number
    next_offset?: number
  }
  candidates: NativeContextCandidate[]
}

interface NativeContextEnvelope<T> {
  ok: boolean
  result?: T
  error?: string
}

export function sanitizeProjectContextMemories(value: unknown): ProjectContextMemory[] {
  if (!Array.isArray(value)) return []
  const unique = new Map<string, ProjectContextMemory>()
  value.slice(0, 64).forEach((entry) => {
    const memory = sanitizeMemory(entry)
    if (memory) unique.set(memory.candidate_id, memory)
  })
  return [...unique.values()]
}

export async function listNativeContextCandidates(input: {
  adminUrl: string
  projectRoot: string
  status: NativeContextCandidateStatus
  offset: number
  limit?: number
}): Promise<NativeContextCandidatePage> {
  const envelope = await nodeApi<NativeContextEnvelope<unknown>>(
    input.adminUrl,
    '/api/project-docs/native-context/candidates',
    {
      method: 'POST',
      body: JSON.stringify({
        project_root: input.projectRoot,
        status: input.status,
        offset: input.offset,
        limit: input.limit ?? 10,
      }),
    },
  )
  if (!envelope.ok || !envelope.result) throw new Error(envelope.error || '读取原生理解候选失败')
  return sanitizeCandidatePage(envelope.result, input.status, input.offset, input.limit ?? 10)
}

export async function reviewNativeContextCandidates(input: {
  adminUrl: string
  projectRoot: string
  candidateIds: string[]
  action: NativeContextReviewAction
  authorizationMode: NativeContextAuthorizationMode
  catalogRevision?: string
  suggestionsRevision?: string
}): Promise<Record<string, unknown>> {
  const envelope = await nodeApi<NativeContextEnvelope<Record<string, unknown>>>(
    input.adminUrl,
    '/api/project-docs/native-context/review',
    {
      method: 'POST',
      body: JSON.stringify({
        project_root: input.projectRoot,
        candidate_ids: input.candidateIds,
        action: input.action,
        authorization_mode: input.authorizationMode,
        expected_catalog_revision: input.catalogRevision,
        expected_suggestions_revision: input.suggestionsRevision,
      }),
    },
  )
  if (!envelope.ok || !envelope.result) throw new Error(envelope.error || '审核原生理解候选失败')
  return envelope.result
}

function sanitizeCandidatePage(
  value: unknown,
  fallbackStatus: NativeContextCandidateStatus,
  fallbackOffset: number,
  fallbackLimit: number,
): NativeContextCandidatePage {
  const page = value && typeof value === 'object' ? value as Record<string, unknown> : {}
  const pagination = page.pagination && typeof page.pagination === 'object'
    ? page.pagination as Record<string, unknown>
    : {}
  const candidates = Array.isArray(page.candidates)
    ? page.candidates.flatMap((entry) => sanitizeCandidate(entry) ?? [])
    : []
  const rawCounts = page.counts && typeof page.counts === 'object'
    ? page.counts as Record<string, unknown>
    : {}
  const status: NativeContextCandidateStatus | 'all' = page.status === 'all'
    ? 'all'
    : isCandidateStatus(page.status) ? page.status : fallbackStatus
  return {
    status,
    counts: {
      pending: safeNumber(rawCounts.pending),
      reviewed: safeNumber(rawCounts.reviewed),
      rejected: safeNumber(rawCounts.rejected),
      applied: safeNumber(rawCounts.applied),
    },
    pagination: {
      offset: safeNumber(pagination.offset, fallbackOffset),
      limit: safeNumber(pagination.limit, fallbackLimit),
      returned: candidates.length,
      total: safeNumber(pagination.total),
      next_offset: optionalNumber(pagination.next_offset),
    },
    candidates,
  }
}

function sanitizeCandidate(value: unknown): NativeContextCandidate | null {
  if (!value || typeof value !== 'object') return null
  const candidate = value as Record<string, unknown>
  const memory = sanitizeMemory(candidate)
  if (!memory || !isCandidateStatus(candidate.status)) return null
  return {
    ...memory,
    status: candidate.status,
    producer: boundedText(candidate.producer, 40),
    created_at_ms: safeNumber(candidate.created_at_ms),
    updated_at_ms: safeNumber(candidate.updated_at_ms),
    evidence_current: candidate.evidence_current === true,
  }
}

function sanitizeMemory(value: unknown): ProjectContextMemory | null {
  if (!value || typeof value !== 'object') return null
  const memory = value as Record<string, unknown>
  const candidateId = boundedText(memory.candidate_id, 80)
  const summary = boundedText(memory.summary, 800)
  const topics = uniqueStrings(memory.topics, 8, 48)
  const evidence = Array.isArray(memory.evidence)
    ? memory.evidence.slice(0, 8).flatMap((entry) => sanitizeEvidence(entry) ?? [])
    : []
  if (!/^[a-zA-Z0-9._-]+$/.test(candidateId) || summary.length < 12 || !topics.length || !evidence.length) {
    return null
  }
  return {
    candidate_id: candidateId,
    summary,
    topics,
    evidence,
    reviewed_at: boundedText(memory.reviewed_at, 40),
  }
}

function sanitizeEvidence(value: unknown): ProjectContextEvidence | null {
  if (!value || typeof value !== 'object') return null
  const evidence = value as Record<string, unknown>
  const path = boundedText(evidence.path, 500).replace(/\\/g, '/')
  const contentHash = boundedText(evidence.content_hash, 64).toLowerCase()
  const kind = boundedText(evidence.evidence_kind, 20) || 'source'
  if (!path || path.startsWith('/') || path.includes('../') || !/^[a-f0-9]{64}$/.test(contentHash)) return null
  if (!['source', 'test', 'document', 'configuration'].includes(kind)) return null
  return {
    path,
    content_hash: contentHash,
    locator: boundedText(evidence.locator, 120),
    evidence_kind: kind as ProjectContextEvidence['evidence_kind'],
  }
}

function isCandidateStatus(value: unknown): value is NativeContextCandidateStatus {
  return ['pending', 'reviewed', 'rejected', 'applied'].includes(String(value))
}

function uniqueStrings(value: unknown, limit: number, charLimit: number): string[] {
  if (!Array.isArray(value)) return []
  return [...new Set(value.map((entry) => boundedText(entry, charLimit)).filter(Boolean))].slice(0, limit)
}

function boundedText(value: unknown, limit: number): string {
  return String(value ?? '').trim().replace(/\s+/g, ' ').slice(0, limit)
}

function safeNumber(value: unknown, fallback = 0): number {
  const number = Number(value)
  return Number.isFinite(number) && number >= 0 ? Math.floor(number) : fallback
}

function optionalNumber(value: unknown): number | undefined {
  return value === null || value === undefined ? undefined : safeNumber(value)
}
