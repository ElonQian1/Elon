import { nodeApi } from '../node/localNodeApi'

export {
  loadNativeContextMemoryHealth,
  type NativeContextMemoryHealth,
  type NativeContextMemoryHealthItem,
} from './projectDocumentNativeContextHealthModel'

export type NativeContextCandidateStatus = 'pending' | 'reviewed' | 'rejected' | 'applied'
export type NativeContextReviewAction = 'accept' | 'reject' | 'restore'
export type NativeContextRejectionReason = 'duplicate' | 'task_local' | 'unsupported' | 'conflict' | 'stale' | 'not_reusable'
export type NativeContextAuthorizationMode = 'git_backed_full' | 'trusted_reversible' | 'review_all' | 'suggestions_only'

export interface ProjectContextEvidence {
  path: string
  content_hash: string
  locator: string
  evidence_kind: 'source' | 'test' | 'document' | 'configuration'
  git_identity?: ProjectContextGitIdentity
}

export interface ProjectContextGitIdentity {
  schema: 'elon.project_context_git_identity.v1'
  head_commit: string
  head_blob_oid: string
  worktree_blob_oid: string
  state: 'tracked_clean' | 'tracked_modified' | 'index_only' | 'untracked'
}

export interface ProjectContextMemory {
  candidate_id: string
  summary: string
  topics: string[]
  evidence: ProjectContextEvidence[]
  reviewed_at: string
  owner: string
  scope: ProjectContextMemoryScope
  review: ProjectContextMemoryReview
}

export interface ProjectContextMemoryScope {
  kind: 'repository' | 'paths'
  paths: string[]
  scope_ids: string[]
  branches: string[]
  releases: string[]
  worktree_state: 'any' | 'clean' | 'dirty'
}

export interface ProjectContextMemoryReview {
  reviewed_on: string
  reviewed_by: string
  review_interval_days: number
  expires_at: string
}

export interface NativeContextCandidate extends ProjectContextMemory {
  status: NativeContextCandidateStatus
  producer: string
  created_at_ms: number
  updated_at_ms: number
  evidence_current: boolean
  ingest_action: 'created' | 'updated' | 'replacement' | 'deduplicated' | 'shared_duplicate' | ''
  provenance: NativeContextProvenance
  conflicts: NativeContextConflict[]
  review_feedback: NativeContextReviewFeedback
}

export interface NativeContextReviewFeedback {
  decision: '' | 'rejected'
  reason: '' | NativeContextRejectionReason
  decided_at: string
  decided_by: string
}

export interface NativeContextProvenance {
  source: string
  assurance: string
  session_fingerprint: string
  evidence_path_count: number
  recorded_at_ms: number
  last_editor: string
  last_edited_at_ms: number
}

export interface NativeContextConflict {
  kind: 'shared_duplicate' | 'shared_replacement' | 'potential_semantic_conflict'
  shared_candidate_id: string
  overlapping_paths: string[]
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
  producer_quality: {
    schema: string
    producers: Record<string, Partial<Record<NativeContextCandidateStatus, number>>>
    interpretation: string
  }
}

interface NativeContextEnvelope<T> {
  ok: boolean
  result?: T
  error?: string
}

export function sanitizeProjectContextMemories(value: unknown): ProjectContextMemory[] {
  if (!Array.isArray(value)) return []
  const unique = new Map<string, ProjectContextMemory>()
  value.slice(0, 256).forEach((entry) => {
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
  reviewReason?: NativeContextRejectionReason
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
        review_reason: input.reviewReason,
      }),
    },
  )
  if (!envelope.ok || !envelope.result) throw new Error(envelope.error || '审核原生理解候选失败')
  return envelope.result
}

