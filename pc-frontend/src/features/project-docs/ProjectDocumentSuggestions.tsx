import { CheckCircle2, FileWarning, Lightbulb, RefreshCw, Sparkles } from 'lucide-react'

import { formatNumber } from './projectDocumentModel'
import type { DocumentOrganizationSuggestions } from './projectDocumentSections'
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
  onRefresh: () => void
  onApply: () => void
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
  onRefresh,
  onApply,
}: Props) {
  const ready = suggestions?.status === 'ready'
  const suggestionCount = (suggestions?.proposed_sections.length ?? 0) + (suggestions?.assignments.length ?? 0)
  return (
    <main className={styles.suggestionsPane}>
      <header className={styles.suggestionsHeader}>
        <div>
          <span><Sparkles size={18} aria-hidden="true" /></span>
          <div>
            <strong>AI 整理建议</strong>
            <small>建议与实际目录分开，审核后才应用</small>
          </div>
        </div>
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
            {(trace.catalog_revision || trace.suggestions_revision || trace.manifest_revision) && (
              <footer className={styles.traceRevisions}>
                {trace.catalog_revision && <span>catalog <code>{shortRevision(trace.catalog_revision)}</code></span>}
                {trace.suggestions_revision && <span>suggestions <code>{shortRevision(trace.suggestions_revision)}</code></span>}
                {trace.manifest_revision && <span>manifest <code>{shortRevision(trace.manifest_revision)}</code></span>}
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
            <span>点击“让当前 AI 生成整理建议”。AI 只写入结构化建议文件，不会自动移动项目文档。</span>
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
              <h3>建议新分区 <em>{suggestions.proposed_sections.length}</em></h3>
              <div className={styles.proposedSections}>
                {suggestions.proposed_sections.map((section) => (
                  <article key={section.id}>
                    <i style={{ background: section.color }} />
                    <strong>{section.label}</strong>
                    <span>{section.detail}</span>
                    <code>custom:{section.id}</code>
                  </article>
                ))}
                {!suggestions.proposed_sections.length && <p>不需要新增分区。</p>}
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
                {!suggestions.assignments.length && <p>没有文档需要调整虚拟分区。</p>}
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
          </>
        )}
      </div>

      <footer className={styles.suggestionsFooter}>
        <span>应用只更新 `.elon/document-sections.json`，不移动或改写 Markdown。</span>
        <button type="button" disabled={!ready || !canEdit || applying || suggestionCount === 0} onClick={onApply}>
          {applying ? '应用中…' : '审核并应用分区建议'}
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
