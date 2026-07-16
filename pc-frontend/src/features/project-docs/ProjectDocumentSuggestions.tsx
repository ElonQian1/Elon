import { CheckCircle2, FileWarning, Lightbulb, RefreshCw, Sparkles } from 'lucide-react'

import { formatNumber } from './projectDocumentModel'
import ProjectDocumentFileOperations from './ProjectDocumentFileOperations'
import { DOCUMENT_AUTOMATION_OPTIONS } from './projectDocumentAutomationPolicy'
import type { DocumentAutomationMode, DocumentOrganizationSuggestions } from './projectDocumentSections'
import type { DocumentOrganizationTrace } from './projectDocumentOrganizationStatus'
import styles from './ProjectDocumentsWorkspace.module.css'

interface Props {
  suggestions: DocumentOrganizationSuggestions | null
  trace: DocumentOrganizationTrace | null
  trackingAvailable: boolean
  trackingError: string
  loading: boolean
  error: string
  canEdit: boolean
  applying: boolean
  applyingFiles: boolean
  canApplyFiles: boolean
  automationMode: DocumentAutomationMode
  onAutomationModeChange: (mode: DocumentAutomationMode) => void
  onRefresh: () => void
  onApply: () => void
  onApplyFiles: (input: { operationIds: string[]; allowRename: boolean; allowMove: boolean }) => Promise<void>
}