export async function reviseNativeContextCandidate(input: {
  adminUrl: string
  projectRoot: string
  candidateId: string
  expectedUpdatedAtMs: number
  summary: string
  topics: string[]
}): Promise<NativeContextCandidate> {
  const envelope = await nodeApi<NativeContextEnvelope<Record<string, unknown>>>(
    input.adminUrl,
    '/api/project-docs/native-context/revise',
    {
      method: 'POST',
      body: JSON.stringify({
        project_root: input.projectRoot,
        candidate_id: input.candidateId,
        expected_updated_at_ms: input.expectedUpdatedAtMs,
        summary: input.summary,
        topics: input.topics,
      }),
    },
  )
  if (!envelope.ok || !envelope.result) throw new Error(envelope.error || '修订原生理解候选失败')
  const candidate = sanitizeCandidate(envelope.result.candidate)
  if (!candidate) throw new Error('修订后的原生理解候选响应无效')
  return candidate
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
    producer_quality: sanitizeProducerQuality(page.producer_quality),
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
    ingest_action: sanitizeIngestAction(candidate.ingest_action),
    provenance: sanitizeProvenance(candidate.provenance),
    conflicts: sanitizeConflicts(candidate.conflicts),
    review_feedback: sanitizeReviewFeedback(candidate.review_feedback),
  }
}

function sanitizeProducerQuality(value: unknown): NativeContextCandidatePage['producer_quality'] {
  const quality = value && typeof value === 'object' ? value as Record<string, unknown> : {}
  const rawProducers = quality.producers && typeof quality.producers === 'object'
    ? quality.producers as Record<string, unknown>
    : {}
  const producers: NativeContextCandidatePage['producer_quality']['producers'] = {}
  Object.entries(rawProducers).slice(0, 32).forEach(([rawProducer, rawCounts]) => {
    const producer = boundedText(rawProducer, 40)
    if (!producer || !rawCounts || typeof rawCounts !== 'object') return
    const counts = rawCounts as Record<string, unknown>
    producers[producer] = {
      pending: safeNumber(counts.pending),
      reviewed: safeNumber(counts.reviewed),
      rejected: safeNumber(counts.rejected),
      applied: safeNumber(counts.applied),
    }
  })
  return {
    schema: boundedText(quality.schema, 80),
    producers,
    interpretation: boundedText(quality.interpretation, 240),
  }
}

function sanitizeReviewFeedback(value: unknown): NativeContextReviewFeedback {
  const feedback = value && typeof value === 'object' ? value as Record<string, unknown> : {}
  const reason = boundedText(feedback.reason, 32)
  return {
    decision: feedback.decision === 'rejected' ? 'rejected' : '',
    reason: isRejectionReason(reason) ? reason : '',
    decided_at: boundedText(feedback.decided_at, 48),
    decided_by: boundedText(feedback.decided_by, 80),
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
    owner: boundedText(memory.owner, 80),
    scope: sanitizeMemoryScope(memory.scope),
    review: sanitizeMemoryReview(memory.review),
  }
}

function sanitizeMemoryScope(value: unknown): ProjectContextMemoryScope {
  const scope = value && typeof value === 'object' ? value as Record<string, unknown> : {}
  const kind = boundedText(scope.kind, 20)
  return {
    kind: kind === 'paths' ? 'paths' : 'repository',
    paths: uniqueStrings(scope.paths, 8, 500).map((path) => path.replace(/\\/g, '/')),
    scope_ids: uniqueStrings(scope.scope_ids, 8, 80),
    branches: uniqueStrings(scope.branches, 8, 120),
    releases: uniqueStrings(scope.releases, 8, 120),
    worktree_state: ['clean', 'dirty'].includes(String(scope.worktree_state))
      ? scope.worktree_state as ProjectContextMemoryScope['worktree_state']
      : 'any',
  }
}

export async function createNativeContextRelocationRepair(input: {
  adminUrl: string
  projectRoot: string
  candidateId: string
  sourcePath: string
  replacementPath: string
}): Promise<NativeContextCandidate> {
  const envelope = await nodeApi<NativeContextEnvelope<Record<string, unknown>>>(
    input.adminUrl,
    '/api/project-docs/native-context/repair-relocation',
    {
      method: 'POST',
      body: JSON.stringify({
        project_root: input.projectRoot,
        candidate_id: input.candidateId,
        source_path: input.sourcePath,
        replacement_path: input.replacementPath,
        producer: 'pc_memory_repair',
      }),
    },
  )
  if (!envelope.ok || !envelope.result) throw new Error(envelope.error || '创建共享记忆修复候选失败')
  const candidate = sanitizeCandidate(envelope.result.candidate)
  if (!candidate) throw new Error('共享记忆修复候选响应无效')
  return candidate
}

