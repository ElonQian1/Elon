import { MousePointer2, RefreshCw, Smartphone } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { PwaDeviceToolbar } from './PwaDeviceToolbar'
import type { SourcePreviewDocument } from './types'
import type { PwaDesignSession } from './usePwaDesignSession'
import type { PwaStyleProperty } from './pwaDesignDraft'
import { readPwaDeviceViewport, savePwaDeviceViewport } from './pwaDeviceViewport'
import styles from './SourcePreview.module.css'

interface Props {
  url: string
  document: SourcePreviewDocument
  zoom: number
  onZoom: (zoom: number) => void
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
  if (design.mode === 'select') return '下一次点击只用于选中组件，选中后会自动回到正常操作；也可以按 Esc 退出，15 秒不操作会自动恢复。'
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

function currentStyleValue(design: PwaDesignSession, property: PwaStyleProperty): string {
  const selection = design.selection
  if (!selection) return ''
  const draftElement = Object.values(design.draft?.elements ?? {}).find((element) => (
    element.identity.key === selection.identity.key || element.identity.selector === selection.identity.selector
  ))
  return draftElement?.styleDiff[property]
    ?? selection.originalStyle.authored[property]
    ?? selection.originalStyle.computed[property]
    ?? ''
}

function formatNumber(value: number): string {
  return Number.isInteger(value) ? String(value) : value.toFixed(2).replace(/\.?0+$/, '')
}

function adjustCssNumber(value: string, delta: number, fallback: number, unit = 'px', min = 0): string {
  const match = value.trim().match(/^(-?\d+(?:\.\d+)?)([a-z%]*)$/i)
  const next = match ? Number(match[1]) + delta : fallback
  return `${formatNumber(Math.max(min, next))}${match?.[2] || unit}`
}

export function PwaInteractivePreviewSurface({ url, document, zoom, onZoom, design }: Props) {
  const [viewport, setViewport] = useState(readPwaDeviceViewport)
  const workspaceRef = useRef<HTMLDivElement>(null)
  const viewportWidth = viewport.width
  const viewportHeight = viewport.height
  const scale = Math.max(.35, Math.min(1.5, zoom))
  const route = design.route
  useEffect(() => savePwaDeviceViewport(viewport), [viewport])

  const fitViewport = () => {
    const availableWidth = Math.max(280, (workspaceRef.current?.clientWidth ?? window.innerWidth) - 72)
    const availableHeight = Math.max(360, window.innerHeight - 410)
    const next = Math.max(.35, Math.min(1.5, availableWidth / viewportWidth, availableHeight / viewportHeight))
    onZoom(Math.round(next * 100) / 100)
  }

  return (
    <div ref={workspaceRef} className={styles.pwaPreviewWorkspace} data-testid="pwa-interactive-preview">
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
      <PwaDeviceToolbar
        viewport={viewport}
        runtimeViewport={route?.viewport}
        zoom={zoom}
        projectRoot={document.projectRoot}
        sourceRevision={document.sourceRevision}
        runtimeUrl={url}
        route={route}
        onViewportChange={setViewport}
        onZoom={onZoom}
        onFit={fitViewport}
      />
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
        <div className={styles.pwaCanvasQuickTune} aria-label="选中组件快速微调">
          <button type="button" onClick={() => design.updateStyles('canvas:compact', { paddingTop: '6px', paddingRight: '10px', paddingBottom: '6px', paddingLeft: '10px', fontSize: '13px' })}>紧凑</button>
          <button type="button" onClick={() => design.updateStyles('canvas:relaxed', { paddingTop: '14px', paddingRight: '18px', paddingBottom: '14px', paddingLeft: '18px', fontSize: '15px' })}>舒展</button>
          <button type="button" onClick={() => design.updateStyle('borderRadius', adjustCssNumber(currentStyleValue(design, 'borderRadius'), 2, 12))}>圆角 +</button>
          <button type="button" onClick={() => design.updateStyle('fontSize', adjustCssNumber(currentStyleValue(design, 'fontSize'), 1, 14, 'px', 8))}>字号 +</button>
        </div>
        <button type="button" onClick={() => design.setMode('select')}><MousePointer2 size={14} />继续选组件</button>
        <button type="button" onClick={() => design.setMode('interact')}><Smartphone size={14} />操作页面</button>
      </div>}
      {design.mode === 'select' && <div className={styles.pwaBindingNotice}>选择模式只拦截下一次点击；选中后自动恢复。按 Esc 或等待 15 秒也会退出，避免页面卡在不能操作。</div>}
      {design.mode !== 'select' && design.unboundLabel && <div className={styles.pwaBindingNotice}>已选中“{design.unboundLabel}”；当前用可解释 DOM 路径保存，尚未假定它已绑定 Android 源码。</div>}
      <div className={styles.pwaDraftBadge}>真实 PWA 页面 · 手工草稿</div>
      <div className={styles.pwaDeviceViewport} style={{ width: viewportWidth * scale, height: viewportHeight * scale }}>
        <iframe key={design.reloadKey} ref={design.iframeRef} className={styles.pwaDeviceFrame} src={pwaUrl(url, design.reloadKey)} title="移动 PWA 真实页面" style={{ width: viewportWidth, height: viewportHeight, transform: `scale(${scale})` }} />
        {viewport.showSafeArea && <div
          className={styles.pwaSafeAreaGuide}
          aria-hidden="true"
          style={{
            top: viewport.safeArea.top * scale,
            right: viewport.safeArea.right * scale,
            bottom: viewport.safeArea.bottom * scale,
            left: viewport.safeArea.left * scale,
          }}
        />}
      </div>
    </div>
  )
}
