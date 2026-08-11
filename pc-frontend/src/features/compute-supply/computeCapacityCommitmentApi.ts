import { api } from '../../api/client'
import { type ReferenceCurveSnapshotBindingReceipt } from '../compute-reference-curves/computeReferenceCurveApi'
import { type MyComputeOfferView } from './computeOfferApi'
import { type MyComputePriceSnapshotView } from './computePriceSnapshotApi'

export interface CapacityCommitmentQuantity {
  meter: string
  quantity_units: number
}

export interface CapacityCommitmentDetail {
  commitment: {
    commitment_id: string
    commitment_revision: number
    commitment_digest: string
    commitment_status: 'committed'
    provider: { provider_id: string; policy_revision: number; provider_digest: string }
    offer: { offer_id: string; offer_version: number; offer_digest: string }
    pool: { pool_id: string; capacity_epoch: number; pool_revision: number; pool_digest: string }
    delivery_window: MyComputePriceSnapshotView['snapshot']['delivery_window']
    price_snapshot_id: string
    price_snapshot_digest: string
    reference_binding: { binding_id: string; binding_digest: string }
    instrument_id: string
    created_at: string
    expires_at: string
  }
  terminal_receipt: {
    terminal_revision: number
    terminal_status: 'canceled' | 'expired'
    terminal_receipt_digest: string
    reason: string | null
    occurred_at: string
  } | null
  current_status: 'committed' | 'canceled' | 'expired'
  quantities: CapacityCommitmentQuantity[]
}

export interface CapacityCommitmentSourceView {
  snapshot: MyComputePriceSnapshotView['snapshot']
  reference_binding: ReferenceCurveSnapshotBindingReceipt
}

export interface CreateCapacityCommitmentBody {
  idempotency_key: string
  provider_policy_revision: number
  provider_digest: string
  offer_id: string
  offer_version: number
  offer_digest: string
  capacity_epoch: number
  pool_revision: number
  pool_digest: string
  delivery_window_id: string
  delivery_window_digest: string
  price_snapshot_id: string
  price_snapshot_digest: string
  reference_binding_id: string
  reference_binding_digest: string
  instrument_id: string
  quantities: CapacityCommitmentQuantity[]
  confirm_commitment: true
}

interface CapacityCommitmentListResponse {
  capacity_commitments: CapacityCommitmentDetail[]
}

interface CapacityCommitmentCreateReceipt {
  commitment: CapacityCommitmentDetail['commitment']
  quantities: CapacityCommitmentQuantity[]
  replayed: boolean
}

interface CapacityCommitmentCancelReceipt {
  terminal_receipt: NonNullable<CapacityCommitmentDetail['terminal_receipt']>
  replayed: boolean
}

function base(view: MyComputeOfferView) {
  const { provider_id: providerId } = view.offer
  const poolId = view.offer.capacity_pool.pool_id
  return `/api/me/compute/providers/${encodeURIComponent(providerId)}/capacity-pools/${encodeURIComponent(poolId)}/capacity-commitments`
}

export const computeCapacityCommitmentApi = {
  list: (view: MyComputeOfferView, limit = 100) =>
    api.get<CapacityCommitmentListResponse>(`${base(view)}?limit=${limit}`)
      .then((response) => response.capacity_commitments),
  source: (view: MyComputeOfferView, snapshotId: string) => {
    const { provider_id: providerId, offer_id: offerId } = view.offer
    const poolId = view.offer.capacity_pool.pool_id
    return api.get<CapacityCommitmentSourceView>(
      `/api/me/compute/providers/${encodeURIComponent(providerId)}/capacity-pools/${encodeURIComponent(poolId)}/offers/${encodeURIComponent(offerId)}/price-snapshots/${encodeURIComponent(snapshotId)}/capacity-commitment-source`,
    )
  },
  create: (view: MyComputeOfferView, body: CreateCapacityCommitmentBody) =>
    api.post<CapacityCommitmentCreateReceipt>(base(view), body),
  cancel: (view: MyComputeOfferView, detail: CapacityCommitmentDetail, reason: string, idempotencyKey: string) =>
    api.post<CapacityCommitmentCancelReceipt>(
      `${base(view)}/${encodeURIComponent(detail.commitment.commitment_id)}/cancel`,
      {
        idempotency_key: idempotencyKey,
        expected_commitment_revision: detail.commitment.commitment_revision,
        expected_commitment_digest: detail.commitment.commitment_digest,
        reason,
        confirm_cancel: true,
      },
    ),
}
