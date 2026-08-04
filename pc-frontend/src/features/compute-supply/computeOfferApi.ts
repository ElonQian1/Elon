import { api } from '../../api/client'
import { type CapacityBucketBinding, type CapacityPoolBinding } from './computeSupplyApi'

export interface ComputePriceComponent {
  meter: string
  unit_size: number
  consumer_unit_price_micros: number
  provider_unit_price_micros: number
  max_units: number
}

export interface ComputeFeeRule {
  fee_kind: string
  charged_to: string
  fixed_amount_micros: number
  rate_basis_points: number
  maximum_amount_micros: number | null
}

export interface ComputeModelRef {
  model_id: string
  model_family: string
  model_digest: string
  tokenizer_digest: string | null
  adapter_digests: string[]
}

export interface ComputeRuntimeRef {
  runtime_family: string
  runtime_version: string
  precision: string
  runner_digest: string
  plugin_id: string | null
  plugin_version: string | null
  plugin_digest: string | null
}

export interface ComputeOfferDraftBody {
  idempotency_key: string
  sku: {
    sku_id: string
    task_kind: string
    context_or_shape_bucket: string
    verification_tier: string
    sla_tier: string
    delivery_window_class: string
  }
  model: ComputeModelRef | null
  runtime: ComputeRuntimeRef
  resource_profile: {
    accelerator_kind: string
    accelerator_count: number
    vram_bytes: number
    ram_bytes: number
  }
  capacity: Array<{ bucket_id: string; total_units: number; reservable_units: number }>
  execution_limits: { max_concurrent_attempts: number; max_attempt_runtime_seconds: number }
  authorization: {
    public: boolean
    allowed_account_ids: string[]
    allowed_project_ids: string[]
    allowed_data_classes: string[]
  }
  price_terms: {
    pricing_mode: 'spot' | 'index_locked' | 'capacity_forward' | 'capacity_future'
    currency: string
    curve_id: null
    curve_version: null
    instrument_id: null
    components: ComputePriceComponent[]
    fee_rules: ComputeFeeRule[]
  }
  valid_from: string
  valid_until: string
  confirm_create: true
}

export type ReviseComputeOfferDraftBody = Omit<ComputeOfferDraftBody, 'idempotency_key' | 'confirm_create'> & {
  expected_offer_version: number
  expected_offer_digest: string
  confirm_revise: true
}

export interface MyComputeOfferView {
  offer: {
    offer_id: string
    offer_version: number
    offer_digest: string
    provider_id: string
    provider_kind: string
    status: string
    sku: {
      sku_id: string
      task_kind: string
      context_or_shape_bucket: string
      verification_tier: string
      sla_tier: string
      region_or_data_zone: string
      delivery_window_class: string
      metering_units: string[]
      sku_digest: string
    }
    model: ComputeModelRef | null
    runtime: ComputeRuntimeRef
    resource_profile: { accelerator_kind: string; accelerator_count: number; vram_bytes: number; ram_bytes: number }
    capacity_pool: CapacityPoolBinding
    capacity: Array<{ bucket: CapacityBucketBinding; total_units: number; reservable_units: number }>
    execution_limits: { max_concurrent_attempts: number; max_attempt_runtime_seconds: number }
    authorization: { public: boolean; allowed_account_ids: string[]; allowed_project_ids: string[]; allowed_data_classes: string[]; policy_revision: number }
    price_terms: { pricing_mode: 'spot' | 'index_locked' | 'capacity_forward' | 'capacity_future'; currency: string; curve_id: string | null; curve_version: number | null; instrument_id: string | null; components: ComputePriceComponent[]; fee_rules: ComputeFeeRule[]; valid_until: string }
    valid_from: string
    valid_until: string
    created_at: string
  }
  provider_policy_revision: number
  provider_digest: string
  replayed: boolean
  market_effect: 'none'
}

function base(providerId: string, poolId: string) {
  return `/api/me/compute/providers/${encodeURIComponent(providerId)}/capacity-pools/${encodeURIComponent(poolId)}/offers`
}

export const computeOfferApi = {
  list: (providerId: string, poolId: string, limit = 50) =>
    api.get<MyComputeOfferView[]>(`${base(providerId, poolId)}?limit=${limit}`),
  create: (providerId: string, poolId: string, body: ComputeOfferDraftBody) =>
    api.post<MyComputeOfferView>(base(providerId, poolId), body),
  revise: (providerId: string, poolId: string, offerId: string, body: ReviseComputeOfferDraftBody) =>
    api.post<MyComputeOfferView>(`${base(providerId, poolId)}/${encodeURIComponent(offerId)}/revise`, body),
  revoke: (providerId: string, poolId: string, offerId: string, expectedVersion: number, expectedDigest: string) =>
    api.post<MyComputeOfferView>(`${base(providerId, poolId)}/${encodeURIComponent(offerId)}/revoke`, {
      expected_offer_version: expectedVersion,
      expected_offer_digest: expectedDigest,
      confirm_revoke: true,
    }),
}
