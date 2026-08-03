export const CONNECTOR_SCHEMA: 'open_commerce.connector.v1'
export const CONNECTOR_CONTRACT_VERSION: '1.0'
export const MAX_SYNC_PAGE_RECORDS: 500
export const MAX_RECEIPT_KEY_LENGTH: 128
export const ADAPTER_HANDOFF_CLAIM_SCHEMA: 'open_commerce.adapter_business_handoff_claim.v1'
export const ADAPTER_HANDOFF_CLAIM_POLL_SCHEMA: 'open_commerce.adapter_business_handoff_claim_poll.v1'
export const ADAPTER_HANDOFF_CLAIM_RELEASE_SCHEMA: 'open_commerce.adapter_business_handoff_claim_release.v1'
export const ADAPTER_HANDOFF_CLAIM_RENEW_SCHEMA: 'open_commerce.adapter_business_handoff_claim_renew.v1'
export const ADAPTER_HANDOFF_MAX_RESPONSE_BYTES: number
export const ADAPTER_HANDOFF_WORKER_SCHEMA: 'open_commerce.adapter_handoff_worker_result.v1'

export type ConnectionMode =
  | 'official_api'
  | 'merchant_export'
  | 'local_adapter'
  | 'manual_import'
export type SyncKind = 'full' | 'incremental' | 'health_check'
export type SyncStatus = 'succeeded' | 'partial' | 'failed'
export type ConnectorHealthStatus = 'ready' | 'degraded' | 'unavailable'
export type ChangeOperation = 'upsert' | 'delete' | 'unchanged'

export interface ConnectorManifest {
  schema: typeof CONNECTOR_SCHEMA
  contractVersion: typeof CONNECTOR_CONTRACT_VERSION
  connectorKey: string
  providerKey: string
  displayName: string
  connectionMode: ConnectionMode
  scopes: string[]
  dataDomains: string[]
}

export interface ConnectorHealth {
  status: ConnectorHealthStatus
  observedAt: string
  evidenceCode: string
  message?: string
}

export interface ConnectorContext {
  projectId?: string
  merchantId?: string
  integrationId?: string
  signal?: AbortSignal
  [key: string]: unknown
}

export interface SyncRequest {
  integrationId: string
  runKey: string
  syncKind: SyncKind
  dataDomains: string[]
  cursor?: string
  limit?: number
}

export interface ConnectorChange {
  recordId: string
  dataDomain: string
  operation: ChangeOperation
  version?: string
  value?: Record<string, unknown>
}

export interface SyncPage {
  receiptKey: string
  syncKind: SyncKind
  status: SyncStatus
  changes: ConnectorChange[]
  nextCursor?: string
  errorCode?: string
  startedAt: string
  completedAt: string
}

export interface OpenCommerceConnector {
  describe(): ConnectorManifest
  health(context: ConnectorContext): Promise<ConnectorHealth>
  sync(request: SyncRequest): Promise<SyncPage>
}

export interface ServerSyncReceipt {
  integration_id: string
  receipt_key: string
  sync_kind: SyncKind
  status: SyncStatus
  records_seen: number
  records_changed: number
  cursor_digest?: string
  error_code?: string
  started_at: string
  completed_at: string
}

export interface CompatibilityOptions {
  request: SyncRequest
  context?: ConnectorContext
  verifyReplay?: boolean
}

export interface CompatibilityReport {
  schema: 'open_commerce.connector.compatibility.v1'
  compatible: true
  manifest: ConnectorManifest
  health: ConnectorHealth
  receipt: ServerSyncReceipt
  replayVerified: boolean
}

export class ConnectorContractError extends Error {
  code: string
  path: string
  constructor(code: string, message: string, path?: string)
}

export type AdapterHandoffReleaseReason =
  | 'adapter_shutdown'
  | 'capacity_pressure'
  | 'transient_failure'
  | 'manual_release'

export interface AdapterHandoffClaim {
  schema: typeof ADAPTER_HANDOFF_CLAIM_SCHEMA
  id: string
  project_id: string
  merchant_id: string
  invocation_id: string
  integration_id: string
  adapter_credential_id: string
  adapter_credential_version: number
  attempt_no: number
  status: 'active' | 'completed' | 'expired' | 'released'
  lease_token_hint: string
  lease_expires_at: string
  lease_deadline_at: string
  release_reason_code?: AdapterHandoffReleaseReason
  released_at?: string
  completion_status?: 'applied' | 'ignored' | 'rejected'
  retry_not_before?: string
  retry_suspended_at?: string
  retry_suspension_reason?: string
  retry_resumed_at?: string
  completed_receipt_id?: string
  created_at: string
  updated_at: string
}

export interface AdapterHandoffClaimIssue {
  claim: AdapterHandoffClaim
  lease_token: string
  lease_token_visible_once: true
  task: {
    evidence: Record<string, unknown>
    result: unknown
  }
}

