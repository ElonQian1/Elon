import { useCallback, useEffect, useState } from 'react'
import { Check, ChevronLeft, ChevronRight, FileSearch, Pencil, RefreshCw, RotateCcw, ShieldCheck, X } from 'lucide-react'

import type { DocumentAutomationMode } from './projectDocumentSections'
import type { DocumentOrganizationTrackingRuntime } from './projectDocumentOrganizationStatus'
import {
  listNativeContextCandidates,
  loadNativeContextMemoryHealth,
  reviseNativeContextCandidate,
  reviewNativeContextCandidates,
  type NativeContextCandidate,
  type NativeContextCandidatePage,
  type NativeContextCandidateStatus,
  type NativeContextMemoryHealth,
  type NativeContextReviewAction,
} from './projectDocumentNativeContextModel'
import ProjectDocumentNativeContextEditor from './ProjectDocumentNativeContextEditor'
import styles from './ProjectDocumentNativeContextInbox.module.css'

interface Props {
  runtime: DocumentOrganizationTrackingRuntime
  canEdit: boolean
  catalogRevision?: string
  suggestionsRevision?: string
  authorizationMode: DocumentAutomationMode
  onSuggestionsChanged: () => void | Promise<void>
}

const FILTERS: Array<{ value: NativeContextCandidateStatus; label: string }> = [
  { value: 'pending', label: '待审核' },
  { value: 'reviewed', label: '已并入建议' },
  { value: 'rejected', label: '已拒绝' },
  { value: 'applied', label: '已共享' },
]

