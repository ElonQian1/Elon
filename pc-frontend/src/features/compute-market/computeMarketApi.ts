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

interface JobListResponse { jobs: ComputeJobReceipt[] }

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
}
