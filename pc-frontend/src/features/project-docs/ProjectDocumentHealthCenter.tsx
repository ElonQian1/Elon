import { Activity, AlertTriangle, CheckSquare2, Filter, GitCommitHorizontal, Network, RefreshCw, Sparkles, X } from 'lucide-react'
import { useMemo, useState, type FormEvent } from 'react'

import { nodeApi } from '../node/localNodeApi'
import type { DocumentOrganizationTrackingRuntime } from './projectDocumentOrganizationStatus'
import type { DocumentHealthAnalysis, DocumentHealthIssue } from './projectDocumentModel'
import ProjectDocumentVersionHistory from './ProjectDocumentVersionHistory'
import styles from './ProjectDocumentHealthCenter.module.css'

interface Props {
  analysis?: DocumentHealthAnalysis
  runtime: DocumentOrganizationTrackingRuntime
  onRefresh: () => void
  onOpenSuggestions: () => void
  onRunAi: (instruction: string) => void
}

interface IssueFilters { severity: string; status: string; owner: string; topic: string; scope: string }
const emptyFilters: IssueFilters = { severity: '', status: '', owner: '', topic: '', scope: '' }

export default function ProjectDocumentHealthCenter({ analysis, runtime, onRefresh, onOpenSuggestions, onRunAi }: Props) {
  const [filters, setFilters] = useState(emptyFilters)
  const [selected, setSelected] = useState<Set<string>>(() => new Set())
  const [editing, setEditing] = useState<DocumentHealthIssue | null>(null)
  const workflow = analysis?.governance_workflow
  const issues = workflow?.issues ?? analysis?.quality.issues ?? []
  const visible = useMemo(() => issues.filter((issue) => {
    const state = issue.workflow?.status ?? 'open'
    return (!filters.severity || issue.severity === filters.severity)
      && (!filters.status || state === filters.status)
      && (!filters.owner || issue.workflow?.owner === filters.owner)
      && (!filters.topic || issue.context?.primary_topic === filters.topic)
      && (!filters.scope || issue.context?.scope_id === filters.scope)
  }), [filters, issues])
  if (!analysis) return <main className={styles.center}><section className={styles.empty}>
    <Activity size={28} /><h1>文档健康分析尚不可用</h1><p>刷新目录后，服务端会用零模型 token 建立增量索引并运行确定性检查。</p>
    <button type="button" onClick={onRefresh}><RefreshCw size={14} />刷新分析</button></section></main>
  const quality = analysis.quality.summary
  const actionable = workflow?.summary.actionable ?? quality.total_issues
  const selectedIssues = issues.filter((issue) => selected.has(issue.fingerprint))

  function runSelectedAi() {
    if (!selectedIssues.length) { onOpenSuggestions(); return }
    const evidence = selectedIssues.slice(0, 30).map((issue) => `${issue.fingerprint}:${issue.type}:${issue.path}`).join(', ')
    onRunAi(`只处理用户在文档健康中心选中的 ${selectedIssues.length} 个问题。先用 project_docs_get_issues 按 fingerprint 核对证据，再只为这些问题生成建议，不扩展到未选问题。选中证据：${evidence}`)
  }
  return <main className={styles.center}>
    <header className={styles.hero}><span><Activity size={22} /></span><div><small>服务端统一真源 · 零模型 token 预检</small>
      <h1>项目文档健康中心</h1><p>发现问题只是开始：在这里分派、设期限、延期、说明忽略原因，并用 Git 安全恢复。</p></div>
      <strong data-status={analysis.overall.status}>{analysis.overall.score}<small>/ 100</small></strong></header>
    <section className={styles.metrics}>
      <article><Activity /><span>结构健康<strong>{analysis.architecture.score}</strong></span></article>
      <article><AlertTriangle /><span>可执行问题<strong>{actionable}</strong></span></article>
      <article><GitCommitHorizontal /><span>本次变更<strong>{analysis.maintenance.changed_documents}</strong></span></article>
      <article><Network /><span>知识节点<strong>{analysis.federation.node_count}</strong></span></article>
    </section>
    <section className={styles.workflowSummary}>
      {(['open', 'assigned', 'snoozed', 'ignored', 'resolved'] as const).map((status) => <button type="button" key={status}
        data-active={filters.status === status || undefined} onClick={() => setFilters({ ...filters, status: filters.status === status ? '' : status })}>
        <span>{statusLabel(status)}</span><strong>{workflow?.summary[status] ?? (status === 'open' ? quality.total_issues : 0)}</strong></button>)}
      {!!workflow?.summary.overdue && <em>{workflow.summary.overdue} 个已逾期</em>}
    </section>
    <div className={styles.grid}>
      <section className={styles.panel}>
        <header><div><strong>需要处理的问题</strong><small>{quality.errors} 错误 · {quality.warnings} 警告 · {quality.info} 提示</small></div>
          <button type="button" onClick={onRefresh}><RefreshCw size={13} />刷新</button></header>
        <div className={styles.issueFilters}><Filter size={13} />
          <FilterSelect label="严重度" value={filters.severity} values={workflow?.filters.severities ?? ['error', 'warning', 'info']} onChange={(severity) => setFilters({ ...filters, severity })} />
          <FilterSelect label="负责人" value={filters.owner} values={workflow?.filters.owners ?? []} onChange={(owner) => setFilters({ ...filters, owner })} />
          <FilterSelect label="主题" value={filters.topic} values={workflow?.filters.topics ?? []} onChange={(topic) => setFilters({ ...filters, topic })} />
          <FilterSelect label="节点" value={filters.scope} values={workflow?.filters.scopes ?? []} onChange={(scope) => setFilters({ ...filters, scope })} />
          <button type="button" disabled={Object.values(filters).every((value) => !value)} onClick={() => setFilters(emptyFilters)}><X size={12} />清除</button>
        </div>
        <div className={styles.issueList}>
          {visible.length ? visible.map((issue) => <article key={issue.fingerprint} data-severity={issue.severity} data-status={issue.workflow?.status ?? 'open'}>
            <label><input type="checkbox" checked={selected.has(issue.fingerprint)} onChange={(event) => setSelected((current) => {
              const next = new Set(current); if (event.target.checked) next.add(issue.fingerprint); else next.delete(issue.fingerprint); return next
            })} /><i /></label>
            <div><strong>{issue.message}</strong><small>{issue.path}</small><p>{issue.evidence}</p>
              <footer><span>{statusLabel(issue.workflow?.status ?? 'open')}</span>{issue.workflow?.owner && <span>负责人：{issue.workflow.owner}</span>}
                {issue.workflow?.due_at && <span>期限：{issue.workflow.due_at}</span>}{issue.context?.scope_id && <span>节点：{issue.context.scope_id}</span>}</footer></div>
            <em>{issue.confidence}%</em>
            <button type="button" disabled={!runtime.enabled} onClick={() => setEditing(issue)}>处理</button>
          </article>) : <p className={styles.muted}>当前筛选条件下没有质量问题。</p>}
        </div>
        <div className={styles.issueActions}><button type="button" onClick={() => setSelected(new Set(visible.map((issue) => issue.fingerprint)))}><CheckSquare2 size={13} />选择当前结果</button>
          <button className={styles.aiButton} type="button" onClick={runSelectedAi}><Sparkles size={14} />{selected.size ? `让 AI 处理选中的 ${selected.size} 项` : '让 AI 根据证据提出整理建议'}</button></div>
      </section>
      <aside className={styles.side}>
        <ScoreExplanation analysis={analysis} />
        <HealthTrend analysis={analysis} />
        <section className={styles.panel}><header><div><strong>持续维护</strong><small>索引 v{analysis.maintenance.index_version}</small></div></header><dl>
          <div><dt>持久事件队列</dt><dd>{analysis.maintenance.durable_queue ? '已启用' : '未启用'}</dd></div>
          <div><dt>待处理事件</dt><dd>{analysis.maintenance.pending_events}</dd></div><div><dt>本轮已处理</dt><dd>{analysis.maintenance.processed_events}</dd></div>
          <div><dt>后台复查</dt><dd>{analysis.maintenance.poll_interval_seconds} 秒</dd></div><div><dt>外链待检查</dt><dd>{quality.external_links_pending}</dd></div></dl></section>
        <section className={styles.panel}><header><div><strong>联邦知识架构</strong><small>{analysis.federation.source === 'manifest' ? '显式清单' : '程序推断'} · 最深 {analysis.federation.max_depth} 层</small></div></header>
          <div className={styles.nodes}>{analysis.federation.nodes.map((node) => <article key={node.id} style={{ marginLeft: Math.min(36, node.parent_id ? 12 : 0) }}>
            <span><strong>{node.label}</strong><small>{node.scope_path || '项目根'}</small></span><em>{node.document_count} · {node.score}</em></article>)}</div></section>
        <ProjectDocumentVersionHistory runtime={runtime} onRestored={onRefresh} />
      </aside>
    </div>
    {editing && <IssueEditor issue={editing} runtime={runtime} onClose={() => setEditing(null)} onSaved={() => { setEditing(null); onRefresh() }} />}
  </main>
}

