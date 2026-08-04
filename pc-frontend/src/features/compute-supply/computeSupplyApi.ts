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

export interface CapacityPoolBinding {
  pool_id: string
  capacity_epoch: number
  pool_revision: number
  pool_digest: string
}

export interface CapacityDeliveryWindowBinding {
  window_id: string
  window_digest: string
}

export interface CapacityBucketBinding {
  bucket_id: string
  bucket_digest: string
  pool: CapacityPoolBinding
  delivery_window: CapacityDeliveryWindowBinding
  meter: string
  meter_mode: CapacityMeterMode
  quantum_units: number
  meter_policy_digest: string
}

export interface CapacityBucketBalance {
  binding: CapacityBucketBinding
  status: 'open' | 'closed' | 'retired'
  issued_units: number
  available_units: number
  held_units: number
  active_units: number
  consumed_units: number
  retired_units: number
  balance_revision: number
  through_ledger_sequence: number | null
}

export interface MyComputeCapacityBucket {
  balance: CapacityBucketBalance
  starts_at_utc: string
  ends_at_utc: string
  replayed: boolean
}

export interface CreateMyComputeCapacityBucketBody {
  bucket_id: string
  window_id: string
  starts_at_utc: string
  ends_at_utc: string
  meter: string
}

export interface ComputeCapacityLedgerWriteReceipt {
  transaction_id: string
  transaction_digest: string
  ledger_sequence: number
  event_kind: 'supply_added' | 'supply_withdrawn' | string
  request_digest: string
  replayed: boolean
  current_balances: CapacityBucketBalance[]
}

export interface ChangeCapacitySupplyBody {
  idempotency_key: string
  lines: Array<{ bucket_id: string; quantity_units: number }>
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
  buckets: (providerId: string, poolId: string, limit = 100) =>
    api.get<MyComputeCapacityBucket[]>(
      `/api/me/compute/providers/${encodeURIComponent(providerId)}/capacity-pools/${encodeURIComponent(poolId)}/buckets?limit=${limit}`,
    ),
  createBucket: (providerId: string, poolId: string, body: CreateMyComputeCapacityBucketBody) =>
    api.post<MyComputeCapacityBucket>(
      `/api/me/compute/providers/${encodeURIComponent(providerId)}/capacity-pools/${encodeURIComponent(poolId)}/buckets`,
      body,
    ),
  addSupply: (providerId: string, poolId: string, body: ChangeCapacitySupplyBody) =>
    api.post<ComputeCapacityLedgerWriteReceipt>(
      `/api/me/compute/providers/${encodeURIComponent(providerId)}/capacity-pools/${encodeURIComponent(poolId)}/supply`,
      { ...body, confirm_supply: true },
    ),
  withdrawSupply: (providerId: string, poolId: string, body: ChangeCapacitySupplyBody) =>
    api.post<ComputeCapacityLedgerWriteReceipt>(
      `/api/me/compute/providers/${encodeURIComponent(providerId)}/capacity-pools/${encodeURIComponent(poolId)}/supply/withdraw`,
      { ...body, confirm_withdrawal: true },
    ),
}
