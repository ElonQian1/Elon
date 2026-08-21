import { api } from '../../api/client'
import {
  type FederationHistoricalLineageKind,
  type FederationHistoricalLineageScope,
  type ValidatedFederationHistoricalLineageRead,
  validateFederationHistoricalLineageReadResponse,
} from '../compute-attempt/federationHistoricalLineageContracts'
import { validateSettlementReleaseLineageReadResponse } from '../compute-attempt/federationHistoricalReleaseLineageContracts'
import { validateExecutionVerificationLineageReadResponse } from '../compute-attempt/federationHistoricalVerificationLineageContracts'

const SCOPE_ROOTS: Record<FederationHistoricalLineageScope, string> = {
  participant: '/api/me/compute/attempt-leases',
  admin: '/api/admin/compute/attempt-leases',
}

async function readLineage(
  scope: FederationHistoricalLineageScope,
  leaseId: string,
  kind: FederationHistoricalLineageKind,
  suffix: 'execution-source-lineage' | 'settlement-source-lineage',
): Promise<ValidatedFederationHistoricalLineageRead> {
  const value = await api.get<unknown>(`${SCOPE_ROOTS[scope]}/${encodeURIComponent(leaseId)}/${suffix}`)
  return validateFederationHistoricalLineageReadResponse(value, kind)
}

export const federationHistoricalLineageApi = {
  readExecution: (scope: FederationHistoricalLineageScope, leaseId: string) =>
    readLineage(scope, leaseId, 'execution_source_v1', 'execution-source-lineage'),
  readSettlement: (scope: FederationHistoricalLineageScope, leaseId: string) =>
    readLineage(scope, leaseId, 'settlement_source_v1', 'settlement-source-lineage'),
  readVerification: async (scope: FederationHistoricalLineageScope, leaseId: string) => {
    const value = await api.get<unknown>(
      `${SCOPE_ROOTS[scope]}/${encodeURIComponent(leaseId)}/execution-verification-source-lineage`,
    )
    return validateExecutionVerificationLineageReadResponse(value)
  },
  readRelease: async (scope: FederationHistoricalLineageScope, leaseId: string) => {
    const value = await api.get<unknown>(
      `${SCOPE_ROOTS[scope]}/${encodeURIComponent(leaseId)}/settlement-release-source-lineage`,
    )
    return validateSettlementReleaseLineageReadResponse(value)
  },
}