function ScoreExplanation({ analysis }: { analysis: DocumentHealthAnalysis }) {
  const explanation = analysis.governance_workflow?.score_explanation
  if (!explanation) return null
  return <section className={styles.panel}><header><div><strong>健康分解释</strong><small>{explanation.formula}</small></div></header>
    <div className={styles.scoreParts}>{explanation.components.map((part) => <article key={part.key}><span>{part.label}<small>权重 {part.weight}%</small></span><strong>{part.score}</strong><em>+{part.contribution}</em></article>)}</div></section>
}

function HealthTrend({ analysis }: { analysis: DocumentHealthAnalysis }) {
  const trend = analysis.governance_workflow?.trend ?? []
  if (!trend.length) return null
  return <section className={styles.panel}><header><div><strong>健康趋势</strong><small>最近 {trend.length} 次有效变化</small></div></header>
    <div className={styles.trend}>{trend.map((point) => <i key={`${point.created_at_ms}-${point.overall_score}`} style={{ height: `${Math.max(8, point.overall_score)}%` }} title={`${new Date(point.created_at_ms).toLocaleString()}：${point.overall_score} 分 / ${point.actionable_count} 项`} />)}</div></section>
}

function FilterSelect({ label, value, values, onChange }: { label: string; value: string; values: string[]; onChange: (value: string) => void }) {
  return <label><span>{label}</span><select value={value} onChange={(event) => onChange(event.target.value)}><option value="">全部</option>{values.map((item) => <option key={item}>{item}</option>)}</select></label>
}

