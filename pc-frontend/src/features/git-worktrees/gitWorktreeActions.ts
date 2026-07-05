import type { MemberConversationTarget } from '../conversation/memberConversationApi'
import type { ProjectGitWorktreeAuditEntry, ProjectGitWorktreeConversation } from './types'

export type DraftMode = 'open' | 'ask' | 'continue'

export function conversationTarget(
  conversation: ProjectGitWorktreeConversation,
  currentUserId?: string,
): MemberConversationTarget | null {
  if (conversation.user_id === currentUserId) return null
  return {
    userId: conversation.user_id,
    account: conversation.user_account || conversation.user_id,
    avatarDataUrl: null,
  }
}

export function draftText(mode: DraftMode, entry: ProjectGitWorktreeAuditEntry) {
  if (mode === 'open') return ''
  const conversation = entry.conversation
  const header = [
    `worktree: ${entry.path}`,
    `branch: ${entry.branch ?? '-'}`,
    `HEAD: ${entry.head ?? '-'}`,
    `未提交/未跟踪: ${entry.uncommitted_count}/${entry.untracked_count}`,
    conversation?.codex_thread_id ? `Codex thread: ${conversation.codex_thread_id}` : '',
  ].filter(Boolean).join('\n')
  if (mode === 'ask') {
    return `${header}\n\n请只读检查这个会话的工作现场，回答做到哪里了、为什么还有未提交/未跟踪、是否应提交或清理；不要修改、不要提交、不要清理。`
  }
  return `${header}\n\n继续处理这个会话的遗留 Git 现场。先只读确认状态和最近上下文，再按项目规则说明下一步；如果需要提交或清理，先列出范围。`
}

export function matchLabel(kind: string) {
  const labels: Record<string, string> = {
    active_workspace_path: '路径记录',
    branch: '分支记录',
    platform_branch_convention: '平台分支',
    platform_path_convention: '平台路径',
  }
  return labels[kind] ?? kind
}
