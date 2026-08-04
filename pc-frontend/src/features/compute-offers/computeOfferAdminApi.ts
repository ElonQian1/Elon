import { api } from '../../api/client'
import { type MyComputeOfferView } from '../compute-supply/computeOfferApi'

export interface ComputeOfferPublicationReceipt {
  schema: string
  publication_id: string
  offer_id: string
  provider_id: string
  pool_id: string
  source_offer_version: number
  source_offer_digest: string
  active_offer_version: number
  active_offer_digest: string
  provider_policy_revision: number
  provider_digest: string
  publication_digest: string
  approved_by_user_id: string
  published_at: string
  replayed: boolean
  offer_effect: 'active'
  price_snapshot_effect: 'none'
  capacity_effect: 'none'
  funds_effect: 'none'
}

export interface ComputeOfferLifecycleReceipt {
  schema: string
  event_id: string
  offer_id: string
  provider_id: string
  pool_id: string
  previous_status: string
  target_status: string
  previous_offer_version: number
  previous_offer_digest: string
  target_offer_version: number
  target_offer_digest: string
  reason: string
  event_digest: string
  changed_by_user_id: string
  changed_at: string
  replayed: boolean
  quote_candidate_effect: string
  reservation_effect: string
  attempt_effect: string
  funds_effect: 'none'
}

export type ComputeOfferAdminReceipt = ComputeOfferPublicationReceipt | ComputeOfferLifecycleReceipt
export type ComputeOfferAdminAction = 'publish' | 'drain' | 'expire' | 'revoke'

interface ComputeOfferDraftListResponse {
  offers: MyComputeOfferView[]
}

function base(offerId?: string) {
  return offerId ? `/api/admin/compute/offers/${encodeURIComponent(offerId)}` : '/api/admin/compute/offers'
}

export const computeOfferAdminApi = {
  drafts: (limit = 50) => api.get<ComputeOfferDraftListResponse>(`${base()}?limit=${limit}`)
    .then((response) => response.offers),
  get: (offerId: string) => api.get<MyComputeOfferView>(base(offerId)),
  publish: (view: MyComputeOfferView) => api.post<ComputeOfferPublicationReceipt>(`${base(view.offer.offer_id)}/publication`, {
    expected_offer_version: view.offer.offer_version,
    expected_offer_digest: view.offer.offer_digest,
    idempotency_key: `offer-publish:${view.offer.offer_digest}`,
    confirm_publish: true,
  }),
  transition: (view: MyComputeOfferView, action: Exclude<ComputeOfferAdminAction, 'publish'>, reason: string) =>
    api.post<ComputeOfferLifecycleReceipt>(`${base(view.offer.offer_id)}/${action}`, {
      expected_offer_version: view.offer.offer_version,
      expected_offer_digest: view.offer.offer_digest,
      reason,
      idempotency_key: `offer-${action}:${view.offer.offer_digest}`,
      ...(action === 'drain' ? { confirm_drain: true } : { confirm_terminal: true }),
    }),
}