export default function ProjectDocumentSuggestions({
  suggestions,
  trace,
  trackingAvailable,
  trackingError,
  loading,
  error,
  canEdit,
  applying,
  applyingFiles,
  canApplyFiles,
  automationMode,
  onAutomationModeChange,
  onRefresh,
  onApply,
  onApplyFiles,
}: Props) {
  const ready = suggestions?.status === 'ready'
  const suggestionCount = (suggestions?.proposed_sections.length ?? 0)
    + (suggestions?.assignments.length ?? 0)
    + Object.keys(suggestions?.document_metadata ?? {}).length
    + (suggestions?.proposed_home ? 1 : 0)
  return (
    <main className={styles.suggestionsPane}>
      <header className={styles.suggestionsHeader}>
        <div>
          <span><Sparkles size={18} aria-hidden="true" /></span>
          <div>
            <strong>AI 整理建议</strong>
            <small>{automationMode === 'git_backed_full' ? '整理前 Git 备份，AI 完全整理，整理后自动提交' : '建议与实际目录分开，按当前权限处理'}</small>
          </div>
        </div>
        <label className={styles.automationMode}>
          <span>AI 权限</span>
          <select value={automationMode} onChange={(event) => onAutomationModeChange(event.target.value as DocumentAutomationMode)}>
            {DOCUMENT_AUTOMATION_OPTIONS.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
          </select>
        </label>
        <button type="button" onClick={onRefresh} disabled={loading}>
          <RefreshCw size={15} className={loading ? styles.spinning : ''} aria-hidden="true" />
          刷新建议
        </button>
      </header>

      <div className={styles.suggestionsBody}>
        {error && <div className={styles.errorBox}>{error}</div>}
        {trackingError && <div className={styles.errorBox}>{trackingError}</div>}
        {trace && (
          <section className={styles.organizationTrace} data-status={trace.status}>
            <header>
              <div>
                <strong>整理运行观测</strong>
                <span>{trace.events[trace.events.length - 1]?.label ?? trace.current_stage}</span>
              </div>
              <code>{trace.operation_id}</code>
            </header>
            <div className={styles.traceMetrics}>
              <span>目录 <strong>{formatNumber(trace.documents_cataloged)}</strong></span>
              <span>歧义 <strong>{formatNumber(trace.ambiguous_documents)}</strong></span>
              <span>正文读取 <strong>{formatNumber(trace.documents_read)}</strong></span>
              <span>估算 token <strong>{formatNumber(trace.estimated_tokens_used)}</strong></span>
            </div>
            <ol className={styles.traceTimeline}>
              {trace.events.map((event, index) => (
                <li key={`${event.stage}:${event.at}:${index}`} data-status={event.status}>
                  <i>{event.status === 'failed' ? '!' : index + 1}</i>
                  <div><strong>{event.label}</strong><span>{event.detail}</span></div>
                  <time>{formatTraceTime(event.at)}</time>
                </li>
              ))}
            </ol>
            {trace.error && (
              <div className={styles.traceFailure}>
                <strong>{trace.error.code}</strong>
                <span>{trace.error.message}</span>
                <p>修复建议：{trace.error.recovery}</p>
              </div>
            )}
            {(trace.catalog_revision || trace.suggestions_revision || trace.manifest_revision || trace.git_baseline_commit || trace.git_result_commit) && (
              <footer className={styles.traceRevisions}>
                {trace.catalog_revision && <span>catalog <code>{shortRevision(trace.catalog_revision)}</code></span>}
                {trace.suggestions_revision && <span>suggestions <code>{shortRevision(trace.suggestions_revision)}</code></span>}
                {trace.manifest_revision && <span>manifest <code>{shortRevision(trace.manifest_revision)}</code></span>}
                {trace.git_baseline_commit && <span>整理前 Git <code>{shortRevision(trace.git_baseline_commit)}</code></span>}
                {trace.git_result_commit && <span>整理后 Git <code>{shortRevision(trace.git_result_commit)}</code></span>}
              </footer>
            )}
          </section>
        )}
        {!trackingAvailable && !trace && (
          <div className={styles.trackingUnavailable}>当前运行路线不经过本机节点；建议文件仍可审核，但不会显示本机 MCP 分阶段日志。</div>
        )}
        {!suggestions && !loading && (
          <div className={styles.suggestionEmpty}>
            <Lightbulb size={34} aria-hidden="true" />
            <strong>还没有 AI 整理建议</strong>
            <span>点击“让当前 AI 生成整理建议”。默认会先备份文档，再自动整理并提交结果。</span>
          </div>
        )}
        {suggestions?.status === 'requested' && (
          <div className={styles.suggestionPending}>
            <Sparkles size={22} aria-hidden="true" />
            <div><strong>AI 整理任务已发起</strong><span>{suggestions.summary}</span></div>
          </div>
        )}
        {suggestions && suggestions.status !== 'requested' && (
          <>
            <section className={styles.suggestionSummary}>
              <CheckCircle2 size={20} aria-hidden="true" />
              <div>
                <strong>{suggestions.status === 'applied' ? '建议已应用' : '建议已生成'}</strong>
                <p>{suggestions.summary || 'AI 未提供摘要。'}</p>
                <small>
                  实际读取 {formatNumber(suggestions.documents_read)} 份文档，
                  估算消耗 {formatNumber(suggestions.estimated_tokens_used)} token
                </small>
              </div>
            </section>

            <section>
              <h3>项目知识架构</h3>
              <div className={styles.architectureProposal}>
                <article><span>项目类型</span><strong>{profileLabel(suggestions.proposed_profile)}</strong></article>
                <article><span>知识首页</span><strong>{suggestions.proposed_home?.title || '沿用当前设置'}</strong></article>
                <article><span>文档关系</span><strong>{Object.keys(suggestions.document_metadata).length} 份</strong></article>
                <article><span>基础文档缺口</span><strong>{suggestions.missing_document_types.length} 类</strong></article>
              </div>
              {!!suggestions.architecture_findings.length && <ul>{suggestions.architecture_findings.map((finding) => <li key={finding}>{finding}</li>)}</ul>}
            </section>

            <section>
              <h3>建议主题知识树 <em>{suggestions.proposed_sections.length}</em></h3>
              <div className={styles.proposedSections}>
                {suggestions.proposed_sections.map((section) => (
                  <article key={section.id}>
                    <i style={{ background: section.color }} />
                    <strong>{section.label}</strong>
                    <span>{section.detail}</span>
                    <code>{section.parent_id ? `${section.parent_id} / ` : ''}custom:{section.id}</code>
                  </article>
                ))}
                {!suggestions.proposed_sections.length && <p>不需要新增主题。</p>}
              </div>
            </section>

            <section>
              <h3>建议归类 <em>{suggestions.assignments.length}</em></h3>
              <div className={styles.assignmentList}>
                {suggestions.assignments.map((assignment) => (
                  <article key={`${assignment.path}:${assignment.section_id}`}>
                    <code>{assignment.path}</code>
                    <strong>→ {assignment.section_id}</strong>
                    <span>{assignment.reason}</span>
                  </article>
                ))}
                {!suggestions.assignments.length && <p>没有文档需要调整主题归类。</p>}
              </div>
            </section>

            {!!suggestions.conflicts.length && (
              <section>
                <h3><FileWarning size={16} aria-hidden="true" /> 权威冲突</h3>
                <ul>{suggestions.conflicts.map((conflict) => <li key={conflict}>{conflict}</li>)}</ul>
              </section>
            )}

            {!!suggestions.move_suggestions.length && (
              <section>
                <h3>建议的实体目录调整</h3>
                <ul>{suggestions.move_suggestions.map((move) => <li key={move}>{move}</li>)}</ul>
                <p className={styles.safetyNote}>这些移动不会自动执行，必须另行审核 Git 变更。</p>
              </section>
            )}

            <ProjectDocumentFileOperations
              operations={suggestions.file_operations}
              canApply={canApplyFiles}
              applying={applyingFiles}
              automationMode={automationMode}
              onApply={onApplyFiles}
            />
          </>
        )}
      </div>

      <footer className={styles.suggestionsFooter}>
        <span>{automationMode === 'git_backed_full' ? '默认建立整理前/后两个仅文档 Git 提交。' : '应用虚拟分区不会改写 Markdown 正文。'}</span>
        <button type="button" disabled={(automationMode === 'git_backed_full' && !trackingAvailable) || automationMode === 'suggestions_only' || !ready || !canEdit || applying || suggestionCount === 0} onClick={onApply}>
          {applying ? '应用中…' : automationMode === 'review_all' ? '审核并应用分区建议' : '应用分区建议'}
        </button>
      </footer>
    </main>
  )
}

function formatTraceTime(value: number) {
  if (!value) return ''
  return new Date(value * 1_000).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })
}

function shortRevision(value: string) {
  return value.length > 12 ? `${value.slice(0, 12)}…` : value
}

function profileLabel(profile: string) {
  const labels: Record<string, string> = {
    'software-platform': '软件平台',
    'software-api': 'API / SDK',
    product: '产品与业务',
    research: '研究项目',
    operations: '运维项目',
    'personal-knowledge': '个人知识库',
    auto: '自动判断',
  }
  return labels[profile] ?? profile
}
