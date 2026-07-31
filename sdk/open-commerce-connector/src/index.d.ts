export const CONNECTOR_SCHEMA: 'open_commerce.connector.v1'
export const CONNECTOR_CONTRACT_VERSION: '1.0'
export const MAX_SYNC_PAGE_RECORDS: 500
export const MAX_RECEIPT_KEY_LENGTH: 128

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
