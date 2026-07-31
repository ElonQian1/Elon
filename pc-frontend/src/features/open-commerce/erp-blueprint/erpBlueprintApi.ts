import { api } from '../../../api/client'
import type {
  ErpBlueprint,
  ErpBlueprintVersion,
  ErpInstance,
  ErpOverview,
  ErpProposal,
  ErpReleaseManifest,
  ErpUpgrade,
  RequirementResolution,
} from './erpBlueprintTypes'

function base(projectId: string) {
  return `/api/projects/${encodeURIComponent(projectId)}/erp`
}
export const erpBlueprintApi = {
  overview: (projectId: string) => api.get<ErpOverview>(`${base(projectId)}/overview`),

  createBlueprint: (projectId: string, request: Record<string, unknown>) =>
    api.post<ErpBlueprint>(`${base(projectId)}/blueprints`, request),

  publishVersion: (projectId: string, blueprintId: string, manifest: ErpReleaseManifest) =>
    api.post<ErpBlueprintVersion>(
      `${base(projectId)}/blueprints/${encodeURIComponent(blueprintId)}/versions`,
      { manifest },
    ),

  createInstance: (projectId: string, blueprintId: string, request: Record<string, unknown>) =>
    api.post<ErpInstance>(
      `${base(projectId)}/blueprints/${encodeURIComponent(blueprintId)}/instances`,
      request,
    ),

  resolveRequirement: (projectId: string, request: Record<string, unknown>) =>
    api.post<RequirementResolution>(`${base(projectId)}/requirements/resolve`, request),

  submitSignal: (
    projectId: string,
    instanceId: string,
    request: Record<string, unknown>,
  ) => api.post(`${base(projectId)}/instances/${encodeURIComponent(instanceId)}/signals`, request),

  decideProposal: (
    projectId: string,
    proposalId: string,
    request: { decision: 'accepted' | 'rejected'; note: string; create_matter: boolean },
  ) => api.post<{ proposal: ErpProposal; matter_id?: string | null }>(
    `${base(projectId)}/proposals/${encodeURIComponent(proposalId)}/decision`,
    request,
  ),

  createProposalMatter: (projectId: string, proposalId: string) =>
    api.post<{ proposal: ErpProposal; matter_id: string }>(
      `${base(projectId)}/proposals/${encodeURIComponent(proposalId)}/matter`,
      {},
    ),

  prepareUpgrade: (projectId: string, instanceId: string, targetVersion: string) =>
    api.post<ErpUpgrade>(
      `${base(projectId)}/instances/${encodeURIComponent(instanceId)}/upgrades`,
      { target_version: targetVersion },
    ),

  decideUpgrade: (
    projectId: string,
    campaignId: string,
    action: 'adopt' | 'rollback',
    reason = '',
  ) => api.post<ErpUpgrade>(
    `${base(projectId)}/upgrades/${encodeURIComponent(campaignId)}/decision`,
    { action, reason },
  ),
}
