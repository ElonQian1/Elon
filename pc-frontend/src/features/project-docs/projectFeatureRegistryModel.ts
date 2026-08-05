import { nodeApi } from '../node/localNodeApi'

export type ProjectFeatureStatus =
  | 'draft' | 'proposed' | 'accepted' | 'ready' | 'claimed' | 'in_progress'
  | 'blocked' | 'implemented' | 'verified' | 'released' | 'retired'
export type ProjectFeaturePriority = 'p0' | 'p1' | 'p2' | 'p3'

export interface ProjectFeatureClaim {
  claim_id: string
  agent_id: string
  claimed_at_ms: number
  expires_at_ms: number
}

export interface ProjectFeatureSnapshot {
  id: string
  title: string
  summary: string
  status: ProjectFeatureStatus
  priority: ProjectFeaturePriority
  requirement_path: string
  requirement_hash: string
  requirement_current: boolean
  knowledge_node_id: string
  owner: string
  tags: string[]
  task_paths: string[]
  acceptance_criteria_count: number
  implementation_evidence_count: number
  implementation_evidence_current: boolean | null
  implementation_evidence_checked: boolean
  dependency_blockers: string[]
  claim?: ProjectFeatureClaim
  claim_expired: boolean
  claimable: boolean
  drift_status: 'current' | 'requirement_drifted' | 'implementation_evidence_drifted' | 'implementation_evidence_not_checked'
  created_at_ms: number
  updated_at_ms: number
}

export interface ProjectFeaturePage {
  schema: 'elon.project_feature_list.v1'
  registry_revision?: string
  total: number
  offset: number
  returned: number
  features: ProjectFeatureSnapshot[]
  source_bodies_returned: 0
}

interface FeatureEnvelope<T> {
  ok: boolean
  result?: T
  error?: string
}

export async function listProjectFeatures(input: {
  adminUrl: string
  projectRoot: string
  statuses?: ProjectFeatureStatus[]
  query?: string
  offset?: number
  limit?: number
}): Promise<ProjectFeaturePage> {
  const envelope = await nodeApi<FeatureEnvelope<unknown>>(
    input.adminUrl,
    '/api/project-docs/features/list',
    {
      method: 'POST',
      body: JSON.stringify({
        project_root: input.projectRoot,
        statuses: input.statuses ?? [],
        query: input.query ?? '',
        offset: input.offset ?? 0,
        limit: input.limit ?? 50,
      }),
    },
  )
  if (!envelope.ok || !envelope.result) throw new Error(envelope.error || '读取功能登记失败')
  return sanitizeFeaturePage(envelope.result)
}

export async function listAllProjectFeatures(input: {
  adminUrl: string
  projectRoot: string
  statuses?: ProjectFeatureStatus[]
  query?: string
}): Promise<ProjectFeaturePage> {
  const features: ProjectFeatureSnapshot[] = []
  let offset = 0
  let total = 0
  let registryRevision: string | undefined
  do {
    const page = await listProjectFeatures({ ...input, offset, limit: 100 })
    if (registryRevision !== undefined && page.registry_revision !== registryRevision) {
      throw new Error('功能登记在分页读取期间发生变化，请刷新后重试')
    }
    registryRevision = page.registry_revision
    total = page.total
    features.push(...page.features)
    if (!page.returned) break
    offset += page.returned
  } while (offset < total && features.length < 512)
  return {
    schema: 'elon.project_feature_list.v1',
    registry_revision: registryRevision,
    total,
    offset: 0,
    returned: features.length,
    features,
    source_bodies_returned: 0,
  }
}

function sanitizeFeaturePage(value: unknown): ProjectFeaturePage {
  const page = objectValue(value)
  const features = Array.isArray(page.features)
    ? page.features.flatMap((entry) => sanitizeFeature(entry) ?? [])
    : []
  return {
    schema: 'elon.project_feature_list.v1',
    registry_revision: stringValue(page.registry_revision) || undefined,
    total: safeNumber(page.total, features.length),
    offset: safeNumber(page.offset),
    returned: features.length,
    features,
    source_bodies_returned: 0,
  }
}

function sanitizeFeature(value: unknown): ProjectFeatureSnapshot | null {
  const item = objectValue(value)
  const id = stringValue(item.id).slice(0, 96)
  const title = stringValue(item.title).slice(0, 160)
  const status = isStatus(item.status) ? item.status : null
  const priority = isPriority(item.priority) ? item.priority : 'p2'
  const requirementPath = stringValue(item.requirement_path)
  if (!id || !title || !status || !requirementPath) return null
  const claimValue = objectValue(item.claim)
  const claimId = stringValue(claimValue.claim_id)
  const claim = claimId ? {
    claim_id: claimId,
    agent_id: stringValue(claimValue.agent_id),
    claimed_at_ms: safeNumber(claimValue.claimed_at_ms),
    expires_at_ms: safeNumber(claimValue.expires_at_ms),
  } : undefined
  const drift = ['current', 'requirement_drifted', 'implementation_evidence_drifted', 'implementation_evidence_not_checked'].includes(String(item.drift_status))
    ? item.drift_status as ProjectFeatureSnapshot['drift_status']
    : 'requirement_drifted'
  return {
    id,
    title,
    summary: stringValue(item.summary).slice(0, 800),
    status,
    priority,
    requirement_path: requirementPath,
    requirement_hash: stringValue(item.requirement_hash).slice(0, 64),
    requirement_current: item.requirement_current === true,
    knowledge_node_id: stringValue(item.knowledge_node_id).slice(0, 96),
    owner: stringValue(item.owner).slice(0, 80),
    tags: stringArray(item.tags, 12),
    task_paths: stringArray(item.task_paths, 24),
    acceptance_criteria_count: safeNumber(item.acceptance_criteria_count),
    implementation_evidence_count: safeNumber(item.implementation_evidence_count),
    implementation_evidence_current: typeof item.implementation_evidence_current === 'boolean'
      ? item.implementation_evidence_current
      : null,
    implementation_evidence_checked: item.implementation_evidence_checked === true,
    dependency_blockers: stringArray(item.dependency_blockers, 32),
    claim,
    claim_expired: item.claim_expired === true,
    claimable: item.claimable === true,
    drift_status: drift,
    created_at_ms: safeNumber(item.created_at_ms),
    updated_at_ms: safeNumber(item.updated_at_ms),
  }
}

function isStatus(value: unknown): value is ProjectFeatureStatus {
  return ['draft', 'proposed', 'accepted', 'ready', 'claimed', 'in_progress', 'blocked', 'implemented', 'verified', 'released', 'retired'].includes(String(value))
}

function isPriority(value: unknown): value is ProjectFeaturePriority {
  return ['p0', 'p1', 'p2', 'p3'].includes(String(value))
}

function objectValue(value: unknown): Record<string, unknown> {
  return value && typeof value === 'object' ? value as Record<string, unknown> : {}
}

function stringValue(value: unknown): string {
  return typeof value === 'string' ? value.trim() : ''
}

function stringArray(value: unknown, limit: number): string[] {
  return Array.isArray(value)
    ? value.slice(0, limit).map(stringValue).filter(Boolean)
    : []
}

function safeNumber(value: unknown, fallback = 0): number {
  const number = Number(value)
  return Number.isFinite(number) && number >= 0 ? number : fallback
}
