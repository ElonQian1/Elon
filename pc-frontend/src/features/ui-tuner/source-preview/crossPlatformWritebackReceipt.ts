import { nodeApi, probeLocalNode } from '../../node/localNodeApi'
import { sourcePreviewAdminUrl } from './sourcePreviewApi'

export type CrossPlatformTarget = 'pwa' | 'apk'
export type CrossPlatformReceiptStatus =
  | 'PREVIEW'
  | 'IN_PROGRESS'
  | 'PARTIAL'
  | 'FAILED'
  | 'EVIDENCE_MISSING'
  | 'COMPLETE'
export type PlatformWritebackStatus =
  | 'PREVIEW'
  | 'AI_WRITING'
  | 'SAVED'
  | 'BUILD_VERIFYING'
  | 'BUILD_VERIFIED'
  | 'FAILED'
  | 'EVIDENCE_MISSING'
export type PlatformWritebackMethod = 'PENDING' | 'DETERMINISTIC' | 'CODEX' | 'MIXED'

export interface PlatformWritebackResult {
  platform: CrossPlatformTarget
  status: PlatformWritebackStatus
  method: PlatformWritebackMethod
  changedFiles: string[]
  sourceRevisions: Record<string, string>
  sourceHashes: Record<string, string>
  buildEvidence?: Record<string, unknown>
  aiTaskId?: string
  error?: string
  evidenceComplete: boolean
}

export interface CrossPlatformWritebackReceipt {
  schemaVersion: 1
  receiptId: string
  operationId: string
  projectRoot: string
  draftRevision: number
  targetPlatforms: CrossPlatformTarget[]
  sourceRevisionBefore: string
  sourceRevision: string
  sourceHash: string
  changedFiles: string[]
  sourceHashes: Record<string, string>
  platformResults: Record<CrossPlatformTarget, PlatformWritebackResult>
  status: CrossPlatformReceiptStatus
  complete: boolean
  evidenceComplete: boolean
  diagnostics: string[]
  createdAt: string
  updatedAt: string
}

export interface PlatformReceiptUpdate {
  status: PlatformWritebackStatus
  method: PlatformWritebackMethod
  changedFiles: string[]
  sourceRevisions?: Record<string, string>
  expectedSourceRevisionBefore?: string
  buildEvidence?: Record<string, unknown>
  aiTaskId?: string
  error?: string
}

export async function beginCrossPlatformWritebackReceipt(input: {
  operationId: string
  projectRoot: string
  draftRevision: number
  targetPlatforms: CrossPlatformTarget[]
}): Promise<CrossPlatformWritebackReceipt> {
  const baseUrl = sourcePreviewAdminUrl()
  await probeLocalNode(baseUrl)
  return nodeApi<CrossPlatformWritebackReceipt>(
    baseUrl,
    '/api/source-preview/writeback-receipts/begin',
    { method: 'POST', body: JSON.stringify(input) },
    15_000,
  )
}

export async function completeCrossPlatformWritebackReceipt(input: {
  receiptId: string
  projectRoot: string
  platformResults: Partial<Record<CrossPlatformTarget, PlatformReceiptUpdate>>
}): Promise<CrossPlatformWritebackReceipt> {
  return nodeApi<CrossPlatformWritebackReceipt>(
    sourcePreviewAdminUrl(),
    '/api/source-preview/writeback-receipts/complete',
    { method: 'POST', body: JSON.stringify(input) },
    20_000,
  )
}
