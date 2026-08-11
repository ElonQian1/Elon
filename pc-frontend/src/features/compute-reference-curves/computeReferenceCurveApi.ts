import { api } from '../../api/client'
import { type ComputeFeeRule, type ComputePriceComponent } from '../compute-supply/computeOfferApi'

export type ReferenceCurveBatchStatus = 'submitted' | 'approved' | 'changes_requested' | 'rejected' | 'applied'
export type ReferenceCurveReviewDecision = 'approved' | 'changes_requested' | 'rejected'

export interface ReferenceCurveEntryIntent {
  entry_key: string
  provider_id: string
  offer_id: string
  offer_version: number
  offer_digest: string
  sku_id: string
  sku_digest: string
  delivery_window_id: string
  delivery_window_digest: string
  pricing_mode: 'spot' | 'capacity_future'
  currency: 'CNY'
  offer_curve_id: string | null
  offer_curve_version: number | null
  instrument_id: string | null
  components: ComputePriceComponent[]
  fee_rules: ComputeFeeRule[]
  consumer_max_amount_micros: number
  provider_max_amount_micros: number
}

export interface ReferenceCurveEntryReceipt {
  schema: string
  batch_id: string
  batch_digest: string
  entry_id: string
  entry_digest: string
  ordinal: number
  entry_key: string
  offer_id: string
  offer_version: number
}

export interface ReferenceCurveBatchReceipt {
  schema: string
  batch_id: string
  batch_digest: string
  batch_material_digest: string
  curve_id: string
  curve_version: number
  entry_set_digest: string
  entries: ReferenceCurveEntryReceipt[]
  status: ReferenceCurveBatchStatus
  submitted_by_admin_user_id: string
  submitted_at: string
  updated_at: string
  replayed: boolean
  market_effect: 'none'
}

export interface ReferenceCurveReviewReceipt {
  schema: string
  review_id: string
  review_digest: string
  batch_id: string
  batch_digest: string
  batch_material_digest: string
  curve_id: string
  curve_version: number
  entry_set_digest: string
  decision: ReferenceCurveReviewDecision
  reviewed_by_admin_user_id: string
  reviewed_at: string
  replayed: boolean
  market_effect: 'none'
}

export interface ReferenceCurveSnapshotBindingReceipt {
  schema: string
  binding_id: string
  binding_digest: string
  application_id: string
  batch_id: string
  review_id: string
  entry_id: string
  entry_digest: string
  ordinal: number
  snapshot_id: string
  snapshot_digest: string
  quote_id: string
  source_kind: 'fallback_curve'
  source_id: string
  source_version: number
  source_digest: string
  quoted_at: string
  expires_at: string
  status: 'snapshot_registered'
}

export interface ReferenceCurveApplicationReceipt {
  schema: string
  application_id: string
  application_digest: string
  batch_id: string
  batch_digest: string
  batch_material_digest: string
  review_id: string
  review_digest: string
  curve_id: string
  curve_version: number
  binding_set_digest: string
  bindings: ReferenceCurveSnapshotBindingReceipt[]
  submitted_by_admin_user_id: string
  reviewed_by_admin_user_id: string
  applied_by_admin_user_id: string
  status: 'applied'
  applied_at: string
  replayed: boolean
  market_effect: 'price_snapshots_registered'
  job_effect: 'none'
  reservation_effect: 'none'
  capacity_effect: 'none'
  funds_effect: 'none'
  settlement_effect: 'none'
}

export interface ReferenceCurveBatchDetail {
  batch: ReferenceCurveBatchReceipt
  review: ReferenceCurveReviewReceipt | null
  application: ReferenceCurveApplicationReceipt | null
}

export interface ReferenceCurvePreflightReport {
  schema: string
  batch_id: string
  batch_digest: string
  curve_id: string
  curve_version: number
  submitted_by_admin_user_id: string
  batch_status: ReferenceCurveBatchStatus
  checked_at: string
  entry_count: number
  review_present: boolean
  application_present: boolean
  admin_review_allowed: boolean
  admin_apply_allowed: boolean
  blockers: string[]
  market_effect: 'none'
}

export interface SubmitReferenceCurveBody {
  idempotency_key: string
  curve_id: string
  curve_version: number
  valid_from: string
  valid_until: string
  quote_ttl_seconds: number
  entries: ReferenceCurveEntryIntent[]
  submission_note: string
  confirm_submission: true
}

interface ReferenceCurveListResponse {
  reference_curve_batches: ReferenceCurveBatchDetail[]
}

const base = '/api/admin/compute/platform-reference-price-curves'

export const computeReferenceCurveApi = {
  list: (status?: ReferenceCurveBatchStatus, limit = 50) => {
    const query = new URLSearchParams({ limit: String(limit) })
    if (status) query.set('status', status)
    return api.get<ReferenceCurveListResponse>(`${base}?${query.toString()}`)
      .then((response) => response.reference_curve_batches.map((detail) => detail.batch))
  },
  get: (batchId: string) => api.get<ReferenceCurveBatchDetail>(`${base}/${encodeURIComponent(batchId)}`),
  preflight: (batchId: string) => api.get<ReferenceCurvePreflightReport>(`${base}/${encodeURIComponent(batchId)}/preflight`),
  submit: (body: SubmitReferenceCurveBody) => api.post<ReferenceCurveBatchReceipt>(base, body),
  review: (detail: ReferenceCurveBatchDetail, decision: ReferenceCurveReviewDecision, reviewNote: string | null) =>
    api.post<ReferenceCurveReviewReceipt>(`${base}/${encodeURIComponent(detail.batch.batch_id)}/review`, {
      idempotency_key: `reference-curve-review:${detail.batch.batch_digest}:${decision}`,
      expected_batch_digest: detail.batch.batch_digest,
      expected_batch_material_digest: detail.batch.batch_material_digest,
      decision,
      review_note: reviewNote,
      confirm_review: true,
    }),
  apply: (detail: ReferenceCurveBatchDetail, applyNote: string) => {
    if (!detail.review) throw new Error('当前批次缺少独立复核回执')
    return api.post<ReferenceCurveApplicationReceipt>(`${base}/${encodeURIComponent(detail.batch.batch_id)}/application`, {
      idempotency_key: `reference-curve-apply:${detail.review.review_digest}`,
      expected_batch_digest: detail.batch.batch_digest,
      expected_batch_material_digest: detail.batch.batch_material_digest,
      expected_review_id: detail.review.review_id,
      expected_review_digest: detail.review.review_digest,
      apply_note: applyNote,
      confirm_application: true,
    })
  },
}
