import { MousePointer2, RefreshCw, Smartphone } from 'lucide-react'
import type { SourcePreviewDocument } from './types'
import type { PwaDesignSession } from './usePwaDesignSession'
import styles from './SourcePreview.module.css'

interface Props {
  url: string
  document: SourcePreviewDocument
  zoom: number
  design: PwaDesignSession
}

function pwaUrl(value: string, reloadKey: number): string {
  const url = new URL(value, window.location.origin)
  if (url.origin !== window.location.origin) return '/web?ui_tuner_preview=1'
  url.searchParams.set('ui_tuner_preview', '1')
  if (reloadKey) url.searchParams.set('ui_tuner_reload', String(reloadKey))
  return `${url.pathname}${url.search}${url.hash}`
}

function modeTitle(design: PwaDesignSession): string {
  if (!design.ready) return '正在连接真实 PWA 页面'
  if (design.mode === 'select') return '选择一个组件'
  if (design.selection) return '正在编辑选中组件'
  return '正常使用页面'
}

function modeDetail(design: PwaDesignSession): string {
  if (!design.ready) return '连接完成后可先像手机一样正常操作；到达目标页后再切换为选择组件。'
  if (design.mode === 'select') return '下一次点击只用于选中组件，选中后会自动回到正常操作，不会长期拦截页面。'
  if (design.selection) return '右侧已经显示尺寸、间距、圆角、字体、颜色等草稿属性；修改会先作用在 PWA 真实页面。'
  return '先登录和点击到目标页面；需要修改时点“选择一个组件”。'
}

function selectedLabel(design: PwaDesignSession): string {
  const identity = design.selection?.identity
  if (!identity) return ''
  return identity.ariaLabel || identity.text || identity.id || identity.stableId || identity.tag || '已选中组件'
}

function selectedConfidenceLabel(design: PwaDesignSession): string {
  const confidence = design.selection?.identity.confidence
  if (confidence === 'high') return '稳定映射'
  if (confidence === 'medium') return '候选映射'
  return 'DOM 路径'
}

export function PwaInteractivePreviewSurface({ url, document, zoom, design }: Props) {
  const viewportWidth = Math.max(320, Math.min(430, Math.round(document.canvas.width / 3)))
  const viewportHeight = Math.max(640, Math.min(932, Math.round(document.canvas.height / 3)))
  const scale = Math.max(.55, Math.min(1.5, zoom))
  const route = design.route

  return (
    <div className={styles.pwaPreviewWorkspace} data-testid="pwa-interactive-preview">
      <div className={styles.pwaWorkflowGuide} aria-label="PWA 手工设计步骤">
        <span className={design.mode === 'interact' ? styles.activeWorkflowStep : ''}>① 真实使用并到达页面</span>
        <span className={design.mode === 'select' && !design.selection ? styles.activeWorkflowStep : ''}>② 点一次选择组件</span>
        <span className={design.mode === 'interact' && design.selection ? styles.activeWorkflowStep : ''}>③ 右侧修改样式</span>
        <span>④ 草稿自动保存</span>
      </div>
      <div className={styles.pwaPreviewToolbar}>
        <span className={design.ready ? styles.pwaReady : styles.pwaConnecting}><i />{design.ready ? '真实 PWA 已连接' : '正在连接真实 PWA…'}</span>
        <div className={styles.pwaModeSwitch}>
          <button className={design.mode === 'interact' ? styles.activePwaMode : ''} type="button" disabled={!design.ready} onClick={() => design.setMode('interact')}><Smartphone size={14} />正常操作页面</button>
          <button className={design.mode === 'select' ? styles.activePwaMode : ''} type="button" disabled={!design.ready} onClick={() => design.setMode('select')}><MousePointer2 size={14} />选择一个组件</button>
        </div>
        <button type="button" title="重新载入 PWA（已保存草稿会自动恢复）" onClick={design.prepareReload}><RefreshCw size={14} /></button>
      </div>
      <div className={styles.pwaModeGuide} data-mode={design.mode} data-ready={design.ready ? 'true' : 'false'}>
        <strong>{modeTitle(design)}</strong>
        <span>{modeDetail(design)}</span>
        {design.mode === 'interact' && !design.selection && (
          <button type="button" disabled={!design.ready} onClick={() => design.setMode('select')}>
            <MousePointer2 size={14} /> 开始设计/修改页面
          </button>
        )}
        {design.mode === 'select' && (
          <button type="button" onClick={() => design.setMode('interact')}>
            <Smartphone size={14} /> 取消选择，继续操作页面
          </button>
        )}
      </div>
      {route && <div className={styles.pwaRouteStatus}>
        当前画面：{route.screenTitle || '未识别画面'}
        {route.screenKey && <> · <code>{route.screenKey}</code></>}
        {' · '}{route.path}{route.search}{route.hash} · {route.viewport.width}×{route.viewport.height}
      </div>}
      {design.selection && <div className={styles.pwaSelectedCanvasSummary} data-confidence={design.selection.identity.confidence}>
        <div>
          <strong>已选中：{selectedLabel(design)}</strong>
          <span>{selectedConfidenceLabel(design)} · 右侧可直接改尺寸、间距、圆角、字体和颜色</span>
          <code>{design.selection.identity.selector}</code>
        </div>
        <button type="button" onClick={() => design.setMode('select')}><MousePointer2 size={14} />继续选组件</button>
        <button type="button" onClick={() => design.setMode('interact')}><Smartphone size={14} />操作页面</button>
      </div>}
      {design.mode === 'select' && <div className={styles.pwaBindingNotice}>选择模式只拦截下一次点击；选中后会自动回到正常操作页面。</div>}
      {design.mode !== 'select' && design.unboundLabel && <div className={styles.pwaBindingNotice}>已选中“{design.unboundLabel}”；当前用可解释 DOM 路径保存，尚未假定它已绑定 Android 源码。</div>}
      <div className={styles.pwaDraftBadge}>真实 PWA 页面 · 手工草稿</div>
      <div className={styles.pwaDeviceViewport} style={{ width: viewportWidth * scale, height: viewportHeight * scale }}>
        <iframe key={design.reloadKey} ref={design.iframeRef} className={styles.pwaDeviceFrame} src={pwaUrl(url, design.reloadKey)} title="移动 PWA 真实页面" style={{ width: viewportWidth, height: viewportHeight, transform: `scale(${scale})` }} />
      </div>
    </div>
  )
}
