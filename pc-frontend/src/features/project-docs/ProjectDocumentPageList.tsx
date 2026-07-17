import { Check, Ellipsis, FileText } from 'lucide-react'
import { useState } from 'react'

import type { ProjectDocumentEntry } from './projectDocumentModel'
import { formatNumber, roleLabel } from './projectDocumentModel'
import type { DocumentNavigationMode } from './projectDocumentArchitecture'
import ProjectDocumentBatchBar from './ProjectDocumentBatchBar'
import type { ProjectDocumentMenuTarget } from './ProjectDocumentCommandMenu'
import type { DocumentSection, DocumentSectionManifest } from './projectDocumentSections'
import { menuPointForButton, useProjectDocumentMenuTrigger, type ProjectDocumentMenuPoint } from './useProjectDocumentMenuTrigger'
import styles from './ProjectDocumentsWorkspace.module.css'

interface Props {
  documents: ProjectDocumentEntry[]
  manifest: DocumentSectionManifest
  navigationMode: DocumentNavigationMode
  assignmentSections: DocumentSection[]
  selectedPath: string
  selectedPaths: Set<string>
  loading: boolean
  error: string
  commandBusy: boolean
  canEdit: boolean
  onChoose: (path: string) => void
  onToggleSelection: (path: string) => void
  onOpenMenu: (target: ProjectDocumentMenuTarget, point: ProjectDocumentMenuPoint) => void
  onBatchAssign: (sectionKey: string) => void
  onBatchAction: (action: 'pin' | 'recommend' | 'automatic') => void
  onBatchAi: (paths: string[]) => void
  onClearSelection: () => void
  onMoveBefore: (path: string, beforePath: string) => void
}

export default function ProjectDocumentPageList({
  documents, manifest, navigationMode, assignmentSections, selectedPath, selectedPaths,
  loading, error, commandBusy, canEdit, onChoose, onToggleSelection, onOpenMenu, onBatchAssign,
  onBatchAction, onBatchAi, onClearSelection, onMoveBefore,
}: Props) {
  const trigger = useProjectDocumentMenuTrigger(onOpenMenu)
  const [dragging, setDragging] = useState('')
  return (
    <div className={styles.pageList} {...trigger({ kind: 'page-list' })}>
      <ProjectDocumentBatchBar
        count={selectedPaths.size}
        navigationMode={navigationMode}
        sections={assignmentSections}
        busy={commandBusy}
        onAssign={onBatchAssign}
        onPin={() => onBatchAction('pin')}
        onRecommend={() => onBatchAction('recommend')}
        onRestoreAutomatic={() => onBatchAction('automatic')}
        onAskAi={() => onBatchAi([...selectedPaths])}
        onClear={onClearSelection}
      />
      {error && <div className={styles.errorBox}>{error}</div>}
      {!loading && !error && documents.length === 0 && <div className={styles.emptyList}>这个分区还没有文档</div>}
      {documents.map((entry) => {
        const normalizedPath = normalizeDocumentPath(entry.path)
        const selected = selectedPaths.has(entry.path)
        const pinned = manifest.document_metadata[normalizedPath]?.pinned === true
        const recommended = manifest.home.start_here.some((path) => normalizeDocumentPath(path) === normalizedPath)
        const target: ProjectDocumentMenuTarget = { kind: 'document', document: entry, selected, pinned, recommended }
        const canDrop = canEdit && !!dragging && dragging !== entry.path
        return (
          <div
            className={styles.pageRow}
            key={entry.path}
            data-selected={selected || undefined}
            data-dragging={dragging === entry.path || undefined}
            data-drop-target={canDrop || undefined}
            draggable={canEdit}
            onDragStart={() => setDragging(entry.path)}
            onDragEnd={() => setDragging('')}
            onDragOver={(event) => { if (canDrop) event.preventDefault() }}
            onDrop={(event) => {
              event.preventDefault()
              if (canDrop) onMoveBefore(dragging, entry.path)
              setDragging('')
            }}
            {...trigger(target)}
          >
            <button
              className={styles.pageSelectButton}
              type="button"
              aria-label={`${selected ? '取消选择' : '选择'} ${entry.title}`}
              aria-pressed={selected}
              onClick={(event) => { event.stopPropagation(); onToggleSelection(entry.path) }}
            >
              {selected && <Check size={12} />}
            </button>
            <button
              className={[styles.pageButton, selectedPath === entry.path ? styles.pageActive : ''].join(' ')}
              type="button"
              onClick={() => onChoose(entry.path)}
            >
              <span className={styles.pageTitle}><FileText size={14} aria-hidden="true" />{entry.title}{pinned && <b>固定</b>}</span>
              <span className={styles.pagePath}>{entry.path}</span>
              <span className={styles.pageMeta}>
                <em>{roleLabel(entry.metadata.role)}</em>
                <small>{formatNumber(entry.metadata.token_estimate)} token</small>
                {entry.source === 'platform_default' && <b>平台模板</b>}
                {entry.metadata.default_retrieval && <b>AI 必读</b>}
              </span>
            </button>
            <button
              className={styles.pageMoreButton}
              type="button"
              title={`更多操作：${entry.title}`}
              onClick={(event) => { event.stopPropagation(); onOpenMenu(target, menuPointForButton(event.currentTarget)) }}
            >
              <Ellipsis size={14} />
            </button>
          </div>
        )
      })}
    </div>
  )
}

function normalizeDocumentPath(path: string) {
  return path.trim().replace(/\\/g, '/')
}
