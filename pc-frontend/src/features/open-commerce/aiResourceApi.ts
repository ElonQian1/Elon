import { api } from '../../api/client'
import type {
  AiResourceOverview,
  AiResourcePolicy,
  AiRoutePreview,
} from './aiResourceTypes'

function base(projectId: string) {
  return `/api/projects/${encodeURIComponent(projectId)}/ai-resources`
}

export const aiResourceApi = {
  overview: (projectId: string) =>
    api.get<AiResourceOverview>(`${base(projectId)}/overview`),
  updatePolicy: (projectId: string, policy: Omit<AiResourcePolicy, 'project_id' | 'updated_by_user_id' | 'created_at' | 'updated_at'>) =>
    api.patch<AiResourcePolicy>(`${base(projectId)}/policy`, policy),
  preview: (
    projectId: string,
    request: { task_kind: string; preferred_model?: string; require_local_execution: boolean },
  ) => api.post<AiRoutePreview>(`${base(projectId)}/preview`, request),
}
