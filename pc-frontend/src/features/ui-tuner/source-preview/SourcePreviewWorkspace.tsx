import { Code2, Image } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import pageStyles from '../UiTunerPage.module.css'
import { SourceDrivenPreviewSurface } from './SourceDrivenPreviewSurface'
import { SourcePreviewInspector } from './SourcePreviewInspector'
import { SourcePreviewModeBar } from './SourcePreviewModeBar'
import { SourcePreviewTreePanel } from './SourcePreviewTreePanel'
import { useSourcePreviewSession } from './useSourcePreviewSession'
import type { SourcePreviewMode } from './types'
import styles from './SourcePreview.module.css'

interface Props { initialProjectRoot: string; active?: boolean; onModeChange: (mode: SourcePreviewMode) => void }

export function SourcePreviewWorkspace({ initialProjectRoot, active = true, onModeChange }: Props) {
  const rememberedRoot = window.localStorage.getItem('elon.uiTuner.sourceProjectRoot') ?? ''
  const session = useSourcePreviewSession(initialProjectRoot || rememberedRoot)
  const [zoom, setZoom] = useState(.82)
  const autoLoaded = useRef(false)
  useEffect(() => {
    if (!autoLoaded.current && session.projectRoot) {
      autoLoaded.current = true
      void session.load()
    }
  }, [session.load, session.projectRoot])
  return (
    <div className={pageStyles.page} style={{ display: active ? 'grid' : 'none' }}>
      <SourcePreviewTreePanel root={session.document?.root ?? null} selectedKey={session.selectedKey} onSelect={session.setSelectedKey} />
      <section className={pageStyles.stage}>
        <SourcePreviewModeBar
          mode="source" projectRoot={session.projectRoot} document={session.document} loading={session.loading} saveState={session.saveState} zoom={zoom}
          canUndo={session.history.past.length > 0} canRedo={session.history.future.length > 0} hasPending={Object.keys(session.pending).length > 0}
          onModeChange={onModeChange} onProjectRootChange={session.setProjectRoot} onLoad={(layout) => { void session.load(layout) }} onSave={() => { void session.commit() }}
          onZoom={setZoom} onUndo={session.undo} onRedo={session.redo}
        />
        <SourceDrivenPreviewSurface document={session.document} selectedKey={session.selectedKey} zoom={zoom} loading={session.loading} error={session.error} onSelect={session.setSelectedKey} />
      </section>
      <SourcePreviewInspector node={session.selected} pendingCount={Object.keys(session.pending).length} saveState={session.saveState} onChange={session.apply} onSave={() => { void session.commit() }} />
    </div>
  )
}

export function EvidenceModeSwitch({ onModeChange }: { initialProjectRoot: string; onModeChange: (mode: SourcePreviewMode) => void }) {
  return <div className={styles.modeBar}><div className={styles.modeTabs}><button onClick={() => onModeChange('source')}><Code2 size={15} />动态设计</button><button className={styles.activeTab} onClick={() => onModeChange('evidence')}><Image size={15} />真机证据</button></div><span>截图 / XML 仅用于运行证据、源码定位和构建后验收</span></div>
}
