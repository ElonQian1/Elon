import { api } from '../../api/client'
import { type MyComputePriceSnapshotView } from '../compute-supply/computePriceSnapshotApi'

export interface ComputeUsageLimit { meter: string; max_quantity: number }

export interface ComputeWorkloadSpec {
  schema: 'compute_federation.workload.v1'
  task_kind: string
  input_artifacts: unknown[]
  model: null
  runtime: null
  resources: {
    accelerator_kinds: string[]
    min_accelerator_count: number
    min_vram_bytes: number
    min_ram_bytes: number
    min_disk_bytes: number
    max_runtime_seconds: number
    allow_network_egress: boolean
  }
  output: {
    media_type: string
    max_output_bytes: number
    streaming: boolean
    result_artifact_required: boolean
    deterministic_digest_expected: boolean
  }
  usage_limits: ComputeUsageLimit[]
  data_class: string
  shard: null
  retry_policy: { max_attempts: number; initial_backoff_ms: number; max_backoff_ms: number; retryable_error_codes: string[] }
  checkpoint_policy: { mode: 'disabled'; interval_seconds: null; max_checkpoints: 0; checkpoint_media_type: null }
  verification_policy: {
    verification_tier: string
    minimum_independent_receipts: number
    duplicate_sample_rate_basis_points: number
    challenge_profile_id: null
    require_server_metering: boolean
  }
  deadline_at: string
}

export interface ComputeProviderScope {
  allowed_provider_ids: string[]
  allowed_provider_kinds: string[]
  excluded_provider_ids: string[]
  required_trust_tier: string
  required_regions: string[]
}

export interface CreateComputeJobBody {
  job_id: string
  idempotency_key: string
  merchant_id: null
  workload: ComputeWorkloadSpec
  provider_scope: ComputeProviderScope
  max_consumer_charge_micros: number
  currency: string
}

export interface ComputeJobReceipt {
  job: {
    schema: string
    job_id: string
    project_id: string | null
    merchant_id: string | null
    consumer_account_id: string
    idempotency_key: string
    workload: ComputeWorkloadSpec
    provider_scope: ComputeProviderScope
    status: string
    selected_offer: { provider_id: string; offer_id: string; offer_version: number; offer_digest: string } | null
    price_snapshot_id: string | null
    max_consumer_charge_micros: number
    currency: string
    submitted_at: string
    updated_at: string
  }
  revision: number
  job_digest: string
  replayed: boolean
}

export interface ComputeQuoteCandidate {
  offer: { provider_id: string; offer_id: string; offer_version: number; offer_digest: string }
  price_snapshot: MyComputePriceSnapshotView['snapshot']
  provider: {
    provider_id: string
    provider_kind: string
    display_name: string
    trust_tier: string
    home_region: string | null
    policy_revision: number
    provider_digest: string
  }
}

export interface ComputeQuoteCandidatePage {
  job_id: string
  job_revision: number
  job_digest: string
  candidates: ComputeQuoteCandidate[]
  scanned_count: number
  scan_truncated: boolean
}

export interface ComputeCapacityClaimBinding {
  claim_id: string
  claim_revision: number
  claim_digest: string
}

export interface ComputeJobVersionBinding {
  job_id: string
  job_revision: number
  job_digest: string
}

export interface ComputeReservedCapacity {
  meter: string
  quantity: number
}

export interface ComputeReservationReceipt {
  reservation: {
    schema: string
    reservation_id: string
    job: ComputeJobVersionBinding
    idempotency_key: string
    offer: { provider_id: string; offer_id: string; offer_version: number; offer_digest: string }
    price_snapshot: MyComputePriceSnapshotView['snapshot']
    capacity_claim: ComputeCapacityClaimBinding
    reserved_capacity: ComputeReservedCapacity[]
    consumer_authorization_ref: string
    status: string
    created_at: string
    updated_at: string
    expires_at: string
    consumed_at: string | null
    released_at: string | null
  }
  revision: number
  reservation_digest: string
  replayed: boolean
}

export interface ReserveComputeJobBody {
  reservation_id: string
  idempotency_key: string
  job_id: string
  expected_job_revision: number
  expected_job_digest: string
  reserved_capacity: ComputeReservedCapacity[]
  expires_at: string
}

export interface ComputeBrokerReservationReceipt {
  reservation_id: string
  consumer_account_id: string
  budget_adapter: string
  budget_reservation_id: string
  budget_reserved_fen: number
  capacity_claim: ComputeCapacityClaimBinding
  reserved_job: ComputeJobVersionBinding
  reservation_revision: number
  reservation_digest: string
  status: string
  replayed: boolean
}

export interface ComputeBrokerFinishReceipt {
  reservation_id: string
  consumer_account_id: string
  action: 'release' | 'expire'
  budget_reservation_id: string
  budget_refunded_fen: number
  capacity_claim: ComputeCapacityClaimBinding
  terminal_job: ComputeJobVersionBinding
  reservation_revision: number
  reservation_digest: string
  status: string
  recorded_at: string
  replayed: boolean
}

interface JobListResponse { jobs: ComputeJobReceipt[] }
interface ReservationListResponse { reservations: ComputeReservationReceipt[] }

function projectBase(projectId: string) {
  return `/api/projects/${encodeURIComponent(projectId)}/compute/jobs`
}

export const computeMarketApi = {
  listJobs: (limit = 100) => api.get<JobListResponse>(`/api/me/compute/jobs?limit=${limit}`).then((response) => response.jobs),
  createJob: (projectId: string, body: CreateComputeJobBody) => api.post<ComputeJobReceipt>(projectBase(projectId), body),
  candidates: (projectId: string, jobId: string, limit = 50) => api.get<ComputeQuoteCandidatePage>(`${projectBase(projectId)}/${encodeURIComponent(jobId)}/quote-candidates?limit=${limit}`),
  quote: (projectId: string, jobId: string, candidate: ComputeQuoteCandidate, page: ComputeQuoteCandidatePage) =>
    api.post<ComputeJobReceipt>(`${projectBase(projectId)}/${encodeURIComponent(jobId)}/quote`, {
      offer_id: candidate.offer.offer_id,
      price_snapshot_id: candidate.price_snapshot.snapshot_id,
      expected_job_revision: page.job_revision,
      expected_job_digest: page.job_digest,
    }),
  listReservations: (limit = 100) =>
    api.get<ReservationListResponse>(`/api/me/compute/reservations?limit=${limit}`).then((response) => response.reservations),
  reserve: (body: ReserveComputeJobBody) =>
    api.post<ComputeBrokerReservationReceipt>('/api/me/compute/reservations', body),
  finishReservation: (receipt: ComputeReservationReceipt, action: 'release' | 'expire', idempotencyKey: string) =>
    api.post<ComputeBrokerFinishReceipt>(`/api/me/compute/reservations/${encodeURIComponent(receipt.reservation.reservation_id)}/${action}`, {
      idempotency_key: idempotencyKey,
      expected_reservation_revision: receipt.revision,
      expected_reservation_digest: receipt.reservation_digest,
    }),
}