function sanitizeMemoryReview(value: unknown): ProjectContextMemoryReview {
  const review = value && typeof value === 'object' ? value as Record<string, unknown> : {}
  return {
    reviewed_on: boundedText(review.reviewed_on, 10),
    reviewed_by: boundedText(review.reviewed_by, 80),
    review_interval_days: safeNumber(review.review_interval_days),
    expires_at: boundedText(review.expires_at, 10),
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
    git_identity: sanitizeGitIdentity(evidence.git_identity),
  }
}

function sanitizeIngestAction(value: unknown): NativeContextCandidate['ingest_action'] {
  const action = boundedText(value, 32)
  return ['created', 'updated', 'replacement', 'deduplicated', 'shared_duplicate'].includes(action)
    ? action as NativeContextCandidate['ingest_action']
    : ''
}

function sanitizeProvenance(value: unknown): NativeContextProvenance {
  const provenance = value && typeof value === 'object' ? value as Record<string, unknown> : {}
  return {
    source: boundedText(provenance.source, 40),
    assurance: boundedText(provenance.assurance, 48),
    session_fingerprint: boundedText(provenance.session_fingerprint, 24),
    evidence_path_count: safeNumber(provenance.evidence_path_count),
    recorded_at_ms: safeNumber(provenance.recorded_at_ms),
    last_editor: boundedText(provenance.last_editor, 40),
    last_edited_at_ms: safeNumber(provenance.last_edited_at_ms),
  }
}

function sanitizeConflicts(value: unknown): NativeContextConflict[] {
  if (!Array.isArray(value)) return []
  return value.slice(0, 4).flatMap((entry) => {
    if (!entry || typeof entry !== 'object') return []
    const conflict = entry as Record<string, unknown>
    const kind = boundedText(conflict.kind, 40)
    if (!['shared_duplicate', 'shared_replacement', 'potential_semantic_conflict'].includes(kind)) return []
    return [{
      kind: kind as NativeContextConflict['kind'],
      shared_candidate_id: boundedText(conflict.shared_candidate_id, 80),
      overlapping_paths: uniqueStrings(conflict.overlapping_paths, 4, 500),
    }]
  })
}

function sanitizeGitIdentity(value: unknown): ProjectContextGitIdentity | undefined {
  if (!value || typeof value !== 'object') return undefined
  const identity = value as Record<string, unknown>
  const schema = boundedText(identity.schema, 80)
  const state = boundedText(identity.state, 32)
  const headCommit = boundedOid(identity.head_commit)
  const headBlobOid = boundedOid(identity.head_blob_oid)
  const worktreeBlobOid = boundedOid(identity.worktree_blob_oid)
  if (schema !== 'elon.project_context_git_identity.v1') return undefined
  if (!['tracked_clean', 'tracked_modified', 'index_only', 'untracked'].includes(state)) return undefined
  return {
    schema,
    state: state as ProjectContextGitIdentity['state'],
    head_commit: headCommit,
    head_blob_oid: headBlobOid,
    worktree_blob_oid: worktreeBlobOid,
  }
}

function boundedOid(value: unknown): string {
  const oid = boundedText(value, 64).toLowerCase()
  return /^([a-f0-9]{40}|[a-f0-9]{64})$/.test(oid) ? oid : ''
}

function isCandidateStatus(value: unknown): value is NativeContextCandidateStatus {
  return ['pending', 'reviewed', 'rejected', 'applied'].includes(String(value))
}

function isRejectionReason(value: unknown): value is NativeContextRejectionReason {
  return ['duplicate', 'task_local', 'unsupported', 'conflict', 'stale', 'not_reusable'].includes(String(value))
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
