import { api } from '../../api/client'
import type { ProjectGitWorktreeAuditResponse } from './types'

export function fetchProjectGitWorktreeAudit(projectId: string) {
  return api.get<ProjectGitWorktreeAuditResponse>(
    `/api/projects/${encodeURIComponent(projectId)}/git/worktrees/audit`,
  )
}
