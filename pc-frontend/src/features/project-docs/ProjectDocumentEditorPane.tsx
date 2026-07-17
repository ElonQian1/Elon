import { Bot, FolderTree, Save } from 'lucide-react'

import MarkdownContent from '../markdown/MarkdownContent'
import ProjectDocumentAccessNotice from './ProjectDocumentAccessNotice'
import { lifecycleLabel, roleLabel, type DocumentCatalog, type DocumentFile, type ProjectDocumentEntry } from './projectDocumentModel'
import type { DocumentSection } from './projectDocumentSections'
import styles from './ProjectDocumentsWorkspace.module.css'

export type ProjectDocumentViewMode = 'edit' | 'preview' | 'split'
export const AUTOMATIC_DOCUMENT_SECTION = '__automatic__'

interface Props {
  catalog: DocumentCatalog | null
  document: DocumentFile | null
  selectedEntry?: ProjectDocumentEntry
  selectedPath: string
  selectedAssignment: string
  automaticSectionLabel: string
  assignmentSections: DocumentSection[]
  viewMode: ProjectDocumentViewMode
  draft: string
  dirty: boolean
  loading: boolean
  error: string
  message: string
  saveState: 'idle' | 'saving' | 'saved' | 'error'
  onViewModeChange: (mode: ProjectDocumentViewMode) => void
  onSave: () => void
  onAssignmentChange: (sectionKey: string) => void
  onDraftChange: (content: string) => void
  onRetryCatalog: () => void
  onRetryDocument: () => void
}

export default function ProjectDocumentEditorPane({
  catalog, document, selectedEntry, selectedPath, selectedAssignment, automaticSectionLabel,
  assignmentSections, viewMode, draft, dirty, loading, error, message, saveState,
  onViewModeChange, onSave, onAssignmentChange, onDraftChange, onRetryCatalog, onRetryDocument,
}: Props) {
  return (
    <main className={styles.documentPane}>
      <header className={styles.documentHeader}>
        <div className={styles.documentIdentity}>
          <FolderTree size={18} aria-hidden="true" />
          <span><strong>{selectedEntry?.title ?? '选择一篇文档'}</strong><small>{selectedEntry?.path ?? catalog?.workspace ?? ''}</small></span>
        </div>
        <div className={styles.viewModes}>
          {(['edit', 'preview', 'split'] as ProjectDocumentViewMode[]).map((mode) => (
            <button className={viewMode === mode ? styles.modeActive : ''} key={mode} type="button" onClick={() => onViewModeChange(mode)}>
              {mode === 'edit' ? '编辑' : mode === 'preview' ? '阅读' : '分栏'}
            </button>
          ))}
        </div>
        <button className={styles.saveButton} type="button" onClick={onSave} disabled={!dirty || !document?.can_edit || saveState === 'saving'}>
          <Save size={15} aria-hidden="true" />
          {saveState === 'saving' ? '保存中' : saveState === 'saved' ? '已保存' : '保存'}
        </button>
      </header>

      <ProjectDocumentAccessNotice
        access={catalog?.access}
        warnings={[...(catalog?.warnings ?? []), ...(document?.warnings ?? [])]}
        onRetry={onRetryCatalog}
      />

      {selectedEntry && (
        <div className={styles.authorityStrip}>
          <span>{roleLabel(selectedEntry.metadata.role)}</span>
          <span>{lifecycleLabel(selectedEntry.metadata.lifecycle)}</span>
          <span>{selectedEntry.metadata.authority || 'unknown'}</span>
          <select aria-label="文档分区" value={selectedAssignment} disabled={!catalog?.can_edit} onChange={(event) => onAssignmentChange(event.target.value)}>
            <option value={AUTOMATIC_DOCUMENT_SECTION}>自动：{automaticSectionLabel}</option>
            {assignmentSections.map((section) => <option key={section.key} value={section.key}>{section.label}</option>)}
          </select>
          <small>{selectedEntry.metadata.reason}</small>
        </div>
      )}
      {message && <div className={styles.messageBar}>{message}</div>}

      <div className={[styles.editorBody, styles[`view_${viewMode}`]].join(' ')}>
        {loading ? <div className={styles.documentEmpty}>正在按需读取这一篇文档…</div> : document ? (
          <>
            {viewMode !== 'preview' && <textarea className={styles.editor} value={draft} onChange={(event) => onDraftChange(event.target.value)} readOnly={!document.can_edit} spellCheck={false} aria-label="Markdown 编辑器" />}
            {viewMode !== 'edit' && <article className={styles.preview}><MarkdownContent content={draft || '（文档为空）'} /></article>}
          </>
        ) : error ? (
          <ProjectDocumentAccessNotice error={error} path={selectedPath} onRetry={onRetryDocument} />
        ) : (
          <div className={styles.documentEmpty}>
            <Bot size={30} aria-hidden="true" /><strong>从左侧选择一篇文档</strong>
            <span>程序只会在你打开时读取正文，目录扫描不会把全部 Markdown 送给 AI。</span>
          </div>
        )}
      </div>
    </main>
  )
}
