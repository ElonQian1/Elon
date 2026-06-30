import { api } from '../../api/client'
import type {
  AssignmentAction,
  AssignmentActionPayload,
  AvailableGroupAiNode,
  BotsResponse,
  CreateMatterPlanPayload,
  MatterDetailResponse,
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
