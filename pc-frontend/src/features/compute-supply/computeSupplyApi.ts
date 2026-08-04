import { api } from '../../api/client'
import { type MyComputeProvider } from '../compute-settlement/myComputeSettlementApi'

export type CapacityMeterMode = 'consumable' | 'reusable'

export interface CapacityMeterPolicy {
  meter: string
  meter_mode: CapacityMeterMode
  quantum_units: number
  policy_digest: string
}

export interface MyComputeCapacityPool {
  pool_id: string
  provider_id: string
  status: string
  capacity_epoch: number
  pool_revision: number
  pool_digest: string
  resource_scope_digest: string
  resource_profile_digest: string
  region_or_data_zone: string
  meter_policies: CapacityMeterPolicy[]
  created_at: string
  replayed: boolean
}

export interface CreateMyComputeCapacityPoolBody {
  pool_id: string
  resource_scope_key: string
  region_or_data_zone: string
  resource_profile: Record<string, unknown>
  meter_policies: Array<{
    meter: string
    meter_mode: CapacityMeterMode
    quantum_units: number
  }>
}

export const computeSupplyApi = {
  providers: (limit = 100) =>
    api.get<MyComputeProvider[]>(`/api/me/compute/providers?limit=${limit}`),
  pools: (providerId: string, limit = 100) =>
    api.get<MyComputeCapacityPool[]>(
      `/api/me/compute/providers/${encodeURIComponent(providerId)}/capacity-pools?limit=${limit}`,
    ),
  createPool: (providerId: string, body: CreateMyComputeCapacityPoolBody) =>
    api.post<MyComputeCapacityPool>(
      `/api/me/compute/providers/${encodeURIComponent(providerId)}/capacity-pools`,
      body,
    ),
}
