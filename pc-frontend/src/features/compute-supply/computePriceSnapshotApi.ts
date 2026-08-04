import { api } from '../../api/client'
import {
  type ComputeDeliveryWindow,
  type ComputeFeeRule,
  type ComputePriceComponent,
  type MyComputeOfferView,
} from './computeOfferApi'

export type ComputeRoundingMode = 'half_up' | 'half_even' | 'floor' | 'ceil'

export interface PublishComputePriceSnapshotBody {
  expected_offer_version: number
  expected_offer_digest: string
  delivery_window_id: string
  consumer_max_amount_micros: number
  provider_max_amount_micros: number
  ttl_seconds: number
  rounding_mode: ComputeRoundingMode
  idempotency_key: string
  confirm_publish: true
}

export interface MyComputePriceSnapshotView {
  snapshot: {
    schema: string
    snapshot_id: string
    snapshot_digest: string
    quote_id: string
    pricing_mode: string
    sku: MyComputeOfferView['offer']['sku']
    provider_id: string
    offer_id: string
    offer_version: number
    offer_digest: string
    delivery_window: ComputeDeliveryWindow
    currency: string
    components: ComputePriceComponent[]
    fee_rules: ComputeFeeRule[]
    consumer_max_amount_micros: number
    provider_max_amount_micros: number
    price_source: {
      source_kind: string
      source_id: string
      source_version: number
      observation_window_start: string
      observation_window_end: string
      sample_count: number
      source_digest: string
    }
    rounding_mode: ComputeRoundingMode
    quoted_at: string
    expires_at: string
  }
  replayed: boolean
  market_effect: 'quote_candidate_enabled'
  reservation_effect: 'none'
  capacity_effect: 'none'
  funds_effect: 'none'
}

interface SnapshotListResponse { snapshots: MyComputePriceSnapshotView[] }

function base(view: MyComputeOfferView) {
  const { provider_id: providerId, offer_id: offerId } = view.offer
  const poolId = view.offer.capacity_pool.pool_id
  return `/api/me/compute/providers/${encodeURIComponent(providerId)}/capacity-pools/${encodeURIComponent(poolId)}/offers/${encodeURIComponent(offerId)}/price-snapshots`
}

export const computePriceSnapshotApi = {
  list: (view: MyComputeOfferView, limit = 20) =>
    api.get<SnapshotListResponse>(`${base(view)}?limit=${limit}`).then((response) => response.snapshots),
  publish: (view: MyComputeOfferView, body: PublishComputePriceSnapshotBody) =>
    api.post<MyComputePriceSnapshotView>(base(view), body),
}
