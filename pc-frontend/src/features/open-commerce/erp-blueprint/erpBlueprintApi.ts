import { api } from '../../../api/client'
import type {
  ErpBlueprint,
  ErpBlueprintVersion,
  ErpInstance,
  ErpMaterializationStatus,
  ErpOverview,
  ErpProposal,
  ErpReleaseManifest,
  ErpUpgrade,
  DecideErpUpgradeRequest,
  RequirementResolution,
  UpdateErpInstanceRequest,
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

  evolveBlueprint: (projectId: string, blueprintId: string, request: Record<string, unknown>) =>
    api.post<ErpBlueprint>(
      `${base(projectId)}/blueprints/${encodeURIComponent(blueprintId)}/evolve`,
      request,
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

  updateInstanceConfiguration: (
    projectId: string,
    instanceId: string,
    request: UpdateErpInstanceRequest,
  ) => api.post<ErpInstance>(
    `${base(projectId)}/instances/${encodeURIComponent(instanceId)}/configuration`,
    request,
  ),

  createInstanceBootstrapMatter: (projectId: string, instanceId: string) =>
    api.post<{ instance: ErpInstance; matter_id: string }>(
      `${base(projectId)}/instances/${encodeURIComponent(instanceId)}/bootstrap-matter`,
      {},
    ),

  materializationStatus: (projectId: string, instanceId: string) =>
    api.get<ErpMaterializationStatus>(
      `${base(projectId)}/instances/${encodeURIComponent(instanceId)}/materialization`,
    ),

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
    request: DecideErpUpgradeRequest,
  ) => api.post<ErpUpgrade>(
    `${base(projectId)}/upgrades/${encodeURIComponent(campaignId)}/decision`,
    request,
  ),
}
