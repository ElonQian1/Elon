import { api } from '../../api/client'
import type {
  AssignmentArtifactResponse,
  AssignmentAction,
  AssignmentActionPayload,
  AutomationMatterResponse,
  AvailableGroupAiNode,
  BotsResponse,
  CreateMergeRequestPayload,
  CreateMatterPlanPayload,
  MatterGovernanceResponse,
  MatterAutomationAction,
  MatterDetailResponse,
  MatterEventsDeltaResponse,
  MergeApplyResponse,
  MergeGateResponse,
  MergeRequestResponse,
  MattersResponse,
  MatterPolicyResponse,
  NodesResponse,
  ApplyMergeRequestPayload,
  UpdateMatterBudgetPolicyPayload,
  UpdateMergeRequestPayload,
} from './types'

const projectPath = (projectId: string, suffix: string) =>
  `/api/projects/${encodeURIComponent(projectId)}/ai/${suffix}`

export async function loadGroupAiNodes(projectId: string) {
  const data = await api.get<NodesResponse>(projectPath(projectId, 'available-nodes'))
  return data.nodes ?? []
}

export async function authorizeGroupAiNode(projectId: string, node: AvailableGroupAiNode) {
  return api.post(projectPath(projectId, 'node-authorizations'), {
    nodeId: node.node_id,
    allowedClis: node.allowed_clis,
    permissionLevel: node.authorization?.permission_level ?? 'project_write',
    enabled: true,
  })
}

export async function loadGroupAiBots(projectId: string) {
  const data = await api.get<BotsResponse>(projectPath(projectId, 'bots'))
  return data.bots ?? []
}

export async function loadGroupAiMatters(projectId: string) {
  const data = await api.get<MattersResponse>(projectPath(projectId, 'matters?limit=50'))
  return data.matters ?? []
}

export async function createMatterPlan(projectId: string, payload: CreateMatterPlanPayload) {
  return api.post<MatterDetailResponse>(projectPath(projectId, 'matters/plan'), payload)
}

export async function loadMatterDetail(projectId: string, matterId: string) {
  return api.get<MatterDetailResponse>(
    projectPath(projectId, `matters/${encodeURIComponent(matterId)}`),
  )
}

export async function loadMatterEvents(projectId: string, matterId: string, after = '') {
  const query = after ? `?after=${encodeURIComponent(after)}` : ''
  return api.get<MatterEventsDeltaResponse>(
    projectPath(projectId, `matters/${encodeURIComponent(matterId)}/events${query}`),
  )
}

export async function loadMatterGovernance(projectId: string, matterId: string) {
  return api.get<MatterGovernanceResponse>(
    projectPath(projectId, `matters/${encodeURIComponent(matterId)}/governance`),
  )
}

export async function postMatterAction(
  projectId: string,
  matterId: string,
  action: 'approve' | 'start' | 'request-changes' | 'accept' | 'cancel',
  comment = '',
) {
  return api.post<MatterDetailResponse>(
    projectPath(projectId, `matters/${encodeURIComponent(matterId)}/${action}`),
    { comment },
  )
}

export async function postAssignmentAction(
  projectId: string,
  matterId: string,
  assignmentId: string,
  action: AssignmentAction,
  payload: AssignmentActionPayload = {},
) {
  return api.post<MatterDetailResponse>(
    projectPath(
      projectId,
      `matters/${encodeURIComponent(matterId)}/assignments/${encodeURIComponent(assignmentId)}/${action}`,
    ),
    payload,
  )
}

export async function postMatterAutomation(
  projectId: string,
  matterId: string,
  action: MatterAutomationAction,
  comment = '',
) {
  return api.post<AutomationMatterResponse>(
    projectPath(projectId, `matters/${encodeURIComponent(matterId)}/${action}`),
    { comment },
  )
}

export async function loadAssignmentArtifact(
  projectId: string,
  matterId: string,
  assignmentId: string,
) {
  return api.get<AssignmentArtifactResponse>(
    projectPath(
      projectId,
      `matters/${encodeURIComponent(matterId)}/assignments/${encodeURIComponent(assignmentId)}/artifact`,
    ),
  )
}

export async function createMatterMergeRequest(
  projectId: string,
  matterId: string,
  payload: CreateMergeRequestPayload,
) {
  return api.post<MergeRequestResponse>(
    projectPath(projectId, `matters/${encodeURIComponent(matterId)}/merge-requests`),
    payload,
  )
}

export async function updateMatterMergeRequest(
  projectId: string,
  matterId: string,
  mergeRequestId: string,
  payload: UpdateMergeRequestPayload,
) {
  return api.post<MergeRequestResponse>(
    projectPath(
      projectId,
      `matters/${encodeURIComponent(matterId)}/merge-requests/${encodeURIComponent(mergeRequestId)}`,
    ),
    payload,
  )
}

export async function updateMatterBudgetPolicy(
  projectId: string,
  matterId: string,
  payload: UpdateMatterBudgetPolicyPayload,
) {
  return api.post<MatterPolicyResponse>(
    projectPath(projectId, `matters/${encodeURIComponent(matterId)}/budget-policy`),
    payload,
  )
}

export async function checkMatterMergeRequest(
  projectId: string,
  matterId: string,
  mergeRequestId: string,
) {
  return api.post<MergeGateResponse>(
    projectPath(
      projectId,
      `matters/${encodeURIComponent(matterId)}/merge-requests/${encodeURIComponent(mergeRequestId)}/check`,
    ),
    {},
  )
}

export async function applyMatterMergeRequest(
  projectId: string,
  matterId: string,
  mergeRequestId: string,
  payload: ApplyMergeRequestPayload = {},
) {
  return api.post<MergeApplyResponse>(
    projectPath(
      projectId,
      `matters/${encodeURIComponent(matterId)}/merge-requests/${encodeURIComponent(mergeRequestId)}/apply`,
    ),
    payload,
  )
}
