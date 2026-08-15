export const VERIFIED_ERP_HANDOFF_SOURCE_SCHEMA: 'open_commerce.verified_erp_handoff_source.v1'
export const VERIFIED_ERP_HANDOFF_READBACK_SCHEMA: 'open_commerce.verified_erp_handoff_readback.v1'

export interface VerifiedErpBusinessReceipt {
  schema: 'open_commerce.merchant_business_receipt.v1'
  entityType: string
  referenceId: string
  state: string
  occurredAt: string
  amountMinor?: number
  currency?: string
}

export interface VerifiedErpHandoffSource {
  schema: typeof VERIFIED_ERP_HANDOFF_SOURCE_SCHEMA
  projectId: string
  merchantId: string
  invocationId: string
  integrationId: string
  idempotencyKey: string
  evidenceResultSha256: string
  businessReceipt: VerifiedErpBusinessReceipt
  fundsMoved: false
  adapterCredential: {
    id: string
    version: number
  }
  sourceDigest: string
}

export interface VerifiedErpPluginContext {
  signal: AbortSignal
  idempotencyKey: string
  attemptNo: number
}

export interface VerifiedErpHandoffWorkerContext extends VerifiedErpPluginContext {
  claim: {
    schema: 'open_commerce.adapter_business_handoff_claim.v1'
    status: 'active' | 'completed' | 'expired' | 'released'
    project_id: string
    merchant_id: string
    invocation_id: string
    integration_id: string
    adapter_credential_id: string
    adapter_credential_version: number
    attempt_no: number
  }
}

export interface VerifiedErpApplyInput {
  source: Readonly<VerifiedErpHandoffSource>
  result: unknown
}

export interface VerifiedErpReadBackInput {
  targetReference: string
  source: Readonly<VerifiedErpHandoffSource>
}

export interface VerifiedErpReadBackResult {
  schema: typeof VERIFIED_ERP_HANDOFF_READBACK_SCHEMA
  targetReference: string
  source: VerifiedErpHandoffSource
}

export class VerifiedErpHandoffError extends Error {
  code: string
  path: string
  constructor(code: string, message: string, path?: string)
}

export function createVerifiedErpHandoffHandler(options: {
  apply(
    input: Readonly<VerifiedErpApplyInput>,
    context: Readonly<VerifiedErpPluginContext>,
  ): Promise<{ targetReference: string }>
  readBack(
    input: Readonly<VerifiedErpReadBackInput>,
    context: Readonly<VerifiedErpPluginContext>,
  ): Promise<VerifiedErpReadBackResult>
}): (
  task: {
    evidence: Record<string, unknown>
    result: unknown
  },
  context: VerifiedErpHandoffWorkerContext,
) => Promise<{ status: 'applied'; targetReference: string }>
