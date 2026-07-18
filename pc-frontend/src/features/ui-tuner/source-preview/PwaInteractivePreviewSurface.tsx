import { MousePointer2, RefreshCw, Smartphone } from 'lucide-react'
import { useState } from 'react'
import type { SourcePreviewDocument } from './types'
import type { PwaDesignSession } from './usePwaDesignSession'
import styles from './SourcePreview.module.css'

interface Props {
  url: string
  document: SourcePreviewDocument
  zoom: number
  design: PwaDesignSession
}

function pwaUrl(value: string): string {
  const url = new URL(value, window.location.origin)
  if (url.origin !== window.location.origin) return '/web?ui_tuner_preview=1'
  url.searchParams.set('ui_tuner_preview', '1')
  return `${url.pathname}${url.search}${url.hash}`
}

export function PwaInteractivePreviewSurface({ url, document, zoom, design }: Props) {
  const [reloadKey, setReloadKey] = useState(0)
  const viewportWidth = Math.max(320, Math.min(430, Math.round(document.canvas.width / 3)))
  const viewportHeight = Math.max(640, Math.min(932, Math.round(document.canvas.height / 3)))
  const scale = Math.max(.55, Math.min(1.5, zoom))
  const route = design.route

  return (
    <div className={styles.pwaPreviewWorkspace} data-testid="pwa-interactive-preview">
      <div className={styles.pwaWorkflowGuide} aria-label="PWA 手工设计步骤">
        <span className={design.mode === 'interact' ? styles.activeWorkflowStep : ''}>① 真实使用并到达页面</span>
        <span className={design.mode === 'select' && !design.selection ? styles.activeWorkflowStep : ''}>② 开始设计并选元素</span>
        <span className={design.mode === 'select' && design.selection ? styles.activeWorkflowStep : ''}>③ 修改样式</span>
        <span>④ 草稿自动保存</span>
      </div>
      <div className={styles.pwaPreviewToolbar}>
        <span className={design.ready ? styles.pwaReady : styles.pwaConnecting}><i />{design.ready ? '真实 PWA 已连接' : '正在连接真实 PWA…'}</span>
        <div className={styles.pwaModeSwitch}>
          <button className={design.mode === 'interact' ? styles.activePwaMode : ''} type="button" disabled={!design.ready} onClick={() => design.setMode('interact')}><Smartphone size={14} />{design.mode === 'select' ? '退出设计并继续使用' : '正常使用'}</button>
          <button className={design.mode === 'select' ? styles.activePwaMode : ''} type="button" disabled={!design.ready} onClick={() => design.setMode('select')}><MousePointer2 size={14} />开始设计/修改页面</button>
        </div>
        <button type="button" title="重新载入 PWA（已保存草稿会自动恢复）" onClick={() => { design.prepareReload(); setReloadKey((value) => value + 1) }}><RefreshCw size={14} /></button>
      </div>
      {route && <div className={styles.pwaRouteStatus}>当前真实页面：<code>{route.path}{route.search}{route.hash}</code> · {route.viewport.width}×{route.viewport.height}</div>}
      {design.mode === 'select' && design.unboundLabel && <div className={styles.pwaBindingNotice}>已选中“{design.unboundLabel}”；当前用可解释 DOM 路径保存，尚未假定它已绑定 Android 源码。</div>}
      <div className={styles.pwaDeviceViewport} style={{ width: viewportWidth * scale, height: viewportHeight * scale }}>
        <div className={styles.pwaDraftBadge}>真实 PWA 页面 · 手工草稿</div>
        <iframe key={reloadKey} ref={design.iframeRef} className={styles.pwaDeviceFrame} src={pwaUrl(url)} title="移动 PWA 真实页面" style={{ width: viewportWidth, height: viewportHeight, transform: `scale(${scale})` }} />
      </div>
    </div>
  )
}
