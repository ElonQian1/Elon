import { useCallback, useEffect, useMemo, useState } from 'react'
import { useNavigate, useSearchParams } from 'react-router-dom'
import {
  AlertTriangle,
  CheckCircle2,
  Copy,
  ExternalLink,
  GitBranch,
  Loader2,
  MessageSquareText,
  PlayCircle,
  RefreshCw,
  SearchCode,
} from 'lucide-react'
import { fetchProjectGitWorktreeAudit } from './api'
import type {
  ProjectGitWorktreeAuditEntry,
  ProjectGitWorktreeAuditResponse,
  ProjectGitWorktreeConversation,
} from './types'
import { saveProjectComposerDraft } from '../updates/composerDrafts'
import { useProjectStore } from '../conversation/useProjectStore'
import { useAuthStore } from '../../store/auth'
import type { MemberConversationTarget } from '../conversation/memberConversationApi'
import styles from './GitWorktreesPage.module.css'

type DraftMode = 'open' | 'ask' | 'continue'

export default function GitWorktreesPage() {
  const navigate = useNavigate()
  const [searchParams, setSearchParams] = useSearchParams()
  const projects = useProjectStore((s) => s.projects)
  const projectsLoaded = useProjectStore((s) => s.projectsLoaded)
  const loadProjects = useProjectStore((s) => s.loadProjects)
  const activeProjectId = useProjectStore((s) => s.activeProjectId)
  const user = useAuthStore((s) => s.user)
  const [selectedProjectId, setSelectedProjectId] = useState(searchParams.get('project') ?? '')
  const [audit, setAudit] = useState<ProjectGitWorktreeAuditResponse | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const [copiedPath, setCopiedPath] = useState('')

  useEffect(() => {
    if (!projectsLoaded) loadProjects().catch(() => {})
  }, [loadProjects, projectsLoaded])

  useEffect(() => {
    const queryProject = searchParams.get('project') ?? ''
    if (queryProject && queryProject !== selectedProjectId) {
      setSelectedProjectId(queryProject)
      return
    }
    if (!selectedProjectId) {
      const fallback = activeProjectId || projects[0]?.id || ''
      if (fallback) setSelectedProjectId(fallback)
    }
  }, [activeProjectId, projects, searchParams, selectedProjectId])

  const selectedProject = useMemo(
    () => projects.find((project) => project.id === selectedProjectId),
    [projects, selectedProjectId],
  )

  const loadAudit = useCallback(async () => {
    if (!selectedProjectId) return
    setLoading(true)
    setError('')
    try {
      const data = await fetchProjectGitWorktreeAudit(selectedProjectId)
      setAudit(data)
    } catch (err) {
      setAudit(null)
      setError((err as { message?: string }).message ?? 'Git 现场读取失败')
    } finally {
      setLoading(false)
    }
  }, [selectedProjectId])

  useEffect(() => {
    if (selectedProjectId) void loadAudit()
  }, [loadAudit, selectedProjectId])

  function selectProject(projectId: string) {
    setSelectedProjectId(projectId)
    setAudit(null)
    setError('')
    setSearchParams(projectId ? { project: projectId } : {})
  }

  async function copyPath(path: string) {
    try {
      await navigator.clipboard.writeText(path)
      setCopiedPath(path)
      window.setTimeout(() => setCopiedPath(''), 1200)
    } catch {
      setCopiedPath('')
    }
  }

  async function openConversation(entry: ProjectGitWorktreeAuditEntry, mode: DraftMode) {
    const conversation = entry.conversation
    if (!selectedProjectId || !conversation) return
    const store = useProjectStore.getState()
    if (store.activeProjectId !== selectedProjectId) {
      await store.selectProject(selectedProjectId)
    }
    const fresh = useProjectStore.getState()
    const channel = fresh.channels.find((item) => item.kind === 'ai_development') ?? fresh.channels[0]
    if (channel?.id) {
      await fresh.selectChannel(channel.id)
    }
    const target = conversationTarget(conversation, user?.id)
    saveProjectComposerDraft({
      userId: user?.id,
      input: draftText(mode, entry),
      attachments: [],
      draftConversationId: conversation.conversation_id,
      activeProjectId: selectedProjectId,
      activeChannelId: channel?.id ?? '',
      sessionView: conversation.conversation_id,
      conversationTarget: target,
    })
    navigate('/')
  }

  const dirtyCount = audit?.summary.dirty_worktrees ?? 0
  const unknownCount = audit?.summary.unknown_dirty_worktrees ?? 0

  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <div>
          <div className={styles.kicker}>Git Worktree Audit</div>
          <h1>Git 现场</h1>
        </div>
        <div className={styles.headerActions}>
          <select
            value={selectedProjectId}
            onChange={(event) => selectProject(event.target.value)}
            className={styles.projectSelect}
            aria-label="选择项目"
          >
            {!selectedProjectId && <option value="">选择项目</option>}
            {projects.map((project) => (
              <option key={project.id} value={project.id}>{project.name}</option>
            ))}
          </select>
          <button className={styles.iconBtn} onClick={loadAudit} disabled={!selectedProjectId || loading} title="刷新 Git 现场" type="button">
            {loading ? <Loader2 size={16} className={styles.spin} aria-hidden="true" /> : <RefreshCw size={16} aria-hidden="true" />}
            <span>刷新</span>
          </button>
        </div>
      </header>

      <section className={styles.summaryBar}>
        <Metric label="工作树" value={audit?.summary.total_worktrees ?? 0} />
        <Metric label="脏工作树" value={dirtyCount} tone={dirtyCount ? 'warn' : 'ok'} />
        <Metric label="未提交/未跟踪条目" value={audit?.summary.uncommitted_entries ?? 0} tone={dirtyCount ? 'warn' : undefined} />
        <Metric label="已归属" value={audit?.summary.matched_worktrees ?? 0} />
        <Metric label="未知脏现场" value={unknownCount} tone={unknownCount ? 'bad' : 'ok'} />
      </section>

      {selectedProject && (
        <section className={styles.projectStrip}>
          <GitBranch size={16} aria-hidden="true" />
          <span>{selectedProject.name}</span>
          <code>{audit?.workspace_path ?? selectedProject.workspace_path ?? '-'}</code>
        </section>
      )}

      {error && (
        <div className={styles.errorBox}>
          <AlertTriangle size={16} aria-hidden="true" />
          <span>{error}</span>
        </div>
      )}

      {audit?.warnings?.length ? (
        <div className={styles.warningList}>
          {audit.warnings.map((warning) => (
            <span key={warning}><AlertTriangle size={14} aria-hidden="true" />{warning}</span>
          ))}
        </div>
      ) : null}

      <main className={styles.list}>
        {loading && !audit && <div className={styles.loading}>正在读取节点 Git worktree 列表…</div>}
        {!loading && !audit && !error && <div className={styles.empty}>选择项目后查看 Git 工作现场</div>}
        {audit?.worktrees.map((entry) => (
          <WorktreeRow
            key={`${entry.path}-${entry.branch ?? ''}`}
            entry={entry}
            copied={copiedPath === entry.path}
            onCopy={() => copyPath(entry.path)}
            onOpen={(mode) => openConversation(entry, mode)}
          />
        ))}
      </main>
    </div>
  )
}

