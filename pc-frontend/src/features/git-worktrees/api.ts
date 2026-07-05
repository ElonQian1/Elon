import { api } from '../../api/client'
import type { GlobalGitWorktreeAuditResponse, ProjectGitWorktreeAuditResponse } from './types'

export function fetchGlobalGitWorktreeAudit() {
  return api.get<GlobalGitWorktreeAuditResponse>('/api/git/worktrees/audit')
}

export function fetchProjectGitWorktreeAudit(projectId: string) {
  return api.get<ProjectGitWorktreeAuditResponse>(
    `/api/projects/${encodeURIComponent(projectId)}/git/worktrees/audit`,
  )
}