export default function ProjectDocumentNativeContextInbox({
  runtime,
  canEdit,
  catalogRevision,
  suggestionsRevision,
  authorizationMode,
  onSuggestionsChanged,
}: Props) {
  const [status, setStatus] = useState<NativeContextCandidateStatus>('pending')
  const [offset, setOffset] = useState(0)
  const [page, setPage] = useState<NativeContextCandidatePage | null>(null)
  const [health, setHealth] = useState<NativeContextMemoryHealth | null>(null)
  const [selected, setSelected] = useState<Set<string>>(() => new Set())
  const [loading, setLoading] = useState(false)
  const [action, setAction] = useState<NativeContextReviewAction | ''>('')
  const [editingId, setEditingId] = useState('')
  const [revising, setRevising] = useState(false)
  const [error, setError] = useState('')

  const load = useCallback(async () => {
    if (!runtime.enabled || !runtime.adminUrl || !runtime.projectRoot) return
    setLoading(true)
    setError('')
    try {
      const result = await listNativeContextCandidates({
        adminUrl: runtime.adminUrl,
        projectRoot: runtime.projectRoot,
        status,
        offset,
      })
      setPage(result)
      setSelected(new Set())
    } catch (reason) {
      setError(errorMessage(reason, '读取原生理解候选失败'))
    } finally {
      setLoading(false)
    }
  }, [offset, runtime.adminUrl, runtime.enabled, runtime.projectRoot, status])

  const loadHealth = useCallback(async () => {
    if (!runtime.enabled || !runtime.adminUrl || !runtime.projectRoot) return
    const memoryHealth = await loadNativeContextMemoryHealth({
      adminUrl: runtime.adminUrl,
      projectRoot: runtime.projectRoot,
    }).catch(() => null)
    setHealth(memoryHealth)
  }, [runtime.adminUrl, runtime.enabled, runtime.projectRoot])

  useEffect(() => { void load() }, [load])
  useEffect(() => { void loadHealth() }, [loadHealth])

  const chooseStatus = (nextStatus: NativeContextCandidateStatus) => {
    setStatus(nextStatus)
    setOffset(0)
    setSelected(new Set())
    setEditingId('')
  }

  const toggle = (candidateId: string) => {
    setSelected((current) => {
      const next = new Set(current)
      if (next.has(candidateId)) next.delete(candidateId)
      else next.add(candidateId)
      return next
    })
  }

  const runAction = async (nextAction: NativeContextReviewAction) => {
    if (!selected.size || action) return
    if (nextAction === 'accept' && !catalogRevision) {
      setError('目录 revision 尚未加载，刷新项目文档后再接受候选。')
      return
    }
    if (nextAction === 'accept' && (page?.candidates ?? []).some(
      (candidate) => selected.has(candidate.candidate_id) && !candidate.evidence_current,
    )) {
      setError('已选候选中存在证据漂移项；可以拒绝清理，但不能接受。')
      return
    }
    setAction(nextAction)
    setError('')
    try {
      await reviewNativeContextCandidates({
        adminUrl: runtime.adminUrl,
        projectRoot: runtime.projectRoot,
        candidateIds: [...selected],
        action: nextAction,
        authorizationMode,
        catalogRevision,
        suggestionsRevision,
      })
      if (nextAction === 'accept') await onSuggestionsChanged()
      await load()
    } catch (reason) {
      setError(errorMessage(reason, '审核原生理解候选失败'))
    } finally {
      setAction('')
    }
  }

  const runRevision = async (candidate: NativeContextCandidate, summary: string, topics: string[]) => {
    if (revising) return
    setRevising(true)
    setError('')
    try {
      await reviseNativeContextCandidate({
        adminUrl: runtime.adminUrl,
        projectRoot: runtime.projectRoot,
        candidateId: candidate.candidate_id,
        expectedUpdatedAtMs: candidate.updated_at_ms,
        summary,
        topics,
      })
      setEditingId('')
      await load()
    } catch (reason) {
      setError(errorMessage(reason, '修订原生理解候选失败'))
    } finally {
      setRevising(false)
    }
  }

  if (!runtime.enabled || !runtime.adminUrl || !runtime.projectRoot) {
    return (
      <section className={styles.inbox}>
        <header><div><FileSearch size={18} /><strong>原生理解候选</strong></div><span>本机候选</span></header>
        <p className={styles.unavailable}>当前项目没有连接本机节点；候选不会随网页请求上传，也不会自动成为项目真源。</p>
      </section>
    )
  }

  const currentCandidates = page?.candidates ?? []
  const selectableCandidates = status === 'pending'
    ? currentCandidates
    : currentCandidates.filter((candidate) => candidate.evidence_current)
  const selectedEvidenceCurrent = currentCandidates
    .filter((candidate) => selected.has(candidate.candidate_id))
    .every((candidate) => candidate.evidence_current)
  const allCurrentSelected = selectableCandidates.length > 0
    && selectableCandidates.every((candidate) => selected.has(candidate.candidate_id))
  return (
    <section className={styles.inbox} data-status={status}>
      <header className={styles.header}>
        <div><FileSearch size={18} aria-hidden="true" /><strong>原生理解候选</strong><span>本机 SQLite · 不含源码正文</span></div>
        <button type="button" onClick={() => { void Promise.all([load(), loadHealth()]) }} disabled={loading || !!action} title="刷新候选与共享记忆健康状态">
          <RefreshCw size={15} className={loading ? styles.spinning : ''} aria-hidden="true" />
        </button>
      </header>
      {health && (
        <p className={styles.intro}>共享记忆健康：{health.current_count}/{health.checked_count} 有效
          {health.drifted_count > 0 ? ` · ${health.drifted_count} 条漂移` : ''}
          {health.relocation_suggested_count > 0 ? ` · ${health.relocation_suggested_count} 条有重定位建议` : ''}
          {health.truncated ? ' · 仅检查前 64 条' : ''}。这是诊断结果，不会自动改写证据。</p>
      )}
      {health && (
        <p className={styles.intro}>任务后回执 Hook：{health.receipt_automation.node_policy_enabled ? '节点策略开启' : '节点策略关闭'}；
          {health.receipt_automation.trust_mode === 'codex_non_managed_hook_review'
            ? 'Codex 首次或定义变化后仍需在 /hooks 审核信任'
            : '当前节点未报告信任模式'}；不会绕过 Hook trust。此状态不证明 Hook 已真实执行。</p>
      )}
      <p className={styles.intro}>Codex Desktop/CLI 核对过的短结论先进入这里。接受只会并入现有建议；应用后才进入 Git 共享记忆，且源码优先、hash 漂移立即失效。</p>
      <nav className={styles.filters} aria-label="候选状态">
        {FILTERS.map((filter) => (
          <button type="button" key={filter.value} data-active={status === filter.value} onClick={() => chooseStatus(filter.value)}>
            {filter.label}<em>{page?.counts[filter.value] ?? 0}</em>
          </button>
        ))}
      </nav>
      {error && <div className={styles.error}>{error}</div>}
      {!loading && !currentCandidates.length && <div className={styles.empty}>当前状态没有候选。</div>}
      {!!currentCandidates.length && (
        <div className={styles.list}>
          {(status === 'pending' || status === 'rejected') && (
            <label className={styles.selectAll}>
              <input type="checkbox" checked={allCurrentSelected} onChange={() => {
                setSelected(allCurrentSelected
                  ? new Set()
                  : new Set(selectableCandidates.map((candidate) => candidate.candidate_id)))
              }} />
              {status === 'pending' ? '选择本页候选（漂移项只能拒绝）' : '选择本页证据仍有效的候选'}
            </label>
          )}
          {currentCandidates.map((candidate) => (
            <article key={candidate.candidate_id} className={styles.candidate} data-current={candidate.evidence_current}>
              <label className={styles.candidateTitle}>
                {(status === 'pending' || status === 'rejected') && (
                  <input type="checkbox" checked={selected.has(candidate.candidate_id)}
                    disabled={status === 'rejected' && !candidate.evidence_current}
                    onChange={() => toggle(candidate.candidate_id)} />
                )}
                <span><strong>{candidate.summary}</strong><code>{candidate.candidate_id}</code></span>
                <i>{candidate.evidence_current ? <><ShieldCheck size={14} />证据有效</> : <>证据已漂移</>}</i>
              </label>
              <div className={styles.topics}>{candidate.topics.map((topic) => <span key={topic}>{topic}</span>)}</div>
              {!!candidate.conflicts.length && (
                <div className={styles.error}>需要人工处理：{candidate.conflicts.map((conflict) => (
                  conflict.kind === 'shared_replacement'
                    ? `替换共享记忆 ${conflict.shared_candidate_id}`
                    : `与共享记忆 ${conflict.shared_candidate_id} 可能冲突`
                )).join('；')}</div>
              )}
              <ul className={styles.evidence}>
                {candidate.evidence.map((evidence) => (
                  <li key={`${evidence.path}:${evidence.locator}`}>
                    <code>{evidence.path}</code>
                    <span>{evidence.locator || evidence.evidence_kind}</span>
                    <small>{evidence.git_identity?.head_blob_oid
                      ? `Git ${evidence.git_identity.head_blob_oid.slice(0, 10)}… · SHA ${evidence.content_hash.slice(0, 8)}…`
                      : `${evidence.content_hash.slice(0, 12)}…`}</small>
                  </li>
                ))}
              </ul>
              {editingId === candidate.candidate_id && (
                <ProjectDocumentNativeContextEditor candidate={candidate} disabled={revising}
                  onCancel={() => setEditingId('')}
                  onSave={(summary, topics) => runRevision(candidate, summary, topics)} />
              )}
              <footer>
                <span>来源 {candidate.producer || 'native tools'} · {candidate.provenance.assurance === 'local_mcp_session_attested'
                  ? '本机会话已认证'
                  : '来源自述'}{candidate.provenance.last_editor ? ' · 已人工修订' : ''}</span>
                <div><time>{formatTime(candidate.updated_at_ms)}</time>
                  {(status === 'pending' || status === 'rejected') && canEdit && (
                    <button type="button" className={styles.editButton} disabled={!!action || revising}
                      onClick={() => setEditingId(editingId === candidate.candidate_id ? '' : candidate.candidate_id)}>
                      <Pencil size={13} />{editingId === candidate.candidate_id ? '收起编辑' : '修订'}
                    </button>
                  )}
                </div>
              </footer>
            </article>
          ))}
        </div>
      )}
      <footer className={styles.actions}>
        <div>
          <button type="button" disabled={!offset || loading || !!action} onClick={() => setOffset(Math.max(0, offset - (page?.pagination.limit ?? 10)))}>
            <ChevronLeft size={15} />上一页
          </button>
          <span>{(page?.pagination.total ?? 0) ? offset + 1 : 0}–{offset + (page?.pagination.returned ?? 0)} / {page?.pagination.total ?? 0}</span>
          <button type="button" disabled={page?.pagination.next_offset === undefined || loading || !!action}
            onClick={() => setOffset(page?.pagination.next_offset ?? offset)}>
            下一页<ChevronRight size={15} />
          </button>
        </div>
        <div>
          <span>已选 {selected.size}</span>
          {status === 'pending' && <>
            <button type="button" className={styles.reject} disabled={!selected.size || !canEdit || !!action} onClick={() => { void runAction('reject') }}>
              <X size={15} />{action === 'reject' ? '拒绝中…' : '拒绝'}
            </button>
            <button type="button" className={styles.accept}
              disabled={!selected.size || !selectedEvidenceCurrent || !canEdit || !catalogRevision || !!action}
              onClick={() => { void runAction('accept') }}>
              <Check size={15} />{action === 'accept' ? '并入中…' : '接受并入建议'}
            </button>
          </>}
          {status === 'rejected' && (
            <button type="button" disabled={!selected.size || !canEdit || !!action} onClick={() => { void runAction('restore') }}>
              <RotateCcw size={15} />{action === 'restore' ? '恢复中…' : '恢复待审核'}
            </button>
          )}
        </div>
      </footer>
    </section>
  )
}

function formatTime(value: number) {
  return value ? new Date(value).toLocaleString([], { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' }) : ''
}

function errorMessage(error: unknown, fallback: string) {
  return (error as { message?: string })?.message ?? fallback
}
