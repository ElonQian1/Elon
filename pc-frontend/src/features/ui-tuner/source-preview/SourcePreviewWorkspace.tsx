import { useEffect, useRef, useState } from 'react'
import pageStyles from '../UiTunerPage.module.css'
import {
  findSourceSelection,
  sourceSelectionHint,
  type UiWorkspaceSelectionHint,
} from '../workspace/uiWorkspaceSelection'
import { SourceUiDesignProgress } from '../workspace/UiDesignProgressBar'
import { SourceDrivenPreviewSurface } from './SourceDrivenPreviewSurface'
import { PwaStyleInspector } from './PwaStyleInspector'
import { SourcePreviewInspector } from './SourcePreviewInspector'
import { SourcePreviewModeBar } from './SourcePreviewModeBar'
import { SourcePreviewTreePanel } from './SourcePreviewTreePanel'
import { useSourcePreviewSession } from './useSourcePreviewSession'
import { usePwaDesignSession } from './usePwaDesignSession'
import type { SourcePreviewMode } from './types'

interface Props {
  initialProjectRoot: string
  projectId: string
  active?: boolean
  onModeChange: (mode: SourcePreviewMode) => void
  selectionHint?: UiWorkspaceSelectionHint | null
  onSelectionHintChange?: (hint: UiWorkspaceSelectionHint) => void
}

export function SourcePreviewWorkspace({ initialProjectRoot, projectId, active = true, onModeChange, selectionHint, onSelectionHintChange }: Props) {
  const rememberedRoot = window.localStorage.getItem('elon.uiTuner.sourceProjectRoot') ?? ''
  const session = useSourcePreviewSession(initialProjectRoot || rememberedRoot)
  const [zoom, setZoom] = useState(.82)
  const pwaDesign = usePwaDesignSession({
    projectId,
    workspaceIdentity: session.projectRoot,
    sourceRevision: session.document?.sourceRevision ?? '',
    root: session.document?.root ?? null,
    onSelect: session.setSelectedKey,
  })
  const pwaPreviewActive = Boolean(
    session.document
    && !session.renderer.render
    && session.renderer.capabilities?.pwaPreview.available
    && session.renderer.capabilities.pwaPreview.url,
  )
  const autoLoaded = useRef(false)
  useEffect(() => {
    if (!autoLoaded.current && session.projectRoot) {
      autoLoaded.current = true
      void session.load()
    }
  }, [session.load, session.projectRoot])
  useEffect(() => {
    const match = findSourceSelection(session.document?.root ?? null, selectionHint ?? null)
    if (match && match !== session.selectedKey) session.setSelectedKey(match)
  }, [selectionHint, session.document?.root, session.selectedKey, session.setSelectedKey])
  useEffect(() => {
    if (session.selected) onSelectionHintChange?.(sourceSelectionHint(session.selected))
  }, [onSelectionHintChange, session.selected])
  return (
    <div className={pageStyles.page} style={{ display: active ? 'grid' : 'none' }}>
      <SourcePreviewTreePanel root={session.document?.root ?? null} selectedKey={session.selectedKey} onSelect={session.setSelectedKey} />
      <section className={pageStyles.stage}>
        <SourcePreviewModeBar
          mode="source" projectRoot={session.projectRoot} document={session.document} loading={session.loading} saveState={session.saveState} zoom={zoom}
          canUndo={session.history.past.length > 0} canRedo={session.history.future.length > 0} hasPending={Object.keys(session.pending).length > 0}
          onModeChange={onModeChange} onProjectRootChange={session.setProjectRoot} onLoad={(layout) => { void session.load(layout) }} onSave={() => { void session.commit() }}
          onZoom={setZoom} onUndo={session.undo} onRedo={session.redo}
          renderer={session.renderer}
        />
        <SourceUiDesignProgress hasDocument={Boolean(session.document)} pendingCount={Object.keys(session.pending).length} saveState={session.saveState} />
        <SourceDrivenPreviewSurface document={session.document} androidRender={session.renderer.render} pwaPreview={session.renderer.capabilities?.pwaPreview ?? null} selectedKey={session.selectedKey} zoom={zoom} loading={session.loading || session.renderer.rendering} error={session.error || session.renderer.error} onSelect={session.setSelectedKey} onModeChange={onModeChange} pwaDesign={pwaDesign} />
      </section>
      {pwaPreviewActive
        ? <PwaStyleInspector session={pwaDesign} />
        : <SourcePreviewInspector node={session.selected} pendingCount={Object.keys(session.pending).length} saveState={session.saveState} onChange={(patch) => { session.renderer.beginLocalDraft(); session.apply(patch) }} onSave={() => { void session.commit() }} onModeChange={onModeChange} />}
    </div>
  )
}
