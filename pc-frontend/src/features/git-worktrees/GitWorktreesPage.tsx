import { useCallback, useEffect, useMemo, useState } from 'react'
import { useNavigate, useSearchParams } from 'react-router-dom'
import { AlertTriangle, GitBranch, Loader2, RefreshCw, Search, X } from 'lucide-react'
import { fetchGlobalGitWorktreeAudit, fetchProjectGitWorktreeAudit } from './api'
import GitWorktreeRow from './GitWorktreeRow'
import { conversationTarget, draftText, type DraftMode } from './gitWorktreeActions'
import type {
  GlobalGitWorktreeAuditProjectResult,
  GlobalGitWorktreeAuditResponse,
  GlobalGitWorktreeAuditSummary,
  ProjectGitWorktreeAuditEntry,
  ProjectGitWorktreeAuditResponse,
} from './types'
import { saveProjectComposerDraft } from '../updates/composerDrafts'
import { useProjectStore } from '../conversation/useProjectStore'
import { useAuthStore } from '../../store/auth'
import { displayMessageContentOrAttachment } from '../../lib/messageDisplay'
import {
  listMemberConversationMessages,
  sendMemberConversationDiscussion,
  type MemberConversationMessage,
} from '../conversation/memberConversationApi'
import styles from './GitWorktreesPage.module.css'

type ScopeMode = 'all' | 'project'
type FilterMode = 'all' | 'dirty' | 'unknown'

interface ContextPreview {
  projectId: string
  projectName: string
  entry: ProjectGitWorktreeAuditEntry
  messages: MemberConversationMessage[]
}

