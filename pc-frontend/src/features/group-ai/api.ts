import { api } from '../../api/client'
import type {
  AssignmentArtifactResponse,
  AssignmentAction,
  AssignmentActionPayload,
  AutomationMatterResponse,
  AvailableGroupAiNode,
  BotsResponse,
  CreateMatterPlanPayload,
  MatterAutomationAction,
  MatterDetailResponse,
  MatterEventsDeltaResponse,
  MattersResponse,
  NodesResponse,
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