export interface AdapterHandoffClaimPoll {
  schema: typeof ADAPTER_HANDOFF_CLAIM_POLL_SCHEMA
  claimed: boolean
  issue?: AdapterHandoffClaimIssue | null
  retry_after_seconds: number
  boundary: string[]
}

export interface AdapterHandoffCompletion {
  receiptKey: string
  status: 'applied' | 'ignored' | 'rejected'
  targetDomain: string
  targetReference?: string
  errorCode?: string
  completedAt: string
}

export interface AdapterHandoffClient {
  claimNext(options?: { leaseSeconds?: number; signal?: AbortSignal }): Promise<AdapterHandoffClaimPoll>
  complete(
    issue: AdapterHandoffClaimIssue,
    receipt: AdapterHandoffCompletion,
    options?: { signal?: AbortSignal },
  ): Promise<Record<string, unknown>>
  release(
    issue: AdapterHandoffClaimIssue,
    reasonCode: AdapterHandoffReleaseReason,
    options?: { signal?: AbortSignal },
  ): Promise<{
    schema: typeof ADAPTER_HANDOFF_CLAIM_RELEASE_SCHEMA
    claim: AdapterHandoffClaim
    retryable: true
    boundary: string[]
  }>
  renew(
    issue: AdapterHandoffClaimIssue,
    options?: { extendSeconds?: number; signal?: AbortSignal },
  ): Promise<{
    schema: typeof ADAPTER_HANDOFF_CLAIM_RENEW_SCHEMA
    claim: AdapterHandoffClaim
    renewed: true
    boundary: string[]
  }>
}

export class AdapterHandoffClientError extends Error {
  code: string
  status?: number
  constructor(code: string, message: string, status?: number)
}

export function createAdapterHandoffClient(options: {
  baseUrl: string
  token: string
  fetch?: typeof globalThis.fetch
}): AdapterHandoffClient

export interface AdapterHandoffWorkerContext {
  claim: AdapterHandoffClaim
  idempotencyKey: string
  attemptNo: number
  signal: AbortSignal
}

export interface AdapterHandoffWorkerOutcome {
  status: 'applied' | 'ignored' | 'rejected'
  receiptKey?: string
  targetReference?: string
  errorCode?: string
  completedAt?: string
}

export interface AdapterHandoffWorkerResult {
  schema: typeof ADAPTER_HANDOFF_WORKER_SCHEMA
  claimed: boolean
  claimId?: string
  invocationId?: string
  status?: 'applied' | 'ignored' | 'rejected'
  receipt?: Record<string, unknown>
  retryAfterMs: number
}

export interface AdapterHandoffWorkerSummary {
  claimed: number
  completed: number
  failed: number
}

export interface AdapterHandoffWorker {
  runOnce(options?: { signal?: AbortSignal }): Promise<AdapterHandoffWorkerResult>
  run(options?: {
    signal?: AbortSignal
    onResult?: (result: AdapterHandoffWorkerResult) => void | Promise<void>
    onError?: (error: unknown) => void | Promise<void>
  }): Promise<AdapterHandoffWorkerSummary>
}

export class AdapterHandoffReleaseError extends Error {
  reasonCode: AdapterHandoffReleaseReason
  constructor(reasonCode: AdapterHandoffReleaseReason, message?: string)
}

export class AdapterHandoffRejectError extends Error {
  errorCode: string
  constructor(errorCode: string, message?: string)
}

export function createAdapterHandoffWorker(options: {
  client?: AdapterHandoffClient
  baseUrl?: string
  token?: string
  fetch?: typeof globalThis.fetch
  targetDomain: string
  handler: (
    task: AdapterHandoffClaimIssue['task'],
    context: AdapterHandoffWorkerContext,
  ) => AdapterHandoffWorkerOutcome | Promise<AdapterHandoffWorkerOutcome>
  leaseSeconds?: number
  extendSeconds?: number
  renewBeforeSeconds?: number
  idleDelayMs?: number
  errorDelayMs?: number
  completionAttempts?: number
}): AdapterHandoffWorker

export function defineConnector<T extends OpenCommerceConnector>(connector: T): Readonly<T>
export function validateConnector<T extends OpenCommerceConnector>(connector: T): T
export function validateManifest<T extends ConnectorManifest>(manifest: T): T
export function validateHealth<T extends ConnectorHealth>(result: T): T
export function validateSyncRequest(request: SyncRequest): SyncRequest & { limit: number }
export function validateSyncPage<T extends SyncPage>(
  page: T,
  request: SyncRequest & { limit: number },
): T
export function createSyncReceipt(
  request: SyncRequest,
  page: SyncPage,
): ServerSyncReceipt
export function runConnectorCompatibility(
  connector: OpenCommerceConnector,
  options: CompatibilityOptions,
): Promise<CompatibilityReport>