export default function GitWorktreesPage() {
  const navigate = useNavigate()
  const [searchParams, setSearchParams] = useSearchParams()
  const projects = useProjectStore((s) => s.projects)
  const projectsLoaded = useProjectStore((s) => s.projectsLoaded)
  const loadProjects = useProjectStore((s) => s.loadProjects)
  const activeProjectId = useProjectStore((s) => s.activeProjectId)
  const user = useAuthStore((s) => s.user)
  const initialProject = searchParams.get('project') ?? ''
  const [scope, setScope] = useState<ScopeMode>(initialProject ? 'project' : 'all')
  const [selectedProjectId, setSelectedProjectId] = useState(initialProject)
  const [globalAudit, setGlobalAudit] = useState<GlobalGitWorktreeAuditResponse | null>(null)
  const [projectAudit, setProjectAudit] = useState<ProjectGitWorktreeAuditResponse | null>(null)
  const [filter, setFilter] = useState<FilterMode>('all')
  const [query, setQuery] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')
  const [copiedPath, setCopiedPath] = useState('')
  const [askingKey, setAskingKey] = useState('')
  const [contextLoadingKey, setContextLoadingKey] = useState('')
  const [contextPreview, setContextPreview] = useState<ContextPreview | null>(null)

  useEffect(() => {
    if (!projectsLoaded) loadProjects().catch(() => {})
  }, [loadProjects, projectsLoaded])

  useEffect(() => {
    const queryProject = searchParams.get('project') ?? ''
    if (queryProject) {
      setScope('project')
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
    if (scope === 'project' && !selectedProjectId) return
    setLoading(true)
    setError('')
    setNotice('')
    try {
      if (scope === 'all') {
        const data = await fetchGlobalGitWorktreeAudit()
        setGlobalAudit(data)
        setProjectAudit(null)
      } else {
        const data = await fetchProjectGitWorktreeAudit(selectedProjectId)
        setProjectAudit(data)
        setGlobalAudit(null)
      }
    } catch (err) {
      setGlobalAudit(null)
      setProjectAudit(null)
      setError((err as { message?: string }).message ?? 'Git 现场读取失败')
    } finally {
      setLoading(false)
    }
  }, [scope, selectedProjectId])

  useEffect(() => {
    if (scope === 'all' || selectedProjectId) void loadAudit()
  }, [loadAudit, scope, selectedProjectId])

  function switchScope(nextScope: ScopeMode) {
    setScope(nextScope)
    setGlobalAudit(null)
    setProjectAudit(null)
    setError('')
    setNotice('')
    setContextPreview(null)
    if (nextScope === 'all') {
      setSearchParams({})
      return
    }
    const projectId = selectedProjectId || activeProjectId || projects[0]?.id || ''
    if (projectId) {
      setSelectedProjectId(projectId)
      setSearchParams({ project: projectId })
    }
  }

  function selectProject(projectId: string) {
    setScope('project')
    setSelectedProjectId(projectId)
    setGlobalAudit(null)
    setProjectAudit(null)
    setError('')
    setNotice('')
    setContextPreview(null)
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

  async function openConversation(projectId: string, entry: ProjectGitWorktreeAuditEntry, mode: DraftMode) {
    const conversation = entry.conversation
    if (!projectId || !conversation) return
    const store = useProjectStore.getState()
    if (store.activeProjectId !== projectId) {
      await store.selectProject(projectId)
    }
    const fresh = useProjectStore.getState()
    const channel = fresh.channels.find((item) => item.kind === 'ai_development') ?? fresh.channels[0]
    if (channel?.id) {
      await fresh.selectChannel(channel.id)
    }
    saveProjectComposerDraft({
      userId: user?.id,
      input: draftText(mode, entry),
      attachments: [],
      draftConversationId: conversation.conversation_id,
      activeProjectId: projectId,
      activeChannelId: channel?.id ?? '',
      sessionView: conversation.conversation_id,
      conversationTarget: conversationTarget(conversation, user?.id),
    })
    navigate('/workspace')
  }

  async function askConversation(projectId: string, entry: ProjectGitWorktreeAuditEntry) {
    const conversation = entry.conversation
    if (!projectId || !conversation) return
    const key = rowKey(projectId, entry)
    setAskingKey(key)
    setError('')
    setNotice('')
    try {
      await sendMemberConversationDiscussion(
        projectId,
        conversation.user_id,
        conversation.conversation_id,
        draftText('ask', entry),
      )
      setNotice('已把询问发送到该会话的讨论消息。')
    } catch (err) {
      setError((err as { message?: string }).message ?? '发送询问失败')
    } finally {
      setAskingKey('')
    }
  }

  async function readConversation(project: GlobalGitWorktreeAuditProjectResult, entry: ProjectGitWorktreeAuditEntry) {
    const conversation = entry.conversation
    if (!conversation) return
    const key = rowKey(project.project.id, entry)
    setContextLoadingKey(key)
    setError('')
    try {
      const messages = await listMemberConversationMessages(
        project.project.id,
        conversation.user_id,
        conversation.conversation_id,
      )
      setContextPreview({
        projectId: project.project.id,
        projectName: project.project.name,
        entry,
        messages,
      })
    } catch (err) {
      setError((err as { message?: string }).message ?? '读取会话上下文失败')
    } finally {
      setContextLoadingKey('')
    }
  }

  const projectResults = useMemo(() => {
    if (scope === 'all') return globalAudit?.projects ?? []
    return projectAudit ? [singleProjectResult(projectAudit)] : []
  }, [globalAudit, projectAudit, scope])

  const summary = useMemo(() => {
    if (scope === 'all') return globalAudit?.summary ?? emptyGlobalSummary()
    if (!projectAudit) return emptyGlobalSummary()
    return summaryFromProject(projectAudit)
  }, [globalAudit, projectAudit, scope])

  const visibleProjects = useMemo(
    () => projectResults
      .map((project) => ({ ...project, worktrees: project.worktrees.filter((entry) => entryMatches(entry, query, filter)) }))
      .filter((project) => shouldShowProject(project, query, filter)),
    [filter, projectResults, query],
  )

  const hasAnyRows = visibleProjects.some((project) => project.worktrees.length > 0 || project.status !== 'audited')

  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <div>
          <div className={styles.kicker}>Git Worktree Audit</div>
          <h1>Git 现场</h1>
        </div>
        <div className={styles.headerActions}>
          <div className={styles.segmented} role="tablist" aria-label="审计范围">
            <button type="button" data-active={scope === 'all'} onClick={() => switchScope('all')}>全部</button>
            <button type="button" data-active={scope === 'project'} onClick={() => switchScope('project')}>项目</button>
          </div>
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
          <button className={styles.iconBtn} onClick={loadAudit} disabled={(scope === 'project' && !selectedProjectId) || loading} title="刷新 Git 现场" type="button">
            {loading ? <Loader2 size={16} className={styles.spin} aria-hidden="true" /> : <RefreshCw size={16} aria-hidden="true" />}
            <span>刷新</span>
          </button>
        </div>
      </header>

      <section className={styles.summaryBar}>
        <Metric label="项目" value={summary.total_projects} />
        <Metric label="工作树" value={summary.total_worktrees} />
        <Metric label="脏工作树" value={summary.dirty_worktrees} tone={summary.dirty_worktrees ? 'warn' : 'ok'} />
        <Metric label="未提交/未跟踪条目" value={summary.uncommitted_entries} tone={summary.dirty_worktrees ? 'warn' : undefined} />
        <Metric label="已归属" value={summary.matched_worktrees} />
        <Metric label="未知脏现场" value={summary.unknown_dirty_worktrees} tone={summary.unknown_dirty_worktrees ? 'bad' : 'ok'} />
      </section>

      <section className={styles.projectStrip}>
        <GitBranch size={16} aria-hidden="true" />
        <span>{scope === 'all' ? `全项目总览 · ${summary.audited_projects} 个已审计` : (selectedProject?.name ?? projectAudit?.project.name ?? '项目审计')}</span>
        <code>{scope === 'all' ? `${summary.skipped_projects} 个跳过，${summary.error_projects} 个异常` : (projectAudit?.workspace_path ?? selectedProject?.workspace_path ?? '-')}</code>
      </section>

      <section className={styles.filterBar}>
        <label className={styles.searchBox}>
          <Search size={15} aria-hidden="true" />
          <input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索路径、分支、会话、成员" />
        </label>
        <div className={styles.segmented}>
          <button type="button" data-active={filter === 'all'} onClick={() => setFilter('all')}>全部</button>
          <button type="button" data-active={filter === 'dirty'} onClick={() => setFilter('dirty')}>只看脏</button>
          <button type="button" data-active={filter === 'unknown'} onClick={() => setFilter('unknown')}>未知归属</button>
        </div>
      </section>

      {error && <MessageBox tone="bad" message={error} />}
      {notice && <MessageBox tone="ok" message={notice} />}

      <main className={styles.list}>
        {contextPreview && (
          <ContextPanel preview={contextPreview} onClose={() => setContextPreview(null)} />
        )}
        {loading && !projectResults.length && <div className={styles.loading}>正在读取节点 Git worktree 列表...</div>}
        {!loading && !projectResults.length && !error && <div className={styles.empty}>刷新后查看全项目 Git 工作现场</div>}
        {!loading && projectResults.length > 0 && !hasAnyRows && <div className={styles.empty}>没有匹配的 Git 工作现场</div>}
        {visibleProjects.map((project) => (
          <section className={styles.projectGroup} key={project.project.id}>
            <div className={styles.projectGroupHeader} data-status={project.status}>
              <div>
                <strong>{project.project.name}</strong>
                <span>{project.workspace_path || project.project.workspace_path || '-'}</span>
              </div>
              <em>{projectStatusText(project)}</em>
            </div>
            {project.warnings?.length ? (
              <div className={styles.warningList}>
                {project.warnings.map((warning) => (
                  <span key={warning}><AlertTriangle size={14} aria-hidden="true" />{warning}</span>
                ))}
              </div>
            ) : null}
            {project.status !== 'audited' && (
              <div className={styles.skipBox}>{project.error || '该项目本次未审计'}</div>
            )}
            {project.worktrees.map((entry) => (
              <GitWorktreeRow
                key={`${project.project.id}-${entry.path}-${entry.branch ?? ''}`}
                entry={entry}
                copied={copiedPath === entry.path}
                asking={askingKey === rowKey(project.project.id, entry)}
                onCopy={() => copyPath(entry.path)}
                onOpen={(mode) => openConversation(project.project.id, entry, mode)}
                onAsk={() => askConversation(project.project.id, entry)}
                onRead={() => readConversation(project, entry)}
              />
            ))}
            {contextLoadingKey.startsWith(`${project.project.id}:`) && (
              <div className={styles.loadingInline}>正在读取会话上下文...</div>
            )}
          </section>
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

function MessageBox({ tone, message }: { tone: 'ok' | 'bad'; message: string }) {
  return (
    <div className={tone === 'bad' ? styles.errorBox : styles.noticeBox}>
      <AlertTriangle size={16} aria-hidden="true" />
      <span>{message}</span>
    </div>
  )
}

function ContextPanel({ preview, onClose }: { preview: ContextPreview; onClose: () => void }) {
  const conversation = preview.entry.conversation
  const recent = preview.messages.slice(-12)
  return (
    <section className={styles.contextPanel}>
      <div className={styles.contextHeader}>
        <div>
          <strong>{conversation?.title || conversation?.conversation_id || '会话上下文'}</strong>
          <span>{preview.projectName} · {conversation?.user_account || conversation?.user_id || '-'}</span>
        </div>
        <button className={styles.iconBtn} type="button" onClick={onClose} title="关闭">
          <X size={15} aria-hidden="true" />
          <span>关闭</span>
        </button>
      </div>
      {conversation?.codex_thread_id && (
        <code className={styles.threadLine}>codex://threads/{conversation.codex_thread_id}</code>
      )}
      <div className={styles.contextMessages}>
        {recent.map((message) => (
          <div key={message.id} className={styles.contextMessage} data-role={message.role}>
            <span>{message.sender_name || message.role || 'message'}</span>
            <p>{displayMessageContentOrAttachment(message.content)}</p>
          </div>
        ))}
        {!recent.length && <p className={styles.contextEmpty}>这个会话没有可展示的消息。</p>}
      </div>
    </section>
  )
}

function singleProjectResult(audit: ProjectGitWorktreeAuditResponse): GlobalGitWorktreeAuditProjectResult {
  return {
    project: audit.project,
    status: 'audited',
    error: null,
    workspace_path: audit.workspace_path,
    git_root: audit.git_root,
    warnings: audit.warnings ?? [],
    summary: audit.summary,
    worktrees: audit.worktrees,
  }
}

function summaryFromProject(audit: ProjectGitWorktreeAuditResponse): GlobalGitWorktreeAuditSummary {
  return {
    ...emptyGlobalSummary(),
    total_projects: 1,
    audited_projects: 1,
    total_worktrees: audit.summary.total_worktrees,
    dirty_worktrees: audit.summary.dirty_worktrees,
    uncommitted_entries: audit.summary.uncommitted_entries,
    untracked_entries: audit.summary.untracked_entries,
    matched_worktrees: audit.summary.matched_worktrees,
    unknown_dirty_worktrees: audit.summary.unknown_dirty_worktrees,
  }
}

function emptyGlobalSummary(): GlobalGitWorktreeAuditSummary {
  return {
    total_projects: 0,
    audited_projects: 0,
    skipped_projects: 0,
    error_projects: 0,
    total_worktrees: 0,
    dirty_worktrees: 0,
    uncommitted_entries: 0,
    untracked_entries: 0,
    matched_worktrees: 0,
    unknown_dirty_worktrees: 0,
  }
}

function entryMatches(entry: ProjectGitWorktreeAuditEntry, query: string, filter: FilterMode) {
  if (filter === 'dirty' && !entry.has_uncommitted_changes) return false
  if (filter === 'unknown' && (!entry.has_uncommitted_changes || entry.conversation)) return false
  const text = query.trim().toLowerCase()
  if (!text) return true
  const conversation = entry.conversation
  return [
    entry.path,
    entry.branch,
    entry.head,
    conversation?.title,
    conversation?.conversation_id,
    conversation?.user_account,
    conversation?.user_id,
  ].some((value) => value?.toLowerCase().includes(text))
}

function shouldShowProject(project: GlobalGitWorktreeAuditProjectResult, query: string, filter: FilterMode) {
  if (project.worktrees.length > 0) return true
  if (filter !== 'all') return false
  const text = query.trim().toLowerCase()
  if (!text) return project.status !== 'audited'
  return [project.project.name, project.workspace_path, project.error]
    .some((value) => value?.toLowerCase().includes(text))
}

function rowKey(projectId: string, entry: ProjectGitWorktreeAuditEntry) {
  return `${projectId}:${entry.path}:${entry.branch ?? ''}`
}

function projectStatusText(project: GlobalGitWorktreeAuditProjectResult) {
  if (project.status === 'audited') {
    return `${project.summary.dirty_worktrees}/${project.summary.total_worktrees} 脏工作树`
  }
  if (project.status === 'skipped') return '已跳过'
  return '审计异常'
}
