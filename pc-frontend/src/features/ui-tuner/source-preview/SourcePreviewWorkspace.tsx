import { Code2, MonitorSmartphone } from 'lucide-react'
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
  return <div className={styles.modeBar}><div className={styles.modeTabs}><button className={styles.activeTab} onClick={() => onModeChange('evidence')}><MonitorSmartphone size={15} />Android 真实渲染</button><button onClick={() => onModeChange('source')}><Code2 size={15} />源码近似预览</button></div><span>画面来自真实 Android Renderer；选择框只负责点选，不会覆盖或模拟组件外观</span></div>
}
