import { api } from '../../api/client'
import type { DeveloperProductionReadinessSummary } from './developerProductionReadinessTypes'

function projectBase(projectId: string) {
  return `/api/projects/${encodeURIComponent(projectId)}/open-commerce`
}

export const developerProductionReadinessClientApi = {
  developerProductionReadiness: (projectId: string, appRecordId: string) =>
    api.get<DeveloperProductionReadinessSummary>(
      `${projectBase(projectId)}/developer-apps/${encodeURIComponent(appRecordId)}/production-readiness`,
    ),
}