function IssueEditor({ issue, runtime, onClose, onSaved }: { issue: DocumentHealthIssue; runtime: DocumentOrganizationTrackingRuntime; onClose: () => void; onSaved: () => void }) {
  const [status, setStatus] = useState(issue.workflow?.status ?? 'open')
  const [owner, setOwner] = useState(issue.workflow?.owner ?? '')
  const [dueAt, setDueAt] = useState(issue.workflow?.due_at ?? '')
  const [reason, setReason] = useState(issue.workflow?.reason ?? '')
  const [snoozedUntil, setSnoozedUntil] = useState(issue.workflow?.snoozed_until ?? '')
  const [busy, setBusy] = useState(false); const [error, setError] = useState('')
  async function submit(event: FormEvent) {
    event.preventDefault(); setBusy(true); setError('')
    try {
      const response = await nodeApi<{ ok: boolean; error?: string }>(runtime.adminUrl, '/api/project-docs/governance/issues/update', { method: 'POST', body: JSON.stringify({
        project_root: runtime.projectRoot, fingerprint: issue.fingerprint, status, owner, due_at: dueAt, reason, snoozed_until: snoozedUntil,
      }) })
      if (!response.ok) throw new Error(response.error || '保存问题处理状态失败')
      onSaved()
    } catch (cause) { setError(cause instanceof Error ? cause.message : '保存问题处理状态失败'); setBusy(false) }
  }
  return <div className={styles.diffBackdrop} role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose() }}><form className={styles.issueDialog} onSubmit={submit}>
    <header><div><strong>处理文档问题</strong><small>{issue.path}</small></div><button type="button" onClick={onClose}><X size={14} /></button></header>
    <p>{issue.message}</p><label>状态<select value={status} onChange={(event) => setStatus(event.target.value as typeof status)}>{['open', 'assigned', 'snoozed', 'ignored', 'resolved'].map((item) => <option key={item} value={item}>{statusLabel(item)}</option>)}</select></label>
    <label>负责人<input value={owner} onChange={(event) => setOwner(event.target.value)} placeholder="分派状态必填" /></label>
    <label>处理期限<input type="date" value={dueAt} onChange={(event) => setDueAt(event.target.value)} /></label>
    {status === 'snoozed' && <label>恢复日期<input required type="date" value={snoozedUntil} onChange={(event) => setSnoozedUntil(event.target.value)} /></label>}
    {(status === 'ignored' || status === 'snoozed') && <label>原因<textarea required value={reason} onChange={(event) => setReason(event.target.value)} placeholder="为什么忽略或延期？" /></label>}
    {error && <div className={styles.errorText}>{error}</div>}<footer><button type="button" onClick={onClose}>取消</button><button type="submit" disabled={busy}>{busy ? '保存中…' : '保存状态'}</button></footer>
  </form></div>
}

function statusLabel(status: string) { return ({ open: '待处理', assigned: '已分派', snoozed: '已延期', ignored: '已忽略', resolved: '已解决' } as Record<string, string>)[status] ?? status }
