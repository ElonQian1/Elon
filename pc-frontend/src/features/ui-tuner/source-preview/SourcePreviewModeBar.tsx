import { Code2, Image, LoaderCircle, Redo2, Save, Undo2, ZoomIn, ZoomOut } from 'lucide-react'
import type { SourcePreviewDocument, SourcePreviewMode, SourcePreviewSaveState } from './types'
import styles from './SourcePreview.module.css'

interface Props {
  mode: SourcePreviewMode; projectRoot: string; document: SourcePreviewDocument | null; loading: boolean; saveState: SourcePreviewSaveState; zoom: number; canUndo: boolean; canRedo: boolean; hasPending: boolean
  onModeChange: (mode: SourcePreviewMode) => void; onProjectRootChange: (value: string) => void; onLoad: (layout?: string) => void; onSave: () => void; onZoom: (value: number) => void; onUndo: () => void; onRedo: () => void
}

export function SourcePreviewModeBar(props: Props) {
  const savedLabel = props.saveState === 'saved' ? '源码已保存' : props.saveState === 'saving' ? '正在写回…' : props.saveState === 'error' ? '保存失败' : '动态预览'
  return (
    <div className={styles.modeBar}>
      <div className={styles.modeTabs}>
        <button className={props.mode === 'source' ? styles.activeTab : ''} onClick={() => props.onModeChange('source')}><Code2 size={15} />动态设计</button>
        <button className={props.mode === 'evidence' ? styles.activeTab : ''} onClick={() => props.onModeChange('evidence')}><Image size={15} />真机证据</button>
      </div>
      {props.mode === 'source' && <>
        <input className={styles.projectInput} value={props.projectRoot} onChange={(event) => props.onProjectRootChange(event.target.value)} placeholder="本机 Android 项目目录" />
        {props.document && <select value={props.document.selectedLayout} onChange={(event) => props.onLoad(event.target.value)}>{props.document.layoutFiles.map((file) => { const parts = file.split('/'); return <option key={file} value={file}>{parts[parts.length - 1]}</option> })}</select>}
        <button onClick={() => props.onLoad()} disabled={props.loading}>{props.loading ? <LoaderCircle className={styles.spin} size={15} /> : <Code2 size={15} />}加载源码</button>
        <span className={`${styles.saveState} ${styles[`state_${props.saveState}`]}`}>{savedLabel}</span>
        <button aria-label="撤销动态修改" onClick={props.onUndo} disabled={!props.canUndo}><Undo2 size={15} /></button><button aria-label="重做动态修改" onClick={props.onRedo} disabled={!props.canRedo}><Redo2 size={15} /></button>
        <button aria-label="缩小动态画布" onClick={() => props.onZoom(Math.max(.35, props.zoom - .1))}><ZoomOut size={15} /></button><span>{Math.round(props.zoom * 100)}%</span><button aria-label="放大动态画布" onClick={() => props.onZoom(Math.min(1.5, props.zoom + .1))}><ZoomIn size={15} /></button>
        <button className={styles.primaryButton} onClick={props.onSave} disabled={!props.document || !props.hasPending || props.saveState === 'saving'}><Save size={15} />写回源码</button>
      </>}
    </div>
  )
}