function Metric({ label, value, tone }: { label: string; value: number; tone?: 'ok' | 'warn' | 'bad' }) {
  return (
    <div className={styles.metric} data-tone={tone}>
      <strong>{value}</strong>
      <span>{label}</span>
    </div>
  )
}

function WorktreeRow({
  entry,
  copied,
  onCopy,
  onOpen,
}: {
  entry: ProjectGitWorktreeAuditEntry
  copied: boolean
  onCopy: () => void
  onOpen: (mode: DraftMode) => void
}) {
  const conversation = entry.conversation
  const dirty = entry.has_uncommitted_changes
  return (
    <article className={styles.worktree} data-dirty={dirty ? 'true' : undefined} data-current={entry.current ? 'true' : undefined}>
      <div className={styles.worktreeMain}>
        <div className={styles.statusIcon} data-state={dirty ? 'dirty' : 'clean'}>
          {dirty ? <AlertTriangle size={16} aria-hidden="true" /> : <CheckCircle2 size={16} aria-hidden="true" />}
        </div>
        <div className={styles.pathBlock}>
          <div className={styles.pathLine}>
            <code title={entry.path}>{entry.path}</code>
            {entry.current && <span className={styles.badge}>当前</span>}
            {entry.bare && <span className={styles.badge}>bare</span>}
            {entry.detached && <span className={styles.badge}>detached</span>}
          </div>
          <div className={styles.metaLine}>
            <span>{entry.branch ?? '无分支'}</span>
            <span>{entry.head ?? '无 HEAD'}</span>
            <span>{entry.uncommitted_count} 项改动</span>
            <span>{entry.untracked_count} 个未跟踪</span>
          </div>
        </div>
      </div>

      <div className={styles.ownerBlock}>
        {conversation ? (
          <>
            <strong>{conversation.title || conversation.conversation_id}</strong>
            <span>{conversation.user_account || conversation.user_id}</span>
            <em>{matchLabel(conversation.match_kind)} · {conversation.match_confidence}%</em>
          </>
        ) : (
          <>
            <strong>{entry.current ? '项目主工作区' : '未识别会话'}</strong>
            <span>{entry.current ? '不是会话 worktree' : '需要人工确认来源'}</span>
            <em>{entry.recommended_action}</em>
          </>
        )}
      </div>

      <div className={styles.actions}>
        <button className={styles.iconBtn} onClick={onCopy} title="复制路径" type="button">
          <Copy size={15} aria-hidden="true" />
          <span>{copied ? '已复制' : '路径'}</span>
        </button>
        <button className={styles.iconBtn} onClick={() => onOpen('open')} disabled={!conversation} title="打开会话" type="button">
          <ExternalLink size={15} aria-hidden="true" />
          <span>打开</span>
        </button>
        <button className={styles.iconBtn} onClick={() => onOpen('ask')} disabled={!conversation} title="询问此会话" type="button">
          <MessageSquareText size={15} aria-hidden="true" />
          <span>询问</span>
        </button>
        <button className={styles.iconBtn} onClick={() => onOpen('continue')} disabled={!conversation} title="继续处理草稿" type="button">
          <PlayCircle size={15} aria-hidden="true" />
          <span>继续</span>
        </button>
      </div>

      {(entry.status_error || entry.status_preview?.length) && (
        <details className={styles.statusPreview}>
          <summary><SearchCode size={14} aria-hidden="true" />状态预览</summary>
          {entry.status_error
            ? <p>{entry.status_error}</p>
            : (
              <pre>
                {(entry.status_preview ?? []).slice(0, 12).join('\n')}
                {entry.status_truncated ? '\n...' : ''}
              </pre>
            )}
        </details>
      )}
    </article>
  )
}

function conversationTarget(
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

function draftText(mode: DraftMode, entry: ProjectGitWorktreeAuditEntry) {
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

function matchLabel(kind: string) {
  const labels: Record<string, string> = {
    active_workspace_path: '路径记录',
    branch: '分支记录',
    platform_branch_convention: '平台分支',
    platform_path_convention: '平台路径',
  }
  return labels[kind] ?? kind
}
